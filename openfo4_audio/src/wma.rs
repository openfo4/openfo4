use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::process::{Command, Stdio};

use openfo4_tracing::trace_span;
use symphonia::core::common::FourCc;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::{
    ProbeDataMatchSpec, ProbeFormatData, ProbeableFormat, Score, Scoreable,
};
use symphonia::core::formats::{
    Attachment, FormatId, FormatInfo, FormatOptions, FormatReader, MediaInfo, SeekMode, SeekTo,
    SeekedTo, Track, TrackType,
};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions, ReadBytes, ScopedStream};
use symphonia::core::meta::{ChapterGroup, Metadata};
use symphonia::core::packet::Packet;
use symphonia::default::formats::WavReader;
use thiserror::Error;

pub struct XwmReader<'s> {
    wav: Box<WavReader<'s>>,
}

impl<'s> XwmReader<'s> {
    fn try_new(mss: MediaSourceStream<'s>, opts: FormatOptions) -> Result<Self, Error> {
        let input = ffmpeg_xwm_to_wav(mss).map_err(|err| Error::IoError(io::Error::other(err)))?;
        let wav_src = MediaSourceStream::new(
            Box::new(Cursor::new(input)),
            MediaSourceStreamOptions::default(),
        );

        let wav = Box::new(WavReader::try_new(wav_src, opts)?);
        Ok(Self { wav })
    }
}

impl FormatReader for XwmReader<'_> {
    fn format_info(&self) -> &FormatInfo {
        self.wav.format_info()
    }

    fn media_info(&self) -> &MediaInfo {
        self.wav.media_info()
    }

    fn metadata(&mut self) -> Metadata<'_> {
        self.wav.metadata()
    }

    fn seek(&mut self, mode: SeekMode, to: SeekTo) -> Result<SeekedTo, Error> {
        self.wav.seek(mode, to)
    }

    fn tracks(&self) -> &[Track] {
        self.wav.tracks()
    }

    fn next_packet(&mut self) -> Result<Option<Packet>, Error> {
        self.wav.next_packet()
    }

    fn into_inner<'s>(self: Box<Self>) -> MediaSourceStream<'s>
    where
        Self: 's,
    {
        self.wav.into_inner()
    }

    fn attachments(&self) -> &[Attachment] {
        self.wav.attachments()
    }

    fn chapters(&self) -> Option<&ChapterGroup> {
        self.wav.chapters()
    }

    fn first_track(&self, track_type: TrackType) -> Option<&Track> {
        self.wav.first_track(track_type)
    }

    fn first_track_known_codec(&self, track_type: TrackType) -> Option<&Track> {
        self.wav.first_track_known_codec(track_type)
    }

    fn default_track(&self, track_type: TrackType) -> Option<&Track> {
        self.wav.default_track(track_type)
    }
}

impl ProbeableFormat<'_> for XwmReader<'_> {
    fn try_probe_new(
        mss: MediaSourceStream<'_>,
        opts: FormatOptions,
    ) -> Result<Box<dyn FormatReader + '_>, Error> {
        Ok(Box::new(XwmReader::try_new(mss, opts)?))
    }

    fn probe_data() -> &'static [ProbeFormatData] {
        const DATA: ProbeFormatData = ProbeFormatData {
            spec: ProbeDataMatchSpec {
                extensions: &["xwm"],
                mime_types: &[],
                markers: &[b"RIFF", b"XWMA"],
            },
            info: FormatInfo {
                format: FormatId::new(FourCc::new(*b"XWMA")),
                short_name: "xwma",
                long_name: "xwma",
            },
        };

        &[DATA]
    }
}

impl Scoreable for XwmReader<'_> {
    fn score(mut src: ScopedStream<&mut MediaSourceStream<'_>>) -> Result<Score, Error> {
        let magic1 = src.read_quad_bytes()?;
        src.ignore_bytes(4)?;
        let magic2 = src.read_quad_bytes()?;

        if magic1 != *b"RIFF" && magic2 == *b"XWMA" {
            Ok(Score::Unsupported)
        } else {
            Ok(Score::Supported(255))
        }
    }
}

#[derive(Debug, Error)]
enum ConvError {
    #[error("failed to read source: {0}")]
    ReadSource(io::Error),
    #[error("failed to spawn ffmpeg process: {0}")]
    Spawn(io::Error),
    #[error("IO error while processing: {0}")]
    Io(io::Error),
    #[error("ffmpeg finished with non-zero status; output: {0}")]
    ErrorStatus(String),
}

fn ffmpeg_xwm_to_wav(mut source: impl Read) -> Result<Vec<u8>, ConvError> {
    let _span = trace_span!("ffmpeg_xwm_to_wav").entered();

    let mut input = Vec::new();
    source
        .read_to_end(&mut input)
        .map_err(ConvError::ReadSource)?;

    let mut output = Vec::with_capacity(input.len());

    let mut proc = Command::new("ffmpeg")
        .arg("-i")
        .arg("pipe:")
        .arg("-f")
        .arg("wav")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg("pipe:")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(ConvError::Spawn)?;

    // We need to constantly read from stdout while feeding
    // stdin, otherwise ffmpeg will pause when running out of
    // buffer space.
    let stdin = proc.stdin.as_mut().unwrap();
    let stdout = proc.stdout.as_mut().unwrap();

    let (res0, res1) = std::thread::scope(|s| {
        // FIXME: Yes this is bad, we should prefer an async API for this.
        let t0 = s.spawn(|| stdin.write_all(&input));
        let t1 = s.spawn(|| stdout.read_to_end(&mut output));

        (t0.join().unwrap(), t1.join().unwrap())
    });

    let proc_output = proc.wait_with_output().map_err(ConvError::Io)?;
    if !proc_output.status.success() {
        let msg = String::from_utf8_lossy(&proc_output.stderr).to_string();
        return Err(ConvError::ErrorStatus(msg));
    }

    res0.map_err(ConvError::Io)?;
    res1.map_err(ConvError::Io)?;

    Ok(output)
}

/// Reader for the FUZE audio format.
pub struct FuzReader<'s> {
    xwm: Box<XwmReader<'s>>,
}

impl FuzReader<'_> {
    fn try_new<'s>(
        mut mss: MediaSourceStream<'s>,
        opts: FormatOptions,
    ) -> Result<FuzReader<'s>, Error> {
        // Header:
        // - 4b Magic ("FUZE")
        // - 4b Version
        // - 4b Lip data bytes
        let mut buf = [0; 12];
        mss.read_exact(&mut buf)?;

        if buf[0..4] != *b"FUZE" {
            return Err(Error::IoError(io::Error::other("bad magic for fuz file")));
        }

        let offset = 12
            + u64::from(u32::from_le_bytes(
                <[u8; 4]>::try_from(&buf[8..12]).unwrap(),
            ));
        mss.seek(SeekFrom::Start(offset))?;

        let xwm = Box::new(XwmReader::try_new(mss, opts)?);
        Ok(FuzReader { xwm })
    }
}

impl FormatReader for FuzReader<'_> {
    fn format_info(&self) -> &FormatInfo {
        self.xwm.format_info()
    }

    fn media_info(&self) -> &MediaInfo {
        self.xwm.media_info()
    }

    fn metadata(&mut self) -> Metadata<'_> {
        self.xwm.metadata()
    }

    fn seek(&mut self, mode: SeekMode, to: SeekTo) -> Result<SeekedTo, Error> {
        self.xwm.seek(mode, to)
    }

    fn tracks(&self) -> &[Track] {
        self.xwm.tracks()
    }

    fn next_packet(&mut self) -> Result<Option<Packet>, Error> {
        self.xwm.next_packet()
    }

    fn into_inner<'s>(self: Box<Self>) -> MediaSourceStream<'s>
    where
        Self: 's,
    {
        self.xwm.into_inner()
    }

    fn attachments(&self) -> &[Attachment] {
        self.xwm.attachments()
    }

    fn chapters(&self) -> Option<&ChapterGroup> {
        self.xwm.chapters()
    }

    fn first_track(&self, track_type: TrackType) -> Option<&Track> {
        self.xwm.first_track(track_type)
    }

    fn first_track_known_codec(&self, track_type: TrackType) -> Option<&Track> {
        self.xwm.first_track_known_codec(track_type)
    }

    fn default_track(&self, track_type: TrackType) -> Option<&Track> {
        self.xwm.default_track(track_type)
    }
}

impl ProbeableFormat<'_> for FuzReader<'_> {
    fn try_probe_new(
        mss: MediaSourceStream<'_>,
        opts: FormatOptions,
    ) -> Result<Box<dyn FormatReader + '_>, Error>
    where
        Self: Sized,
    {
        Ok(Box::new(Self::try_new(mss, opts)?))
    }

    fn probe_data() -> &'static [ProbeFormatData] {
        const DATA: ProbeFormatData = ProbeFormatData {
            spec: ProbeDataMatchSpec {
                extensions: &["fuz"],
                mime_types: &[],
                markers: &[b"FUZE"],
            },
            info: FormatInfo {
                format: FormatId::new(FourCc::new(*b"FUZE")),
                short_name: "fuz",
                long_name: "fuz",
            },
        };

        &[DATA]
    }
}

impl<'s> Scoreable for FuzReader<'s> {
    fn score(mut src: ScopedStream<&mut MediaSourceStream<'_>>) -> Result<Score, Error> {
        let magic = src.read_quad_bytes()?;
        if magic != *b"FUZE" {
            Ok(Score::Unsupported)
        } else {
            Ok(Score::Supported(255))
        }
    }
}
