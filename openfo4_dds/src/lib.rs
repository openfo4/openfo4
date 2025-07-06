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
}

#[derive(Clone, Debug)]
pub struct Image {
    pub format: Format,
    pub mips: Vec<MipLevel>,
}

impl Image {
    pub fn parse(mut buf: &[u8]) -> Result<Self, Error> {
        let magic = u32::decode(&mut buf)?;
        if magic != MAGIC {
            return Err(Error::BadMagic);
        }

        let header = Header::decode(&mut buf)?;

        if header.pixel_format.four_cc == *b"DX10" {
            // TODO: Extended header
            todo!();
        }

        let mip_size = UVec2::new(header.width, header.height);
        let mip_count = if header.flags.contains(HeaderFlags::MIPMAP_COUNT) {
            header.mipmap_count
        } else {
            1
        };

        let format = if header.pixel_format.flags.contains(PixelFormatFlags::FOURCC) {
            match &header.pixel_format.four_cc {
                b"DXT1" => Format::Dxt1,
                b"DXT2" => Format::Dxt2,
                b"DXT3" => Format::Dxt3,
                b"DXT4" => Format::Dxt4,
                b"DXT5" => Format::Dxt5,
                b"BC5U" => Format::Bc5U,
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
                Format::Rgba8
            } else {
                Format::Rgb8
            }
        } else {
            return Err(Error::NoData);
        };

        let mips = match format {
            Format::Rgb8 | Format::Rgba8 => read_uncompressed(&header, buf, mip_size, mip_count)?,
            Format::Dxt1
            | Format::Dxt2
            | Format::Dxt3
            | Format::Dxt4
            | Format::Dxt5
            | Format::Bc5U => {
                // All formats encode a 4x4 block.
                // Block size in bytes.
                let block_size = match format {
                    Format::Dxt1 => 8,
                    Format::Dxt2 => 16,
                    Format::Dxt3 => 16,
                    Format::Dxt4 => 16,
                    Format::Dxt5 => 16,
                    Format::Bc5U => 16,
                    Format::Rgb8 | Format::Rgba8 => unreachable!(),
                };

                read_compressed(buf, mip_size, mip_count, block_size)?
            }
        };

        Ok(Self { format, mips })
    }
}

fn read_uncompressed(
    header: &Header,
    mut buf: &[u8],
    mut mip_size: UVec2,
    mip_count: u32,
) -> Result<Vec<MipLevel>, Error> {
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

        dbg!(size_in_bytes, header.pitch_or_linear_size);

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

        return Ok(mips);

        mip_size /= 2;
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

fn read_compressed(
    mut buf: &[u8],
    mut mip_size: UVec2,
    mip_count: u32,
    block_size: u32,
) -> Result<Vec<MipLevel>, Error> {
    let mut mips = Vec::new();

    for _ in 0..mip_count {
        let size =
            (u32::max(1, mip_size.x / 4) * u32::max(1, mip_size.y / 4) * block_size) as usize;

        if buf.remaining() < size {
            return Err(Error::Eof);
        }

        let bytes = buf.copy_to_bytes(size);
        mips.push(MipLevel {
            size: mip_size,
            bytes,
        });

        mip_size /= 2;
    }

    Ok(mips)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    /// 8-bit uncompressed RGBA.
    Rgba8,
    Rgb8,
    /// BC1
    Dxt1,
    /// BC2 with premultiplied alpha
    Dxt2,
    /// BC2
    Dxt3,
    /// BC3 with premultiplied alpha
    Dxt4,
    /// BC3
    Dxt5,
    /// BC5_UNORM
    Bc5U,
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

#[derive(Clone, Debug)]
pub struct MipLevel {
    pub size: UVec2,
    pub bytes: Bytes,
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
