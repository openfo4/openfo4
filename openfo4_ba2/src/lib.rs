mod dds;

use std::io::{self, Read, Seek, SeekFrom};

use flate2::read::ZlibDecoder;
use hashbrown::HashMap;
use thiserror::Error;

const MAGIC: u32 = u32::from_le_bytes([b'B', b'T', b'D', b'X']);

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("bad magic: {0:x}")]
    BadMagic(u32),
    #[error("unknown version: {0}")]
    UnknownVersion(u32),
    #[error("unknown data format: {0}")]
    UnknownDataFormat(u32),
    #[error(transparent)]
    InvalidString(std::string::FromUtf8Error),
    #[error("no key in archive")]
    NoKey,
}

#[derive(Debug)]
pub struct Archive<T> {
    reader: T,
    files: HashMap<String, FileHeader>,
}

impl<T> Archive<T>
where
    T: Read + Seek,
{
    pub fn new(mut reader: T) -> Result<Self, Error> {
        let header = Header::decode(&mut reader)?;

        let mut files = Vec::new();
        for _ in 0..header.file_count {
            match header.format {
                Format::Gnrl => {
                    let file = GnrlFileHeader::decode(&mut reader)?;
                    files.push(FileHeader::Gnrl(file));
                }
                Format::Dx10 => {
                    let file = Dx10FileHeader::decode(&mut reader)?;
                    files.push(FileHeader::Dx10(file));
                }
            }
        }

        reader.seek(SeekFrom::Start(header.strings_offset.into()))?;

        let mut file_names = Vec::new();
        for _ in 0..header.file_count {
            let name = String::decode(&mut reader)?;
            let name = name.to_lowercase().replace("\\", "/");

            file_names.push(name);
        }

        let files = file_names.into_iter().zip(files).collect();

        Ok(Self { reader, files })
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(|v| v.as_str())
    }

    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    pub fn read(&mut self, name: &str) -> Result<Vec<u8>, Error> {
        let header = self.files.get(name).ok_or(Error::NoKey)?;

        let mut buf = Vec::new();

        let mut bytes = Vec::new();

        let chunks = match header {
            FileHeader::Gnrl(header) => &header.chunks,
            FileHeader::Dx10(header) => {
                header.dds_header(&mut bytes);
                &header.chunks
            }
        };

        for chunk in chunks {
            self.reader.seek(SeekFrom::Start(chunk.offset))?;

            buf.clear();
            buf.resize(chunk.compressed_len as usize, 0);
            self.reader.read_exact(&mut buf)?;

            let mut decoder = ZlibDecoder::new(&buf[..]);
            decoder.read_to_end(&mut bytes)?;
        }

        Ok(bytes)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Version {
    V1,
    V7,
    V8,
}

impl Decode for Version {
    fn decode<R>(mut reader: R) -> Result<Self, Error>
    where
        R: Read,
    {
        let value = u32::decode(&mut reader)?;
        match value {
            1 => Ok(Self::V1),
            7 => Ok(Self::V7),
            8 => Ok(Self::V8),
            _ => Err(Error::UnknownVersion(value)),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Header {
    version: Version,
    format: Format,
    file_count: u32,
    strings_offset: u32,
}

impl Decode for Header {
    fn decode<R>(mut reader: R) -> Result<Self, Error>
    where
        R: Read,
    {
        let magic = u32::decode(&mut reader)?;
        if magic != MAGIC {
            return Err(Error::BadMagic(magic));
        }

        let version = Version::decode(&mut reader)?;
        let format = Format::decode(&mut reader)?;

        let file_count = u32::decode(&mut reader)?;
        let strings_offset = u32::decode(&mut reader)?;

        u32::decode(&mut reader)?;

        Ok(Self {
            version,
            format,
            file_count,
            strings_offset,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Format {
    Gnrl,
    Dx10,
}

impl Decode for Format {
    fn decode<R>(reader: R) -> Result<Self, Error>
    where
        R: Read,
    {
        let value = u32::decode(reader)?;
        match &value.to_le_bytes() {
            b"GNRL" => Ok(Self::Gnrl),
            b"DX10" => Ok(Self::Dx10),
            _ => Err(Error::UnknownDataFormat(value)),
        }
    }
}

trait Decode: Sized {
    fn decode<R>(reader: R) -> Result<Self, Error>
    where
        R: Read;
}

macro_rules! impl_decode {
    ($($t:ty),*) => {
        $(
            impl Decode for $t {
                fn decode<R>(mut reader: R) -> Result<Self, Error>
                where
                    R: Read,
                {
                    let mut buf = [0; size_of::<Self>()];
                    reader.read_exact(&mut buf)?;
                    Ok(Self::from_le_bytes(buf))
                }
            }
        )*
    };
}

impl_decode!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl Decode for String {
    fn decode<R>(mut reader: R) -> Result<Self, Error>
    where
        R: Read,
    {
        let len = u16::decode(&mut reader)?;

        let mut bytes = vec![0; len.into()];
        reader.read_exact(&mut bytes)?;

        Self::from_utf8(bytes).map_err(Error::InvalidString)
    }
}

#[derive(Clone, Debug)]
enum FileHeader {
    Gnrl(GnrlFileHeader),
    Dx10(Dx10FileHeader),
}

#[derive(Clone, Debug)]
struct Dx10FileHeader {
    file_hash: u32,
    extension: u32,
    directory_hash: u32,
    chunks: Vec<Chunk>,
    width: u16,
    height: u16,
    mip_levels: u8,
    format: u8,
}

impl Dx10FileHeader {
    fn dds_header(&self, dst: &mut Vec<u8>) {
        // https://learn.microsoft.com/en-us/windows/win32/direct3ddds/dds-header
        //
        // Magic
        dst.extend_from_slice(&0x20534444_u32.to_le_bytes());
        // DDS_HEADER size
        dst.extend_from_slice(&124_u32.to_le_bytes());
        // dwFlags
        dst.extend_from_slice(
            &(dds::Flags::CAPS
                | dds::Flags::HEIGHT
                | dds::Flags::WIDTH
                | dds::Flags::PIXELFORMAT
                | dds::Flags::MIPMAP_COUNT)
                .bits()
                .to_le_bytes(),
        );
        // dwHeight
        dst.extend_from_slice(&(self.height as u32).to_le_bytes());
        // dwWidth
        dst.extend_from_slice(&(self.width as u32).to_le_bytes());
        // dwPitchOrLinearSize
        dst.extend_from_slice(&0_u32.to_le_bytes());
        // dwDepth
        dst.extend_from_slice(&1_u32.to_le_bytes());
        // dwMipMapCount
        dst.extend_from_slice(&(self.mip_levels as u32).to_le_bytes());

        // dwReserved1[11]
        for _ in 0..11 {
            dst.extend_from_slice(&0_u32.to_le_bytes());
        }

        // ddspf
        // dwSize
        dst.extend_from_slice(&32_u32.to_le_bytes());
        // dwFlags
        dst.extend_from_slice(&0x4_u32.to_le_bytes());
        // dwFourCC
        dst.extend_from_slice(&[b'D', b'X', b'1', b'0']);
        // dwRGBBitCount
        dst.extend_from_slice(&0_u32.to_le_bytes());
        // dwRBitMask
        dst.extend_from_slice(&0_u32.to_le_bytes());
        // dwGBitMask
        dst.extend_from_slice(&0_u32.to_le_bytes());
        // dwBBitMask
        dst.extend_from_slice(&0_u32.to_le_bytes());
        // dwABitMask
        dst.extend_from_slice(&0_u32.to_le_bytes());

        // dwCaps
        dst.extend_from_slice(
            &(dds::Caps::COMPLEX | dds::Caps::MIPMAP | dds::Caps::TEXTURE)
                .bits()
                .to_le_bytes(),
        );

        // dwCaps2
        dst.extend_from_slice(&0_u32.to_le_bytes());
        // dwCaps3
        dst.extend_from_slice(&0_u32.to_le_bytes());
        // dwCaps4
        dst.extend_from_slice(&0_u32.to_le_bytes());
        // dwReserved2
        dst.extend_from_slice(&0_u32.to_le_bytes());

        // DDS_HEADER_DXT10
        // dxgiFormat
        dst.extend_from_slice(&(self.format as u32).to_le_bytes());
        // resourceDimension
        dst.extend_from_slice(&3_u32.to_le_bytes());
        // miscFlag
        dst.extend_from_slice(&0_u32.to_le_bytes());
        // arraySize
        dst.extend_from_slice(&1_u32.to_le_bytes());
        // miscFlag2
        dst.extend_from_slice(&0_u32.to_le_bytes());
    }
}

impl Decode for Dx10FileHeader {
    fn decode<R>(mut reader: R) -> Result<Self, Error>
    where
        R: Read,
    {
        let file_hash = u32::decode(&mut reader)?;
        let extension = u32::decode(&mut reader)?;
        let directory_hash = u32::decode(&mut reader)?;

        u8::decode(&mut reader)?;
        let chunk_count = u8::decode(&mut reader)?;
        let chunk_size = u16::decode(&mut reader)?;

        let height = u16::decode(&mut reader)?;
        let width = u16::decode(&mut reader)?;
        let mip_count = u8::decode(&mut reader)?;
        let format = u8::decode(&mut reader)?;
        let flags = u8::decode(&mut reader)?;
        let tile_mode = u8::decode(&mut reader)?;

        let mut chunks = Vec::new();
        for _ in 0..chunk_count {
            let offset = u64::decode(&mut reader)?;

            let compressed_len = u32::decode(&mut reader)?;
            let decompressed_len = u32::decode(&mut reader)?;

            let first_mip = u16::decode(&mut reader)?;
            let last_mip = u16::decode(&mut reader)?;

            // Always 0x0df0 adba
            u32::decode(&mut reader)?;

            chunks.push(Chunk {
                offset,
                compressed_len,
                decompressed_len,
            });
        }

        Ok(Self {
            file_hash,
            extension,
            directory_hash,
            chunks,
            width,
            height,
            mip_levels: mip_count,
            format,
        })
    }
}

#[derive(Clone, Debug)]
struct GnrlFileHeader {
    file_hash: u32,
    extension: u32,
    directory_hash: u32,
    chunks: Vec<Chunk>,
}

impl Decode for GnrlFileHeader {
    fn decode<R>(mut reader: R) -> Result<Self, Error>
    where
        R: Read,
    {
        let file_hash = u32::decode(&mut reader)?;
        let extension = u32::decode(&mut reader)?;
        let directory_hash = u32::decode(&mut reader)?;

        u8::decode(&mut reader)?;
        let chunk_count = u8::decode(&mut reader)?;
        let chunk_size = u16::decode(&mut reader)?;

        let mut chunks = Vec::new();
        for _ in 0..chunk_count {
            let offset = u64::decode(&mut reader)?;
            let compressed_len = u32::decode(&mut reader)?;
            let decompressed_len = u32::decode(&mut reader)?;

            // Always 0x0df0 adba?
            u32::decode(&mut reader)?;

            chunks.push(Chunk {
                offset,
                compressed_len,
                decompressed_len,
            });
        }

        Ok(Self {
            file_hash,
            directory_hash,
            extension,
            chunks,
        })
    }
}

#[derive(Copy, Clone, Debug)]
struct Chunk {
    offset: u64,
    compressed_len: u32,
    decompressed_len: u32,
}
