use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
use std::io::Cursor;
use std::ops::{Add, Mul};

use bytemuck::Zeroable;
use bytes::Buf;
use glam::{FloatExt, Quat, Vec3};
use thiserror::Error;

use crate::{PadCursor, Transform};

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("unexpected end of data")]
    Eof,
    #[error("unknown scalar quantization type: {0}")]
    UnknownScalarQuantizationType(u8),
    #[error("unknown quaternion quantization type: {0}")]
    UnknownQuatQuantizationType(u8),
}

pub fn decompress(
    data: &[u8],
    num_transform_tracks: u32,
    num_blocks: u32,
    num_frames: u32,
    num_frames_per_block: u32,
) -> Result<Vec<Transform>, Error> {
    let mut cursor = Cursor::new(data);

    let mut tracks = Vec::new();
    for block in 0..num_blocks {
        let start = block as usize * num_transform_tracks as usize;
        let end = (block as usize + 1) * num_transform_tracks as usize;

        for _ in 0..num_transform_tracks {
            let header = TrackHeader::decode(&mut cursor)?;
            tracks.push(TransformTrack::new(header));
        }

        for track in &mut tracks[start..end] {
            let header = track.header;

            let spline_translation = header
                .translation_flags
                .intersects(Flags::SPLINE_X | Flags::SPLINE_Y | Flags::SPLINE_Z);

            let static_rotation = header
                .rotation_flags
                .intersects(Flags::STATIC_X | Flags::STATIC_Y | Flags::STATIC_Z | Flags::STATIC_W);
            let spline_rotation = header
                .rotation_flags
                .intersects(Flags::SPLINE_X | Flags::SPLINE_Y | Flags::SPLINE_Z | Flags::SPLINE_W);

            let spline_scale = header
                .scale_flags
                .intersects(Flags::SPLINE_X | Flags::SPLINE_Y | Flags::SPLINE_Z);

            cursor.pad(4);

            let translation = if spline_translation {
                TrackValue::Spline(SplineTrackVec3::new(
                    &mut cursor,
                    header.translation_flags,
                    header.translation_quantization,
                )?)
            } else {
                let mut translation = Vec3::ZERO;

                if header.translation_flags.contains(Flags::STATIC_X) {
                    translation.x = f32::decode(&mut cursor)?;
                }

                if header.translation_flags.contains(Flags::STATIC_Y) {
                    translation.y = f32::decode(&mut cursor)?;
                }

                if header.translation_flags.contains(Flags::STATIC_Z) {
                    translation.z = f32::decode(&mut cursor)?;
                }

                TrackValue::Static(translation)
            };

            cursor.pad(4);

            let rotation = if spline_rotation {
                TrackValue::Spline(SplineTrackQuaternion::new(
                    &mut cursor,
                    header.rotation_quantization,
                )?)
            } else if static_rotation {
                let value = decode_quantized_quat(&mut cursor, header.rotation_quantization)?;
                TrackValue::Static(value)
            } else {
                TrackValue::Static(Quat::IDENTITY)
            };

            cursor.pad(4);

            let scale = if spline_scale {
                TrackValue::Spline(SplineTrackVec3::new(
                    &mut cursor,
                    header.scale_flags,
                    header.scale_quantization,
                )?)
            } else {
                let mut scale = Vec3::ONE;

                if header.scale_flags.contains(Flags::STATIC_X) {
                    scale.x = f32::decode(&mut cursor)?;
                }

                if header.scale_flags.contains(Flags::STATIC_Y) {
                    scale.y = f32::decode(&mut cursor)?;
                }

                if header.scale_flags.contains(Flags::STATIC_Z) {
                    scale.z = f32::decode(&mut cursor)?;
                }

                TrackValue::Static(scale)
            };

            track.translation = translation;
            track.rotation = rotation;
            track.scale = scale;
        }

        cursor.pad(16);
    }

    let mut transforms = Vec::new();
    for frame in 0..num_frames {
        let frame = frame % num_frames_per_block;
        let block = frame / num_frames_per_block;

        for track in 0..num_transform_tracks {
            let track = &tracks[block as usize * num_transform_tracks as usize + track as usize];
            transforms.push(Transform {
                translation: track.translation.get(frame, Vec3::ZERO),
                rotation: track.rotation.get(frame),
                scale: track.scale.get(frame, Vec3::ONE),
            });
        }
    }

    Ok(transforms)
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct Flags: u8 {
        const STATIC_X = 0b00000001;
        const STATIC_Y = 0b00000010;
        const STATIC_Z = 0b00000100;
        const STATIC_W = 0b00001000;
        const SPLINE_X = 0b00010000;
        const SPLINE_Y = 0b00100000;
        const SPLINE_Z = 0b01000000;
        const SPLINE_W = 0b10000000;
    }
}

#[derive(Clone, Debug)]
struct SplineTrackVec3 {
    x: Option<Channel<f32>>,
    y: Option<Channel<f32>>,
    z: Option<Channel<f32>>,
    knots: Vec<u8>,
    degree: u8,
}

impl SplineTrackVec3 {
    fn new(
        mut buf: &mut Cursor<&[u8]>,
        flags: Flags,
        quant: ScalarQuantizationType,
    ) -> Result<Self, Error> {
        let num_items = u16::decode(&mut buf)?;
        let degree = u8::decode(&mut buf)?;

        // `num_items` is u16, `degree` is u8, so this cannot overflow.
        let knot_count = u32::from(num_items) + u32::from(degree) + 2;
        // Preallocate for `knot_count`, however to prevent memory
        // exhausting attacks do not allocate more than we have data.
        let mut knots = Vec::with_capacity(usize::min(knot_count as usize, buf.remaining()));
        for _ in 0..knot_count {
            knots.push(u8::decode(&mut buf)?);
        }

        buf.pad(4);

        let mut min_x = 0.0;
        let mut max_x = 0.0;
        let mut min_y = 0.0;
        let mut max_y = 0.0;
        let mut min_z = 0.0;
        let mut max_z = 0.0;

        // Preallocate enough for all items. Note that `num_items` is inclusive.
        // Do not allocate more than we have data remaining to prevent memory exhaustion attacks.
        let alloc_size = usize::min(num_items as usize + 1, buf.remaining() / quant.bytes());

        let mut x = if flags.contains(Flags::SPLINE_X) {
            min_x = f32::decode(&mut buf)?;
            max_x = f32::decode(&mut buf)?;
            Some(Channel::Spline(Vec::with_capacity(alloc_size)))
        } else if flags.contains(Flags::STATIC_X) {
            let value = f32::decode(&mut buf)?;
            Some(Channel::Static(value))
        } else {
            None
        };

        let mut y = if flags.contains(Flags::SPLINE_Y) {
            min_y = f32::decode(&mut buf)?;
            max_y = f32::decode(&mut buf)?;
            Some(Channel::Spline(Vec::with_capacity(alloc_size)))
        } else if flags.contains(Flags::STATIC_Y) {
            let value = f32::decode(&mut buf)?;
            Some(Channel::Static(value))
        } else {
            None
        };

        let mut z = if flags.contains(Flags::SPLINE_Z) {
            min_z = f32::decode(&mut buf)?;
            max_z = f32::decode(&mut buf)?;
            Some(Channel::Spline(Vec::with_capacity(alloc_size)))
        } else if flags.contains(Flags::STATIC_Z) {
            let value = f32::decode(&mut buf)?;
            Some(Channel::Static(value))
        } else {
            None
        };

        for _ in 0..=num_items {
            if let Some(Channel::Spline(values)) = &mut x {
                let value = decode_quantized_scalar(buf, min_x, max_x, quant)?;
                values.push(value);
            }

            if let Some(Channel::Spline(values)) = &mut y {
                let value = decode_quantized_scalar(buf, min_y, max_y, quant)?;
                values.push(value);
            }

            if let Some(Channel::Spline(values)) = &mut z {
                let value = decode_quantized_scalar(buf, min_z, max_z, quant)?;
                values.push(value);
            }
        }

        Ok(Self {
            x,
            y,
            z,
            knots,
            degree,
        })
    }

    fn get_x(&self, frame: u32) -> Option<f32> {
        self.x
            .as_ref()
            .map(|channel| channel.get(&self.knots, self.degree, frame))
    }

    fn get_y(&self, frame: u32) -> Option<f32> {
        self.y
            .as_ref()
            .map(|channel| channel.get(&self.knots, self.degree, frame))
    }

    fn get_z(&self, frame: u32) -> Option<f32> {
        self.z
            .as_ref()
            .map(|channel| channel.get(&self.knots, self.degree, frame))
    }
}

#[derive(Clone, Debug)]
struct SplineTrackQuaternion {
    channel: Channel<Quat>,
    knots: Vec<u8>,
    degree: u8,
}

impl SplineTrackQuaternion {
    fn new(mut buf: &mut Cursor<&[u8]>, quant: QuatQuantizationType) -> Result<Self, Error> {
        let num_items = u16::decode(&mut buf)?;
        let degree = u8::decode(&mut buf)?;

        // `num_items` is u16, `degree` is u8, so this cannot overflow.
        let knot_count = u32::from(num_items) + u32::from(degree) + 2;
        // Preallocate for `knot_count`, however to prevent memory
        // exhausting attacks do not allocate more than we have data.
        let mut knots = Vec::with_capacity(usize::min(knot_count as usize, buf.remaining()));
        for _ in 0..knot_count {
            knots.push(u8::decode(&mut buf)?);
        }

        buf.pad(quant.align());

        // Preallocate for all items. Note that `num_items` is inclusive.
        // Do not allocate more than we have data bytes remaining to prevent
        // memory exhaustion attacks.
        let alloc_size = usize::min(num_items as usize + 1, buf.remaining() / quant.bytes());
        let mut values = Vec::with_capacity(alloc_size);
        for _ in 0..=num_items {
            let quat = decode_quantized_quat(buf, quant)?;
            values.push(quat);
        }

        Ok(Self {
            channel: Channel::Spline(values),
            knots,
            degree,
        })
    }

    fn get(&self, frame: u32) -> Quat {
        self.channel.get(&self.knots, self.degree, frame)
    }
}

#[derive(Clone, Debug)]
enum Channel<T> {
    Static(T),
    Spline(Vec<T>),
}

impl<T> Channel<T> {
    fn get(&self, knots: &[u8], degree: u8, frame: u32) -> T
    where
        T: Point,
    {
        match self {
            Self::Static(value) => *value,
            Self::Spline(values) => {
                let knotspan = find_knot_span(degree, frame, values.len(), knots);
                get_single_point(knotspan, degree, frame, knots, values)
            }
        }
    }
}

fn find_knot_span(degree: u8, value: u32, num_points: usize, knots: &[u8]) -> usize {
    // FIXME: These conditions should be validated when the the spline track is
    // decoded.
    assert!(usize::from(degree) < num_points);
    assert!(num_points < knots.len());

    if value >= u32::from(knots[num_points]) {
        return num_points - 1;
    }

    let mut low = usize::from(degree);
    let mut high = num_points;
    let mut mid = low.midpoint(high);

    // Under the assumption that degree < num_points, high < low is always
    // given, such that all following slice accesses are always in bounds
    // since mid < high is given when we enter the loop and high may only
    // be decreased inside the loop.
    // Note that `mid + 1` is at most `high = num_points`, which we have already
    // bounds checked above.
    while value < u32::from(knots[mid]) || value >= u32::from(knots[mid + 1]) {
        if value < u32::from(knots[mid]) {
            high = mid;
        } else {
            low = mid;
        }

        mid = low.midpoint(high);
    }

    debug_assert!(mid < num_points);

    mid
}

// Basis_ITS1 from Time-Efficient NURBS Curve Evaluation Algorithms
// https://www.researchgate.net/publication/228411721_Time-Efficient_NURBS_Curve_Evaluation_Algorithms
fn get_single_point<T>(
    knot_span_index: usize,
    degree: u8,
    frame: u32,
    knots: &[u8],
    points: &[T],
) -> T
where
    T: Point,
{
    // The caller guarantees that knot_span_index is in bounds.
    debug_assert!(knot_span_index < knots.len());

    // FIXME: Can we validate these ahead of time?
    assert!(knot_span_index >= usize::from(degree));
    assert!(knot_span_index + usize::from(degree) < knots.len());

    let frame = frame as f32;

    // The maximum degree is the maximum number of a u8.
    // We need an extra element to prevent potential out of bounds accesses.
    let mut n = [0.0; 256];
    n[0] = 1.0;

    let degree = usize::from(degree);

    for i in 1..=degree {
        for j in (0..=i - 1).rev() {
            let nom = frame - knots[knot_span_index - j] as f32;
            let denom = knots[knot_span_index + i - j] as f32 - knots[knot_span_index - j] as f32;
            let a = nom / denom;

            // All the following accesses are never out of bounds
            // since j is at most i - 1, and is at most `degree`.
            // As such the value j + 1 is at most i, which is at
            // most `degree` (255) and n is of size 256.
            let tmp = n[j] * a;
            n[j + 1] += n[j] - tmp;
            n[j] = tmp;
        }
    }

    let mut value = T::zeroed();
    for i in 0..=degree {
        let point = points[knot_span_index - i] * n[i];
        value = value + point;
    }

    value
}

/// A point that can be evaulated on a spline.
trait Point: Copy + Zeroable + Add<Output = Self> + Mul<f32, Output = Self> {}

impl Point for f32 {}
impl Point for Quat {}

#[derive(Clone, Debug)]
struct TransformTrack {
    header: TrackHeader,

    translation: TrackValue<Vec3, SplineTrackVec3>,
    rotation: TrackValue<Quat, SplineTrackQuaternion>,
    scale: TrackValue<Vec3, SplineTrackVec3>,
}

impl TransformTrack {
    fn new(header: TrackHeader) -> Self {
        Self {
            header,
            translation: TrackValue::Static(Vec3::ZERO),
            rotation: TrackValue::Static(Quat::IDENTITY),
            scale: TrackValue::Static(Vec3::ONE),
        }
    }
}

#[derive(Clone, Debug)]
pub enum TrackValue<Static, Spline> {
    Static(Static),
    Spline(Spline),
}

impl TrackValue<Vec3, SplineTrackVec3> {
    fn get(&self, frame: u32, or_default: Vec3) -> Vec3 {
        match self {
            Self::Static(value) => *value,
            Self::Spline(spline) => {
                let x = spline.get_x(frame).unwrap_or(or_default.x);
                let y = spline.get_y(frame).unwrap_or(or_default.y);
                let z = spline.get_z(frame).unwrap_or(or_default.z);
                Vec3 { x, y, z }
            }
        }
    }
}

impl TrackValue<Quat, SplineTrackQuaternion> {
    fn get(&self, frame: u32) -> Quat {
        match self {
            Self::Static(value) => *value,
            Self::Spline(spline) => spline.get(frame),
        }
    }
}

fn decode_quantized_scalar(
    buf: &mut Cursor<&[u8]>,
    min: f32,
    max: f32,
    typ: ScalarQuantizationType,
) -> Result<f32, Error> {
    let s = match typ {
        ScalarQuantizationType::Bits8 => u8::decode(buf)? as f32 / u8::MAX as f32,
        ScalarQuantizationType::Bits16 => u16::decode(buf)? as f32 / u16::MAX as f32,
    };

    Ok(f32::lerp(min, max, s))
}

fn decode_quantized_quat(
    mut buf: &mut Cursor<&[u8]>,
    typ: QuatQuantizationType,
) -> Result<Quat, Error> {
    match typ {
        QuatQuantizationType::Polar32 => decode_quantized_quat_polar32(buf),
        QuatQuantizationType::ThreeComp40 => decode_quantized_quat_threecomp40(buf),
        QuatQuantizationType::ThreeComp48 => decode_quantized_quat_threecomp48(buf),
        QuatQuantizationType::Uncompressed => {
            let x = f32::decode(&mut buf)?;
            let y = f32::decode(&mut buf)?;
            let z = f32::decode(&mut buf)?;
            let w = f32::decode(&mut buf)?;
            Ok(Quat::from_xyzw(x, y, z, w))
        }
    }
}

fn decode_quantized_quat_polar32(buf: &mut Cursor<&[u8]>) -> Result<Quat, Error> {
    // r 10 bits
    // phi theta 18 bits
    // 4 sign bits
    const R_MASK: u32 = (1 << 10) - 1;
    const R_FRAC: f32 = 1.0 / R_MASK as f32;
    const PHI_FRAC: f32 = FRAC_PI_2 / (R_MASK >> 1) as f32;
    const PHI_THETA_MASK: u32 = (1 << 18) - 1;

    let val = u32::decode(buf)?;

    let mut r = f32::from_bits(val >> 18) * R_FRAC;
    r = 1.0 - (r * r);

    let phi_theta = (val & PHI_THETA_MASK) as f32;

    let mut phi = phi_theta.sqrt().floor();
    let mut theta = 0.0;

    if phi > 0.0 {
        theta = FRAC_PI_4 * (phi_theta - (phi * phi)) / phi;
        phi *= PHI_FRAC;
    }

    let magnitude = (1.0 - r * r).sqrt();

    let mut x = phi.sin() * theta.cos() * magnitude;
    let mut y = phi.sin() * theta.sin() * magnitude;
    let mut z = phi.cos() * magnitude;
    let mut w = r;

    if (val & 0x10000000) != 0 {
        x = -x;
    }

    if (val & 0x20000000) != 0 {
        y = -y;
    }

    if (val & 0x40000000) != 0 {
        z = -z;
    }

    if (val & 0x80000000) != 0 {
        w = -w;
    }

    Ok(Quat::from_xyzw(x, y, z, w))
}

fn decode_quantized_quat_threecomp40(mut buf: &mut Cursor<&[u8]>) -> Result<Quat, Error> {
    // Quaternion encoded in 40 bits:
    // - 3x12 bit quantized floats in the range -1/sqrt(2)..+1/sqrt(2).
    // - 2 bits indicating the index of the fourth (implicit) value.
    // - 1 bit indicating the sign of the fourth (implicit) value.
    // See also: "Optimizing Orientation" on https://gafferongames.com/post/snapshot_compression/

    const BITS_PER_COMPONENT: u32 = 12;
    const MASK: u64 = (1 << BITS_PER_COMPONENT) - 1;
    const POSITIVE_MASK: u64 = MASK >> 1;
    // The float value that a int value of 1 represents.
    // FRAC = (1 / sqrt(2) ) / POSITIVE_MASK
    const FRAC: f32 = 0.00034543565275356496;

    let val0 = u8::decode(&mut buf)?;
    let val1 = u8::decode(&mut buf)?;
    let val2 = u8::decode(&mut buf)?;
    let val3 = u8::decode(&mut buf)?;
    let val4 = u8::decode(&mut buf)?;

    let val = u64::from_le_bytes([val0, val1, val2, val3, val4, 0, 0, 0]);

    let mut x = ((val >> (BITS_PER_COMPONENT * 0)) & MASK) as i64;
    let mut y = ((val >> (BITS_PER_COMPONENT * 1)) & MASK) as i64;
    let mut z = ((val >> (BITS_PER_COMPONENT * 2)) & MASK) as i64;

    let w_index = (val >> (BITS_PER_COMPONENT * 3)) & 3;
    let w_sign = (val >> (BITS_PER_COMPONENT * 3 + 2) & 1) != 0;

    x -= POSITIVE_MASK as i64;
    y -= POSITIVE_MASK as i64;
    z -= POSITIVE_MASK as i64;

    let x = x as f32 * FRAC;
    let y = y as f32 * FRAC;
    let z = z as f32 * FRAC;
    // w = sqrt(1 - x^2 - y^2 - z ^2)
    let mut w = f32::max(1.0 - (x * x) - (y * y) - (z * z), 0.0).sqrt();

    if w_sign {
        w = -w;
    }

    let components = match w_index {
        0 => [w, x, y, z],
        1 => [x, w, y, z],
        2 => [x, y, w, z],
        3 => [x, y, z, w],
        _ => unreachable!(),
    };

    Ok(Quat::from_array(components))
}

fn decode_quantized_quat_threecomp48(mut buf: &mut Cursor<&[u8]>) -> Result<Quat, Error> {
    const BITS_PER_COMPONENT: u32 = 15;
    const MASK: u16 = (1 << BITS_PER_COMPONENT) - 1;
    const POSITIVE_MASK: u16 = MASK >> 1;
    // The float value that a int value of 1 represents.
    // FRAC = (1 / sqrt(2) ) / POSITIVE_MASK
    const FRAC: f32 = 0.000043161006;

    let x = u16::decode(&mut buf)?;
    let y = u16::decode(&mut buf)?;
    let z = u16::decode(&mut buf)?;

    let w_index = (((y >> 14) & 2) | ((x >> 15) & 1)) as u8;
    let w_sign = (z >> 15) != 0;

    let mut x = (x & MASK) as i16;
    let mut y = (y & MASK) as i16;
    let mut z = (z & MASK) as i16;

    x -= POSITIVE_MASK as i16;
    y -= POSITIVE_MASK as i16;
    z -= POSITIVE_MASK as i16;

    let x = x as f32 * FRAC;
    let y = y as f32 * FRAC;
    let z = z as f32 * FRAC;
    let mut w = f32::max(1.0 - (x * x) - (y * y) - (z * z), 0.0).sqrt();

    if w_sign {
        w = -w;
    }

    let components = match w_index {
        0 => [w, x, y, z],
        1 => [x, w, y, z],
        2 => [x, y, w, z],
        3 => [x, y, z, w],
        _ => unreachable!(),
    };

    Ok(Quat::from_array(components))
}

#[derive(Copy, Clone, Debug)]
struct TrackHeader {
    translation_quantization: ScalarQuantizationType,
    translation_flags: Flags,
    rotation_quantization: QuatQuantizationType,
    rotation_flags: Flags,
    scale_quantization: ScalarQuantizationType,
    scale_flags: Flags,
}

impl Decode for TrackHeader {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        // The header is 32-bits long and contains a single byte with the
        // quantization types for translation, rotation and scale followed
        // by a byte for the flags for translation, rotation and scale:
        //
        // LSB      MSB
        // 0  2    6  8
        // |TQ| RQ |SQ|
        // |    TF    |
        // |    RF    |
        // |    SF    |
        //
        // Where TQ: Translation Quantization, RQ: Rotation Quantization,
        // SQ: Scale Quantization, TF: Translation Flags, RF: Rotation Flags,
        // SF: Scale Flags.
        let quantization_bits = u8::decode(&mut buf)?;

        let translation_quantization = match quantization_bits & 0x3 {
            0 => ScalarQuantizationType::Bits8,
            1 => ScalarQuantizationType::Bits16,
            value => return Err(Error::UnknownScalarQuantizationType(value)),
        };

        let rotation_quantization = match (quantization_bits >> 2) & 0xf {
            0 => QuatQuantizationType::Polar32,
            1 => QuatQuantizationType::ThreeComp40,
            2 => QuatQuantizationType::ThreeComp48,
            5 => QuatQuantizationType::Uncompressed,
            value => return Err(Error::UnknownQuatQuantizationType(value)),
        };

        let scale_quantization = match (quantization_bits >> 6) & 0x3 {
            0 => ScalarQuantizationType::Bits8,
            1 => ScalarQuantizationType::Bits16,
            value => return Err(Error::UnknownScalarQuantizationType(value)),
        };

        let translation_flags = Flags::from_bits_truncate(u8::decode(&mut buf)?);
        let rotation_flags = Flags::from_bits_truncate(u8::decode(&mut buf)?);
        let scale_flags = Flags::from_bits_truncate(u8::decode(&mut buf)?);

        Ok(Self {
            translation_quantization,
            translation_flags,
            rotation_quantization,
            rotation_flags,
            scale_quantization,
            scale_flags,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScalarQuantizationType {
    Bits8,
    Bits16,
}

impl ScalarQuantizationType {
    /// Returns the number of bytes needed to store one scalar.
    const fn bytes(&self) -> usize {
        match self {
            Self::Bits8 => 1,
            Self::Bits16 => 2,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum QuatQuantizationType {
    Polar32,
    ThreeComp40,
    ThreeComp48,
    Uncompressed,
}

impl QuatQuantizationType {
    const fn bytes(&self) -> usize {
        match self {
            Self::Polar32 => 4,
            Self::ThreeComp40 => 5,
            Self::ThreeComp48 => 6,
            Self::Uncompressed => 16,
        }
    }

    fn align(&self) -> u64 {
        match self {
            Self::Polar32 => 4,
            Self::ThreeComp40 => 1,
            Self::ThreeComp48 => 2,
            Self::Uncompressed => 4,
        }
    }
}

trait Decode: Sized {
    fn decode<B>(buf: B) -> Result<Self, Error>
    where
        B: Buf;
}

impl Decode for u8 {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        if buf.has_remaining() {
            Ok(buf.get_u8())
        } else {
            Err(Error::Eof)
        }
    }
}

impl Decode for u16 {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        if buf.remaining() >= 2 {
            Ok(buf.get_u16_le())
        } else {
            Err(Error::Eof)
        }
    }
}

impl Decode for u32 {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        if buf.remaining() >= 4 {
            Ok(buf.get_u32_le())
        } else {
            Err(Error::Eof)
        }
    }
}

impl Decode for f32 {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        if buf.remaining() >= 4 {
            Ok(buf.get_f32_le())
        } else {
            Err(Error::Eof)
        }
    }
}
