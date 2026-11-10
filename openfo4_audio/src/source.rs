use std::cmp;
use std::fmt::{self, Debug, Formatter};
use std::fs::File;
use std::io::Cursor;
use std::path::Path;
use std::sync::LazyLock;

use openfo4_tracing::trace_span;
use symphonia::core::audio::conv::FromSample;
use symphonia::core::audio::sample::Sample;
use symphonia::core::audio::{Audio, AudioBuffer, GenericAudioBufferRef};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::codecs::registry::CodecRegistry;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::{Hint, Probe};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Timestamp;
use thiserror::Error;

use crate::buffer::{Buf, BufMut, Sequential, SequentialView, SequentialViewMut};
use crate::effects::Volume;
use crate::resampler::{self, Resampler};
use crate::sound::Frame;

pub use symphonia::core::io::MediaSource;

static PROBE: LazyLock<Probe> = LazyLock::new(|| {
    let mut probe = Probe::default();
    symphonia::default::register_enabled_formats(&mut probe);

    #[cfg(feature = "xwma")]
    probe.register_format::<crate::wma::XwmReader<'_>>();
    probe.register_format::<crate::wma::FuzReader<'_>>();

    probe
});

static CODECS: LazyLock<CodecRegistry> = LazyLock::new(|| {
    let mut registry = CodecRegistry::default();
    symphonia::default::register_enabled_codecs(&mut registry);
    registry
});

#[derive(Debug)]
pub struct AudioSource {
    sample_rate: u32,
    buffer: FrameBuffer,
    resampler: Option<Resampler>,
    decode_buffer: FrameBuffer,
    source: Source,
    track_id: u32,
}

impl AudioSource {
    pub fn from_file<P>(path: P) -> Result<Self, MediaSourceError>
    where
        P: AsRef<Path>,
    {
        let file = File::open(path).map_err(MediaSourceError::new)?;
        Self::from_reader(file)
    }

    pub fn from_reader<R>(reader: R) -> Result<Self, MediaSourceError>
    where
        R: MediaSource + 'static,
    {
        let mss = MediaSourceStream::new(Box::new(reader), Default::default());

        let format_reader = PROBE
            .probe(
                &Hint::default(),
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(MediaSourceError::new)?;

        let track = format_reader
            .default_track(TrackType::Audio)
            .ok_or_else(|| MediaSourceError::msg("source has no default audio track"))?;
        let codec_params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or_else(|| MediaSourceError::msg("unable to determine audio codec parameters"))?;
        let sample_rate = codec_params.sample_rate.unwrap();

        let decoder = CODECS
            .make_audio_decoder(codec_params, &AudioDecoderOptions::default())
            .map_err(MediaSourceError::new)?;

        Ok(Self {
            track_id: track.id,
            sample_rate,
            buffer: FrameBuffer::new(),
            resampler: None,
            decode_buffer: FrameBuffer::new(),
            source: Source::Io(IoSource {
                decoder,
                format_reader,
            }),
        })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, MediaSourceError> {
        let cursor = Cursor::new(bytes);
        Self::from_reader(cursor)
    }

    pub fn from_data(data: SoundData) -> Self {
        let left = data.frames.iter().map(|f| f.left * data.volume.0);
        let right = data.frames.iter().map(|f| f.right * data.volume.0);

        let mut buf = FrameBuffer::new();
        buf.reserve(data.frames.len());

        let mut dst = buf.spare_capacity_mut();
        for (src, dst) in left.zip(dst.channel_mut(0).unwrap()) {
            *dst = src;
        }

        for (src, dst) in right.zip(dst.channel_mut(1).unwrap()) {
            *dst = src;
        }

        buf.increase_len(data.frames.len());

        Self {
            source: Source::Empty,
            decode_buffer: buf,
            sample_rate: data.sample_rate,
            buffer: FrameBuffer::new(),
            resampler: None,
            track_id: 0,
        }
    }

    /// Sets the sample rate that should be used by this `AudioSource`.
    pub(crate) fn set_sample_rate(&mut self, sample_rate: u32) {
        if sample_rate != self.sample_rate {
            self.resampler = Some(Resampler::new(self.sample_rate, sample_rate));
        }
    }

    /// Reads frames of this source into `buf`. Returns the number of frames written.
    ///
    /// When `looping` is `true` the sound will start wrap to the start and [`Eof`] is never returned.
    ///
    /// [`Eof`]: Error::Eof
    pub(crate) fn read(&mut self, mut buf: &mut [Frame], looping: bool) -> Result<usize, Error> {
        let _span = trace_span!("AudioSource::read").entered();

        // If we samples from the previous packet we flush
        // them first.
        // If they are enough to fill `buf` we don't need
        // to decode any packets.
        let mut frames_written = self.buffer.move_frames_into(buf);
        buf = &mut buf[frames_written..];
        if buf.is_empty() {
            return Ok(frames_written);
        }

        loop {
            match self.prepare_next_frame() {
                Ok(()) => (),
                Err(err) => {
                    if matches!(err, Error::Eof) && looping {
                        tracing::debug!("error reading from audio source: {}", err);
                        return Err(err);
                    }

                    // Read the next frame from the source start.
                    if looping {
                        self.source.reset(self.track_id)?;
                        continue;
                    }

                    // If we have already written some frames those
                    // need to be returned first.
                    // The next call to `read` will then yield EOF.
                    if frames_written != 0 {
                        return Ok(frames_written);
                    }

                    return Err(err);
                }
            }

            let count = self.buffer.move_frames_into(buf);
            buf = &mut buf[count..];
            frames_written += count;

            if buf.is_empty() {
                return Ok(frames_written);
            }
        }
    }

    /// Load the next frame into the current buffer.
    ///
    /// May load more than a single frame at once.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Eof`] if the `AudioSource` has no more frames that can be presented.
    fn prepare_next_frame(&mut self) -> Result<(), Error> {
        match &mut self.resampler {
            Some(resampler) => loop {
                let src = self.decode_buffer.initialized();
                let dst = self.buffer.spare_capacity_mut();

                match resampler.resample(src, dst) {
                    Ok(output) => {
                        self.decode_buffer.remove_frames(output.frames_read);
                        self.buffer.increase_len(output.frames_written);
                        break;
                    }
                    Err(resampler::Error::InputTooSmall(len_required)) => {
                        // This means we need to decode another packet.
                        while self.decode_buffer.len < len_required {
                            self.source.decode_packet(&mut self.decode_buffer)?;
                        }

                        // Try again now that we have a big enough
                        // input buffer.
                        continue;
                    }
                    Err(resampler::Error::OutputTooSmall(len)) => {
                        // Reserve additional space in the output buffer
                        // and try again.
                        self.buffer.reserve(len);
                        continue;
                    }
                }
            },
            None => {
                if self.decode_buffer.is_empty() {
                    self.source.decode_packet(&mut self.decode_buffer)?;
                }

                // If we don't need to resample we only
                // have to copy all decoded frames into
                // the "ready-to-output" buffer.
                let src = self.decode_buffer.initialized();

                self.buffer.reserve(src.num_frames());
                let mut dst = self.buffer.spare_capacity_mut();

                for channel_index in 0..src.num_channels() {
                    let count = src.num_frames();

                    let src = src.channel(channel_index).unwrap();
                    let dst = dst.channel_mut(channel_index).unwrap();
                    dst[..count].copy_from_slice(src);
                }

                let frames_written = src.num_frames();
                self.buffer.increase_len(frames_written);
                self.decode_buffer.remove_frames(frames_written);
            }
        }

        Ok(())
    }
}

fn copy_frames_from_buffer_ref<Dst>(src: &GenericAudioBufferRef<'_>, dst: Dst) -> usize
where
    Dst: BufMut<Sample = f32>,
{
    match src {
        GenericAudioBufferRef::U8(buf) => copy_frames_from_buffer(buf, dst),
        GenericAudioBufferRef::U16(buf) => copy_frames_from_buffer(buf, dst),
        GenericAudioBufferRef::U24(buf) => copy_frames_from_buffer(buf, dst),
        GenericAudioBufferRef::U32(buf) => copy_frames_from_buffer(buf, dst),
        GenericAudioBufferRef::S8(buf) => copy_frames_from_buffer(buf, dst),
        GenericAudioBufferRef::S16(buf) => copy_frames_from_buffer(buf, dst),
        GenericAudioBufferRef::S24(buf) => copy_frames_from_buffer(buf, dst),
        GenericAudioBufferRef::S32(buf) => copy_frames_from_buffer(buf, dst),
        GenericAudioBufferRef::F32(buf) => copy_frames_from_buffer(buf, dst),
        GenericAudioBufferRef::F64(buf) => copy_frames_from_buffer(buf, dst),
    }
}

fn copy_frames_from_buffer<T, Dst>(src: &AudioBuffer<T>, mut dst: Dst) -> usize
where
    f32: FromSample<T>,
    T: Sample,
    Dst: BufMut<Sample = f32>,
{
    src.spec().channels().count();

    match src.spec().channels().count() {
        1 => {
            let dst = dst.channel_mut(0).unwrap();

            debug_assert!(dst.len() >= src.plane(0).unwrap().len());
            for (sample, dst) in src.plane(0).unwrap().iter().zip(dst) {
                *dst = f32::from_sample(*sample);
            }
        }
        2 => {
            debug_assert!(dst.channel_mut(0).unwrap().len() >= src.plane(0).unwrap().len());
            for (src, dst) in src
                .plane(0)
                .unwrap()
                .iter()
                .zip(dst.channel_mut(0).unwrap())
            {
                *dst = f32::from_sample(*src);
            }

            debug_assert!(dst.channel_mut(1).unwrap().len() >= src.plane(1).unwrap().len());
            for (src, dst) in src
                .plane(1)
                .unwrap()
                .iter()
                .zip(dst.channel_mut(1).unwrap())
            {
                *dst = f32::from_sample(*src);
            }
        }
        _ => panic!("unsupported channel config"),
    }

    //let mut frames_written = dst.num_frames();
    // for index in 0..src.spec().channels.count() {
    //     frames_written = frames_written.min(src.chan(index).len());
    // }

    //frames_written
    0
}

#[derive(Debug, Error)]
pub(crate) enum Error {
    /// [`AudioSource`] has finished.
    #[error("eof")]
    Eof,
    #[error(transparent)]
    Decode(symphonia::core::errors::Error),
}

#[derive(Clone, Debug, Default)]
struct FrameBuffer {
    buffer: Sequential<f32>,
    /// Number of frames written in the buffer.
    len: usize,
}

impl FrameBuffer {
    fn new() -> Self {
        Self {
            buffer: Sequential::new(0),
            len: 0,
        }
    }

    /// Returns the number of frames stored in the buffer.
    fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the buffer stores no frames.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reserves capacity for at least `free` number of uninitialized frames at the end.
    fn reserve(&mut self, free: usize) {
        let new_len = self.len + free;

        if self.buffer.num_frames() < new_len {
            self.buffer.resize(new_len);
        }
    }

    /// Moves as many frames as possible from the buffer into `dst`.
    fn move_frames_into(&mut self, dst: &mut [Frame]) -> usize {
        let count = cmp::min(self.len, dst.len());

        for channel in [0, 1] {
            let src = self.buffer.channel_mut(channel).unwrap();

            for (src, dst) in src.iter().zip(dst.iter_mut()).take(count) {
                match channel {
                    // Left
                    0 => dst.left = *src,
                    // Right
                    1 => dst.right = *src,
                    _ => unreachable!(),
                }
            }
        }

        self.remove_frames(count);

        count
    }

    /// Returns the initialized subsection of the buffer.
    fn initialized(&mut self) -> SequentialView<'_, f32> {
        self.buffer.frames_range(..self.len)
    }

    /// Returns the uninitialized subsection of the buffer.
    fn spare_capacity_mut(&mut self) -> SequentialViewMut<'_, f32> {
        self.buffer.frames_range_mut(self.len..)
    }

    /// Increases the len by `extra`, marking the last `extra` elements as initialized.
    fn increase_len(&mut self, extra: usize) {
        self.len += extra;
    }

    /// Removes the first `count` frames.
    fn remove_frames(&mut self, count: usize) {
        for frames in self.buffer.channels_mut() {
            // Remove the first `count` frames from `self.buffer`
            // by shifting all elements starting at index `count` to the left.
            frames.copy_within(count.., 0);
        }

        self.len -= count;
    }
}

#[derive(Debug)]
enum Source {
    Io(IoSource),
    /// A source that will never yield any frames.
    ///
    /// This `Source` only returns [`Error::Eof`].
    Empty,
}

impl Source {
    /// Decodes a single packet from this `Source` into the given [`FrameBuffer`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error::Eof`] when the last packet has been decoded and this `Source` will
    /// never yield another packet.
    fn decode_packet(&mut self, dst: &mut FrameBuffer) -> Result<(), Error> {
        loop {
            match self {
                Source::Empty => return Err(Error::Eof),
                Source::Io(source) => match source.format_reader.next_packet() {
                    Ok(Some(packet)) => {
                        if let Err(err) = source.decoder.decode(&packet) {
                            match err {
                                SymError::IoError(err) => {
                                    tracing::debug!("discarding invalid packet: {err}");
                                    continue;
                                }
                                SymError::DecodeError(err) => {
                                    tracing::debug!("discarding invalid packet: {err}");
                                    continue;
                                }
                                err => return Err(Error::Decode(err)),
                            }
                        }

                        let buffer = source.decoder.decode(&packet).map_err(Error::Decode)?;

                        dst.reserve(buffer.frames());
                        copy_frames_from_buffer_ref(&buffer, &mut dst.spare_capacity_mut());
                        dst.increase_len(buffer.frames());

                        return Ok(());
                    }
                    Ok(None) => return Err(Error::Eof),
                    Err(err) => return Err(Error::Decode(err)),
                },
            }
        }
    }

    fn reset(&mut self, track_id: u32) -> Result<(), Error> {
        match self {
            Self::Empty => {}
            Self::Io(source) => {
                source
                    .format_reader
                    .seek(
                        SeekMode::Accurate,
                        SeekTo::Timestamp {
                            ts: Timestamp::ZERO,
                            track_id,
                        },
                    )
                    .map_err(Error::Decode)?;
                source.decoder.reset();
            }
        }

        Ok(())
    }
}

/// A source backed by some IO object, usually a file or similar.
struct IoSource {
    decoder: Box<dyn AudioDecoder>,
    format_reader: Box<dyn FormatReader>,
}

impl Debug for IoSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("IoSource").finish_non_exhaustive()
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct MediaSourceError(Box<dyn std::error::Error>);

impl MediaSourceError {
    pub fn new<E>(err: E) -> Self
    where
        E: std::error::Error + 'static,
    {
        Self(Box::new(err))
    }

    pub fn msg<T>(msg: T) -> Self
    where
        T: ToString,
    {
        Self(msg.to_string().into())
    }
}

#[derive(Clone, Debug)]
pub struct SoundData {
    pub frames: Vec<Frame>,
    pub sample_rate: u32,
    pub volume: Volume,
}
