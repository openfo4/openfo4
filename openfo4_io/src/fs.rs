use std::fmt::{self, Debug, Formatter};
use std::io::{self, Error, ErrorKind};
use std::path::Path;

pub struct File {
    #[cfg(unix)]
    inner: unix::File,
    #[cfg(not(unix))]
    inner: generic::File,
}

impl File {
    pub fn open<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            #[cfg(unix)]
            inner: unix::File::open(path.as_ref())?,
            #[cfg(not(unix))]
            inner: generic::File::open(path.as_ref())?,
        })
    }
}

impl AsyncReadAt for File {
    fn read_at(
        &self,
        buf: &mut [u8],
        offset: u64,
    ) -> impl Future<Output = io::Result<usize>> + Send {
        self.inner.read_at(buf, offset)
    }

    fn read_exact_at(
        &self,
        buf: &mut [u8],
        offset: u64,
    ) -> impl Future<Output = io::Result<()>> + Send {
        self.inner.read_at_exact(buf, offset)
    }
}

impl Debug for File {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("File").finish_non_exhaustive()
    }
}

pub trait AsyncReadAt: Send + Sync {
    fn read_at(
        &self,
        buf: &mut [u8],
        offset: u64,
    ) -> impl Future<Output = io::Result<usize>> + Send;

    fn read_exact_at(
        &self,
        mut buf: &mut [u8],
        mut offset: u64,
    ) -> impl Future<Output = io::Result<()>> + Send {
        async move {
            while !buf.is_empty() {
                match self.read_at(buf, offset).await {
                    Ok(0) => break,
                    Ok(n) => {
                        buf = &mut buf[n..];
                        offset += n as u64;
                    }
                    Err(e) => return Err(e),
                }
            }

            if !buf.is_empty() {
                Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer",
                ))
            } else {
                Ok(())
            }
        }
    }
}

impl<T> AsyncReadAt for &T
where
    T: AsyncReadAt,
{
    fn read_at(
        &self,
        buf: &mut [u8],
        offset: u64,
    ) -> impl Future<Output = io::Result<usize>> + Send {
        <T as AsyncReadAt>::read_at(self, buf, offset)
    }

    fn read_exact_at(
        &self,
        buf: &mut [u8],
        offset: u64,
    ) -> impl Future<Output = io::Result<()>> + Send {
        <T as AsyncReadAt>::read_exact_at(self, buf, offset)
    }
}

impl<T> AsyncReadAt for &mut T
where
    T: AsyncReadAt,
{
    fn read_at(
        &self,
        buf: &mut [u8],
        offset: u64,
    ) -> impl Future<Output = io::Result<usize>> + Send {
        <T as AsyncReadAt>::read_at(self, buf, offset)
    }

    fn read_exact_at(
        &self,
        buf: &mut [u8],
        offset: u64,
    ) -> impl Future<Output = io::Result<()>> + Send {
        <T as AsyncReadAt>::read_exact_at(self, buf, offset)
    }
}

/// A wrapper around a positionless reader that provides a position.
///
/// This may be preferred over a position that handled by the kernel because it allows multiple
/// [`Cursor`]s to exist backing the same file descriptor without interfering with each other.
///
/// [`Cursor`] maintains an internal buffer for short reads, akin to [`BufReader`].
///
/// [`BufReader`]: std::io::BufReader
#[derive(Clone, Debug)]
pub struct Cursor<T> {
    reader: T,
    buffer: Vec<u8>,
    position: u64,
}

impl<T> Cursor<T>
where
    T: AsyncReadAt,
{
    pub fn new(reader: T) -> Self {
        const DEFUALT_BUFFER_SIZE: usize = 8192;

        Self {
            reader,
            buffer: Vec::with_capacity(DEFUALT_BUFFER_SIZE),
            position: 0,
        }
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn set_position(&mut self, position: u64) {
        // Going backwards always invalidates the buffer.
        if position < self.position {
            self.buffer.clear();
        } else {
            let delta = (position - self.position) as usize;

            if self.buffer.len() < delta {
                self.buffer.clear();
            } else {
                self.buffer.copy_within(delta.., 0);
                self.buffer.truncate(self.buffer.len() - delta);
            }
        }

        self.position = position;
    }

    /// Advances the cursor by `count` bytes.
    ///
    /// This is equivalent to `set_position(position() + count)`.
    pub fn advance(&mut self, count: u64) {
        self.set_position(self.position() + count);
    }

    fn fill_from_buffer(&mut self, buf: &mut [u8]) -> usize {
        let bytes_copied = usize::min(self.buffer.len(), buf.len());
        buf[..bytes_copied].copy_from_slice(&self.buffer[..bytes_copied]);
        bytes_copied
    }

    /// Reads some bytes into the given buffer.
    ///
    /// Returns the number of bytes read. The position will be advanced by the number of bytes
    /// read. The position is unaffected if an error is returned.
    pub async fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let buffered_bytes_copied = self.fill_from_buffer(buf);
        buf = &mut buf[buffered_bytes_copied..];
        if buf.is_empty() {
            self.set_position(self.position + buffered_bytes_copied as u64);
            return Ok(buffered_bytes_copied);
        }

        // If the read is small attempt to fill the buffer with it.
        let mut bytes = vec![0; usize::max(buf.len(), self.buffer.capacity())];
        let bytes_read = self
            .reader
            .read_at(&mut bytes, self.position + buffered_bytes_copied as u64)
            .await?;

        bytes.truncate(bytes_read);

        let read_bytes_copied = usize::min(buf.len(), bytes.len());
        buf[..read_bytes_copied].copy_from_slice(&bytes[..read_bytes_copied]);

        debug_assert_eq!(buffered_bytes_copied, self.buffer.len());
        self.buffer.clear();
        self.buffer.extend(&bytes[read_bytes_copied..]);

        self.position += (buffered_bytes_copied + read_bytes_copied) as u64;
        Ok(buffered_bytes_copied + read_bytes_copied)
    }

    pub async fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        let prev_pos = self.position;

        while !buf.is_empty() {
            match self.read(buf).await {
                Ok(0) => break,
                Ok(n) => {
                    buf = &mut buf[n..];
                }
                Err(e) => {
                    // The position must be reset if the read_exact fails.
                    // Since other `read` calls may have succeeded the position
                    // may have been updated.
                    self.set_position(prev_pos);
                    return Err(e);
                }
            }
        }

        if !buf.is_empty() {
            Err(Error::new(
                ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(unix)]
mod unix {
    use std::io;
    use std::os::unix::fs::FileExt;
    use std::path::Path;
    use std::sync::Arc;

    use blocking::unblock;

    pub struct File {
        file: Arc<std::fs::File>,
    }

    impl File {
        pub fn open(path: &Path) -> io::Result<Self> {
            let file = std::fs::File::open(path)?;

            Ok(Self {
                file: Arc::new(file),
            })
        }

        pub async fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
            let mut bytes = vec![0; buf.len()];
            let file = self.file.clone();
            let (bytes, bytes_read) = unblock(move || {
                let res = file.read_at(&mut bytes, offset);
                res.map(|bytes_read| (bytes, bytes_read))
            })
            .await?;

            buf[..bytes_read].copy_from_slice(&bytes[..bytes_read]);
            Ok(bytes_read)
        }

        pub async fn read_at_exact(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
            let mut bytes = vec![0; buf.len()];
            let file = self.file.clone();
            let bytes = unblock(move || {
                let res = file.read_exact_at(&mut bytes, offset);
                res.map(|()| bytes)
            })
            .await?;

            buf.copy_from_slice(&bytes);
            Ok(())
        }
    }
}

mod generic {
    use std::io::{self, Read, Seek, SeekFrom};
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use blocking::unblock;

    pub struct File {
        file: Arc<Mutex<std::fs::File>>,
    }

    impl File {
        pub fn open(path: &Path) -> io::Result<Self> {
            let file = std::fs::File::open(path)?;

            Ok(Self {
                file: Arc::new(Mutex::new(file)),
            })
        }

        pub async fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
            let file = self.file.clone();

            let mut bytes = vec![0; buf.len()];
            let (bytes, bytes_read) = unblock(move || {
                // The seek => read sequence must happen atomically.
                let mut file = file.lock().unwrap();
                file.seek(SeekFrom::Start(offset))?;
                let bytes_read = file.read(&mut bytes)?;
                Ok::<_, io::Error>((bytes, bytes_read))
            })
            .await?;

            buf[..bytes_read].copy_from_slice(&bytes[..bytes_read]);
            Ok(bytes_read)
        }

        pub async fn read_at_exact(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
            let file = self.file.clone();

            let mut bytes = vec![0; buf.len()];
            let bytes = unblock(move || {
                // The seek => read sequence must happen atomically.
                let mut file = file.lock().unwrap();
                file.seek(SeekFrom::Start(offset))?;
                file.read_exact(&mut bytes)?;
                Ok::<_, io::Error>(bytes)
            })
            .await?;

            buf.copy_from_slice(&bytes);

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_lite::future::block_on;

    use crate::fs::{AsyncReadAt, Cursor};

    #[derive(Clone, Debug)]
    struct TestBuffer(Vec<u8>);

    impl AsyncReadAt for TestBuffer {
        async fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
            if offset as usize > self.0.len() {
                return Ok(0);
            }

            let bytes_written = self.0.len() - offset as usize;
            buf[..bytes_written]
                .copy_from_slice(&self.0[offset as usize..offset as usize + bytes_written]);
            Ok(bytes_written)
        }
    }

    #[test]
    fn cursor_read_buffered() {
        let reader = TestBuffer(vec![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ]);
        let mut cursor = Cursor::new(reader);

        for i in 0..8 {
            let mut buf = [0; 2];
            let bytes_read = block_on(cursor.read(&mut buf)).unwrap();
            assert_eq!(bytes_read, 2);
            assert_eq!(buf, [i * 2, i * 2 + 1]);
        }
    }

    #[test]
    fn cursor_seek_forward() {
        let reader = TestBuffer(vec![0, 1, 2, 3, 4, 5, 6, 7]);
        let mut cursor = Cursor::new(reader);

        let mut buf = [0; 2];
        let bytes_read = block_on(cursor.read(&mut buf)).unwrap();
        assert_eq!(bytes_read, 2);
        assert_eq!(buf, [0, 1]);

        cursor.set_position(7);

        let mut buf = [0; 1];
        let bytes_read = block_on(cursor.read(&mut buf)).unwrap();
        assert_eq!(bytes_read, 1);
        assert_eq!(buf, [7]);
    }
}
