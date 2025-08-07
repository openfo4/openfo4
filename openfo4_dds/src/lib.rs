//! DDS file format reader
//!
//! # References
//!
//! <https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dx-graphics-dds-pguide>
//! <https://registry.khronos.org/DataFormat/specs/1.1/dataformat.1.1.html>

use bitflags::bitflags;
use bytes::{Buf, Bytes};
use glam::UVec2;
use thiserror::Error;

const MAGIC: u32 = 0x20534444;

#[derive(Debug, Error)]
pub enum Error {
    #[error("bad magic")]
    BadMagic,
    #[error("unexpected eof")]
    Eof,
    #[error("unknown image format: {0:?}")]
    UnknownFormat([u8; 4]),
    #[error("file contains no data")]
    NoData,
    #[error("unsupported RGB bit count {0}, must be a factor of 8 and less than or equal to 32")]
    UnsupportedRgbBitCount(u32),
    #[error("unknown dxgi format: {0}")]
    UnknownDxgiFormat(u32),
}

#[derive(Clone, Debug)]
pub struct Image {
    pub format: Format,
    pub alpha_mode: AlphaMode,
    pub mips: Vec<MipLevel>,
}

impl Image {
    pub fn parse(mut buf: &[u8]) -> Result<Self, Error> {
        let magic = u32::decode(&mut buf)?;
        if magic != MAGIC {
            return Err(Error::BadMagic);
        }

        let header = Header::decode(&mut buf)?;

        let mut dx10header = None;
        if header.pixel_format.flags.contains(PixelFormatFlags::FOURCC)
            && header.pixel_format.four_cc == *b"DX10"
        {
            dx10header = Some(Dxt10Header::decode(&mut buf)?);
        }

        let mip_size = UVec2::new(header.width, header.height);
        let mip_count = if header.flags.contains(HeaderFlags::MIPMAP_COUNT) {
            header.mipmap_count
        } else {
            1
        };

        let (format, alpha_mode) = if let Some(dx10header) = dx10header {
            (dx10header.format, dx10header.alpha_mode)
        } else if header.pixel_format.flags.contains(PixelFormatFlags::FOURCC) {
            match &header.pixel_format.four_cc {
                b"DXT1" => (Format::Bc1Unorm, AlphaMode::Straight),
                b"DXT2" => (Format::Bc2Unorm, AlphaMode::Premultiplied),
                b"DXT3" => (Format::Bc2Unorm, AlphaMode::Straight),
                b"DXT4" => (Format::Bc3Unorm, AlphaMode::Premultiplied),
                b"DXT5" => (Format::Bc3Unorm, AlphaMode::Straight),
                b"BC5U" => (Format::Bc5Unorm, AlphaMode::Opaque),
                _ => return Err(Error::UnknownFormat(header.pixel_format.four_cc)),
            }
        } else if header.pixel_format.flags.contains(PixelFormatFlags::RGB) {
            if header.pixel_format.rgb_bit_count % 8 != 0 || header.pixel_format.rgb_bit_count > 32
            {
                return Err(Error::UnsupportedRgbBitCount(
                    header.pixel_format.rgb_bit_count,
                ));
            }

            if header
                .pixel_format
                .flags
                .contains(PixelFormatFlags::ALPHAPIXELS)
            {
                (Format::Rgba8Unorm, AlphaMode::Straight)
            } else {
                (Format::Rgba8Unorm, AlphaMode::Straight)
            }
        } else {
            return Err(Error::NoData);
        };

        let mips = if format.is_compressed() || dx10header.is_some() {
            read_copy(buf, mip_size, mip_count, format)?
        } else {
            read_uncompressed(&header, buf, mip_size, mip_count)?
        };

        Ok(Self {
            format,
            mips,
            alpha_mode,
        })
    }
}

fn read_uncompressed(
    header: &Header,
    mut buf: &[u8],
    mut mip_size: UVec2,
    mip_count: u32,
) -> Result<Vec<MipLevel>, Error> {
    if mip_size.x == 0 || mip_size.y == 0 {
        return Ok(Vec::new());
    }

    let mut mips = Vec::new();
    for _ in 0..mip_count {
        let size = mip_size.x as usize * mip_size.y as usize;
        let size_in_bytes = size * (header.pixel_format.rgb_bit_count / 8) as usize;

        if size_in_bytes == 0 {
            break;
        }

        if buf.remaining() < size_in_bytes {
            return Err(Error::Eof);
        }

        let mut bytes = Vec::with_capacity(size_in_bytes);

        assert_eq!(header.pixel_format.rgb_bit_count, 32);

        for _ in 0..size {
            // let mut pixel = 0;
            let pixel = buf.get_u32().swap_bytes();
            // for index in 0..header.pixel_format.rgb_bit_count / 8 {
            //     pixel |= (buf.get_u8() as u32) << (index * 8);
            // }

            let b = pixel & 0xff;
            let g = (pixel & 0xff_00) >> 8;
            let r = (pixel & 0xff_00_00) >> 16;
            let a = (pixel & 0xff_00_00_00) >> 24;

            // let r = extract_bits(pixel, header.pixel_format.r_bit_mask);
            // let g = extract_bits(pixel, header.pixel_format.g_bit_mask);
            // let b = extract_bits(pixel, header.pixel_format.b_bit_mask);
            // let a = extract_bits(pixel, header.pixel_format.a_bit_mask);

            bytes.push(r as u8);
            bytes.push(g as u8);
            bytes.push(b as u8);

            if header
                .pixel_format
                .flags
                .contains(PixelFormatFlags::ALPHAPIXELS)
            {
                bytes.push(a as u8);
            }
        }

        mips.push(MipLevel {
            size: mip_size,
            bytes: Bytes::from(bytes),
        });

        mip_size = UVec2::max(UVec2::ONE, mip_size / 2);
    }

    Ok(mips)
}

/// Extracts all bits from `a` that are set in `mask` and moves them into a contigous region.
///
/// This can be used to create an integer from a non-contigous bitmask. This is the same operation
/// as the x86 `pext_u32` instruction.
///
/// See also:
/// - https://www.felixcloutier.com/x86/pext
fn extract_bits(a: u32, mask: u32) -> u32 {
    #[cfg(not(all(target_arch = "x86_64", target_feature = "bmi2")))]
    {
        extract_bits_naive(a, mask)
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
    {
        extract_bits_bmi2(a, mask)
    }
}

fn extract_bits_naive(a: u32, mask: u32) -> u32 {
    let src = a & mask;
    let mut dst = 0;

    let mut next_bit = 0;
    for index in 0..u32::BITS {
        let bit = (src & (1 << index)) >> index;
        dst |= bit << next_bit;
        next_bit += bit;
    }

    dst
}

#[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
fn extract_bits_bmi2(a: u32, mask: u32) -> u32 {
    unsafe { core::arch::x86_64::_pext_u32(a, mask) }
}

fn read_copy(
    mut buf: &[u8],
    mut mip_size: UVec2,
    mip_count: u32,
    format: Format,
) -> Result<Vec<MipLevel>, Error> {
    let mut mips = Vec::new();

    if mip_size.x == 0 || mip_size.y == 0 {
        return Ok(Vec::new());
    }

    for _ in 0..mip_count {
        let size = match format {
            Format::Bc1Unorm | Format::Bc1UnormSrgb => {
                (u32::max(1, mip_size.x / 4) * u32::max(1, mip_size.y / 4) * 8) as usize
            }
            Format::Bc2Unorm
            | Format::Bc2UnormSrgb
            | Format::Bc3Unorm
            | Format::Bc3UnormSrgb
            | Format::Bc4Unorm
            | Format::Bc4Snorm
            | Format::Bc5Unorm
            | Format::Bc5Snorm
            | Format::Bc7Unorm
            | Format::Bc7UnormSrgb => {
                (u32::max(1, mip_size.x / 4) * u32::max(1, mip_size.y / 4) * 16) as usize
            }
            Format::Rgba32Float | Format::Rgba32Uint | Format::Rgba32Sint => {
                u32::max(1, mip_size.x) as usize * u32::max(1, mip_size.y) as usize * 16
            }
            Format::Rgb32Float | Format::Rgb32Uint | Format::Rgb32Sint => {
                u32::max(1, mip_size.x) as usize * u32::max(1, mip_size.y) as usize * 12
            }
            Format::Rg32Float
            | Format::Rg32Uint
            | Format::Rg32Sint
            | Format::Rgba16Float
            | Format::Rgba16Unorm
            | Format::Rgba16Uint
            | Format::Rgba16Snorm
            | Format::Rgba16Sint => {
                u32::max(1, mip_size.x) as usize * u32::max(1, mip_size.y) as usize * 8
            }
            Format::R32Float
            | Format::R32Uint
            | Format::R32Sint
            | Format::Rg16Float
            | Format::Rg16Unorm
            | Format::Rg16Uint
            | Format::Rg16Snorm
            | Format::Rg16Sint
            | Format::Rgba8Unorm
            | Format::Rgba8UnormSrgb
            | Format::Rgba8Uint
            | Format::Rgba8Snorm
            | Format::Rgba8Sint
            | Format::Bgra8Unorm
            | Format::Bgra8UnormSrgb => {
                u32::max(1, mip_size.x) as usize * u32::max(1, mip_size.y) as usize * 4
            }
            Format::R16Float
            | Format::R16Unorm
            | Format::R16Uint
            | Format::R16Snorm
            | Format::R16Sint
            | Format::Rg8Unorm
            | Format::Rg8Uint
            | Format::Rg8Snorm
            | Format::Rg8Sint => {
                u32::max(1, mip_size.x) as usize * u32::max(1, mip_size.y) as usize * 2
            }
            Format::R8Unorm | Format::R8Uint | Format::R8Snorm | Format::R8Sint => {
                u32::max(1, mip_size.x) as usize * u32::max(1, mip_size.y) as usize
            }
        };

        if buf.remaining() < size {
            return Err(Error::Eof);
        }

        let bytes = buf.copy_to_bytes(size);
        mips.push(MipLevel {
            size: mip_size,
            bytes,
        });

        mip_size = UVec2::max(UVec2::ONE, mip_size / 2);
    }

    Ok(mips)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AlphaMode {
    Opaque,
    Straight,
    Premultiplied,
}

trait Decode: Sized {
    fn decode<B>(buf: B) -> Result<Self, Error>
    where
        B: Buf;
}

impl<const N: usize> Decode for [u8; N] {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        if buf.remaining() < N {
            Err(Error::Eof)
        } else {
            let mut arr = [0; N];
            buf.copy_to_slice(&mut arr);
            Ok(arr)
        }
    }
}

macro_rules! decode_primitive {
    ($($t:ty),*) => {
        $(
            impl Decode for $t {
                fn decode<B>(buf: B) -> Result<Self, Error>
                where
                    B: Buf,
                {
                    <[u8; size_of::<Self>()]>::decode(buf).map(Self::from_le_bytes)
                }
            }
        )*

    };
}

decode_primitive!(u8, u16, u32, u64);

#[derive(Copy, Clone, Debug)]
struct Header {
    size: u32,
    flags: HeaderFlags,
    height: u32,
    width: u32,
    pitch_or_linear_size: u32,
    depth: u32,
    mipmap_count: u32,
    pixel_format: PixelFormat,
    caps: u32,
    caps2: u32,
    caps3: u32,
    caps4: u32,
}

impl Decode for Header {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        let size = u32::decode(&mut buf)?;
        let flags = u32::decode(&mut buf)?;
        let height = u32::decode(&mut buf)?;
        let width = u32::decode(&mut buf)?;
        let pitch_or_linear_size = u32::decode(&mut buf)?;
        let depth = u32::decode(&mut buf)?;
        let mipmap_count = u32::decode(&mut buf)?;
        // Reserved1
        for _ in 0..11 {
            u32::decode(&mut buf)?;
        }
        let pixel_format = PixelFormat::decode(&mut buf)?;
        let caps = u32::decode(&mut buf)?;
        let caps2 = u32::decode(&mut buf)?;
        let caps3 = u32::decode(&mut buf)?;
        let caps4 = u32::decode(&mut buf)?;
        // Reseved2
        u32::decode(&mut buf)?;

        Ok(Self {
            size,
            flags: HeaderFlags::from_bits_truncate(flags),
            height,
            width,
            pitch_or_linear_size,
            depth,
            mipmap_count,
            pixel_format,
            caps,
            caps2,
            caps3,
            caps4,
        })
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct HeaderFlags: u32 {
        /// Required in every file.
        const CAPS = 0x1;
        /// Required in every file.
        const HEIGHT = 0x2;
        /// Required in every file.
        const WIDTH = 0x4;
        /// Pitch is provided for an uncompressed texture.
        const PITCH = 0x8;
        /// Required for every file.
        const PIXELFORMAT = 0x1000;
        const MIPMAP_COUNT = 0x20000;
        const LINEAR_SIZE = 0x80000;
        const DEPTH = 0x800000;
    }
}

#[derive(Copy, Clone, Debug)]
struct PixelFormat {
    size: u32,
    flags: PixelFormatFlags,
    four_cc: [u8; 4],
    rgb_bit_count: u32,
    r_bit_mask: u32,
    g_bit_mask: u32,
    b_bit_mask: u32,
    a_bit_mask: u32,
}

impl Decode for PixelFormat {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        let size = u32::decode(&mut buf)?;
        let flags = u32::decode(&mut buf)?;
        let four_cc = <[u8; 4]>::decode(&mut buf)?;
        let rgb_bit_count = u32::decode(&mut buf)?;
        let r_bit_mask = u32::decode(&mut buf)?;
        let g_bit_mask = u32::decode(&mut buf)?;
        let b_bit_mask = u32::decode(&mut buf)?;
        let a_bit_mask = u32::decode(&mut buf)?;

        Ok(Self {
            size,
            flags: PixelFormatFlags::from_bits_truncate(flags),
            four_cc,
            rgb_bit_count,
            r_bit_mask,
            g_bit_mask,
            b_bit_mask,
            a_bit_mask,
        })
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    struct PixelFormatFlags: u32 {
        const ALPHAPIXELS = 0x1;
        const ALPHA = 0x2;
        const FOURCC = 0x4;
        const RGB = 0x40;
        const YUV = 0x200;
        const LUMINANCE = 0x20000;
    }
}

#[derive(Copy, Clone, Debug)]
struct Dxt10Header {
    format: Format,
    dimension: u32,
    array_size: u32,
    alpha_mode: AlphaMode,
}

impl Decode for Dxt10Header {
    fn decode<B>(mut buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        let format = Format::decode(&mut buf)?;
        let dimension = u32::decode(&mut buf)?;
        let _misc_flag = u32::decode(&mut buf)?;
        let array_size = u32::decode(&mut buf)?;
        let misc_flags2 = u32::decode(&mut buf)?;

        // The lower 3 bits contain the alpha mode.
        let alpha_mode = match misc_flags2 & 0b111 {
            // Unknown, assume straight
            0 => AlphaMode::Straight,
            1 => AlphaMode::Straight,
            2 => AlphaMode::Premultiplied,
            3 => AlphaMode::Opaque,
            // Custom, i.e. the alpha channel has no alpha
            // meaning. If this mode is used the texture
            // user must already know how they intend to
            // use this texture and we can write anything here.
            4 => AlphaMode::Opaque,
            _ => AlphaMode::Straight,
        };

        Ok(Self {
            format,
            dimension,
            array_size,
            alpha_mode,
        })
    }
}

#[derive(Clone, Debug)]
pub struct MipLevel {
    pub size: UVec2,
    pub bytes: Bytes,
}

/// All possible DXGI texture formats.
///
/// <https://learn.microsoft.com/en-us/windows/win32/api/dxgiformat/ne-dxgiformat-dxgi_format>
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Rgba32Float,
    Rgba32Uint,
    Rgba32Sint,
    Rgb32Float,
    Rgb32Uint,
    Rgb32Sint,
    Rgba16Float,
    Rgba16Unorm,
    Rgba16Uint,
    Rgba16Snorm,
    Rgba16Sint,
    Rg32Float,
    Rg32Uint,
    Rg32Sint,
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Rgba8Uint,
    Rgba8Snorm,
    Rgba8Sint,
    Rg16Float,
    Rg16Unorm,
    Rg16Uint,
    Rg16Snorm,
    Rg16Sint,
    R32Float,
    R32Uint,
    R32Sint,
    Rg8Unorm,
    Rg8Uint,
    Rg8Snorm,
    Rg8Sint,
    R16Float,
    R16Unorm,
    R16Uint,
    R16Snorm,
    R16Sint,
    R8Unorm,
    R8Uint,
    R8Snorm,
    R8Sint,
    Bc1Unorm,
    Bc1UnormSrgb,
    Bc2Unorm,
    Bc2UnormSrgb,
    Bc3Unorm,
    Bc3UnormSrgb,
    Bc4Unorm,
    Bc4Snorm,
    Bc5Unorm,
    Bc5Snorm,
    Bgra8Unorm,
    Bgra8UnormSrgb,
    Bc7Unorm,
    Bc7UnormSrgb,
}

impl Format {
    /// Returns `true` if the format contains a compressed texture.
    fn is_compressed(&self) -> bool {
        match self {
            Self::Bc1Unorm => true,
            Self::Bc1UnormSrgb => true,
            Self::Bc2Unorm => true,
            Self::Bc2UnormSrgb => true,
            Self::Bc3Unorm => true,
            Self::Bc3UnormSrgb => true,
            Self::Bc4Unorm => true,
            Self::Bc4Snorm => true,
            Self::Bc5Unorm => true,
            Self::Bc5Snorm => true,
            Self::Bc7Unorm => true,
            Self::Bc7UnormSrgb => true,
            _ => false,
        }
    }
}

impl Decode for Format {
    fn decode<B>(buf: B) -> Result<Self, Error>
    where
        B: Buf,
    {
        let value = u32::decode(buf)?;
        Self::try_from(value).map_err(|UnknownFormat(value)| Error::UnknownDxgiFormat(value))
    }
}

#[derive(Copy, Clone, Debug, Error)]
#[error("unknown dxgi format: {0}")]
pub struct UnknownFormat(u32);

impl TryFrom<u32> for Format {
    type Error = UnknownFormat;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(Self::Rgba32Float),
            3 => Ok(Self::Rgba32Uint),
            4 => Ok(Self::Rgba32Sint),
            6 => Ok(Self::Rgb32Float),
            7 => Ok(Self::Rgb32Uint),
            8 => Ok(Self::Rgb32Sint),
            10 => Ok(Self::Rgba16Float),
            11 => Ok(Self::Rgba16Unorm),
            12 => Ok(Self::Rgba16Uint),
            13 => Ok(Self::Rgba16Snorm),
            14 => Ok(Self::Rgba16Sint),
            16 => Ok(Self::Rg32Float),
            17 => Ok(Self::Rg32Uint),
            18 => Ok(Self::Rg32Sint),
            28 => Ok(Self::Rgba8Unorm),
            29 => Ok(Self::Rgba8UnormSrgb),
            30 => Ok(Self::Rgba8Uint),
            31 => Ok(Self::Rgba8Snorm),
            32 => Ok(Self::Rgba8Sint),
            34 => Ok(Self::Rg16Float),
            35 => Ok(Self::Rg16Unorm),
            36 => Ok(Self::Rg16Uint),
            37 => Ok(Self::Rg16Snorm),
            38 => Ok(Self::Rg16Sint),
            41 => Ok(Self::R32Float),
            42 => Ok(Self::R32Uint),
            43 => Ok(Self::R32Sint),
            49 => Ok(Self::Rg8Unorm),
            50 => Ok(Self::Rg8Uint),
            51 => Ok(Self::Rg8Snorm),
            52 => Ok(Self::Rg8Sint),
            54 => Ok(Self::R16Float),
            56 => Ok(Self::R16Unorm),
            57 => Ok(Self::R16Uint),
            58 => Ok(Self::R16Snorm),
            59 => Ok(Self::R16Sint),
            61 => Ok(Self::R8Unorm),
            62 => Ok(Self::R8Uint),
            63 => Ok(Self::R8Snorm),
            64 => Ok(Self::R8Sint),
            71 => Ok(Self::Bc1Unorm),
            72 => Ok(Self::Bc1UnormSrgb),
            74 => Ok(Self::Bc2Unorm),
            75 => Ok(Self::Bc2UnormSrgb),
            77 => Ok(Self::Bc3Unorm),
            78 => Ok(Self::Bc3UnormSrgb),
            80 => Ok(Self::Bc4Unorm),
            81 => Ok(Self::Bc4Snorm),
            83 => Ok(Self::Bc5Unorm),
            84 => Ok(Self::Bc5Snorm),
            87 => Ok(Self::Bgra8Unorm),
            91 => Ok(Self::Bgra8UnormSrgb),
            98 => Ok(Self::Bc7Unorm),
            99 => Ok(Self::Bc7UnormSrgb),
            _ => Err(UnknownFormat(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::extract_bits_naive;

    use super::extract_bits;

    #[test]
    fn test_extract_bits() {
        let src = 0b01_01_11_01;
        let mask = 0b11_00_11_00;
        let dst = 0b00_00_01_11;

        assert_eq!(extract_bits(src, mask), dst);
        assert_eq!(extract_bits_naive(src, mask), extract_bits(src, mask));
    }
}
