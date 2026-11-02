use std::ffi::c_void;
use std::io;

use bitflags::bitflags;

/// An anonymus memory file.
#[derive(Debug)]
pub struct Memfd {
    fd: imp::Fd,
}

impl Memfd {
    /// Creates a new file with the given size.
    pub fn new(size: usize) -> Result<Self, io::Error> {
        let fd = imp::memfd_create(size)?;
        Ok(Self { fd })
    }

    pub unsafe fn map(&self, size: usize, prot: ProtFlags) -> Result<*mut c_void, io::Error> {
        unsafe { imp::mmap(&self.fd, size, prot) }
    }

    pub unsafe fn unmap(&self, ptr: *mut c_void, len: usize) -> Result<(), io::Error> {
        unsafe { imp::unmap(ptr, len) }
    }
}

impl Drop for Memfd {
    fn drop(&mut self) {
        // Ignore possible error here, we cannot handle it.
        unsafe {
            imp::memfd_close(&self.fd).ok();
        }
    }
}

bitflags! {
    #[derive(Copy,Clone,Debug,PartialEq,Eq,Hash)]
    pub struct ProtFlags: u8 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXEC = 1 << 2;
    }
}

#[cfg(unix)]
mod imp {
    use std::ffi::c_void;
    use std::io::{Error, ErrorKind};
    use std::os::fd::OwnedFd;
    use std::ptr::null_mut;

    use rustix::fs::MemfdFlags;
    use rustix::mm::MapFlags;

    use super::ProtFlags;

    pub type Fd = OwnedFd;

    pub fn memfd_create(size: usize) -> Result<Fd, Error> {
        let Ok(size) = u64::try_from(size) else {
            return Err(Error::new(
                ErrorKind::FileTooLarge,
                "maximum file size is u64::MAX",
            ));
        };

        let fd = rustix::fs::memfd_create("", MemfdFlags::CLOEXEC)?;
        rustix::fs::ftruncate(&fd, size)?;
        Ok(fd)
    }

    pub unsafe fn memfd_close(_fd: &Fd) -> Result<(), Error> {
        Ok(())
    }

    pub unsafe fn mmap(fd: &Fd, size: usize, prot: ProtFlags) -> Result<*mut c_void, Error> {
        let mut flags = rustix::mm::ProtFlags::empty();
        if prot.contains(ProtFlags::READ) {
            flags |= rustix::mm::ProtFlags::READ;
        }
        if prot.contains(ProtFlags::WRITE) {
            flags |= rustix::mm::ProtFlags::WRITE;
        }
        if prot.contains(ProtFlags::EXEC) {
            flags |= rustix::mm::ProtFlags::EXEC;
        }

        let ptr = unsafe { rustix::mm::mmap(null_mut(), size, flags, MapFlags::SHARED, fd, 0)? };
        Ok(ptr)
    }

    pub unsafe fn unmap(ptr: *mut c_void, len: usize) -> Result<(), Error> {
        unsafe {
            rustix::mm::munmap(ptr, len)?;
        }

        Ok(())
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::io::{Error, ErrorKind};
    use std::ptr::null;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Memory::{
        CreateFileMappingW, FILE_MAP_EXECUTE, FILE_MAP_READ, FILE_MAP_WRITE,
        MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, PAGE_EXECUTE_READWRITE, UnmapViewOfFile,
    };

    use super::ProtFlags;

    pub type Fd = HANDLE;

    pub fn memfd_create(size: usize) -> Result<Fd, Error> {
        let Ok(size) = u32::try_from(size) else {
            return Err(Error::new(
                ErrorKind::FileTooLarge,
                "maximum file size is u32::MAX",
            ));
        };

        let handle = unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                null(),
                PAGE_EXECUTE_READWRITE,
                0,
                size,
                null(),
            )
        };

        if handle.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(handle)
        }
    }

    pub unsafe fn memfd_close(fd: &Fd) -> Result<(), Error> {
        // Failure on zero value
        if unsafe { CloseHandle(*fd) } == 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    pub unsafe fn mmap(fd: &Fd, size: usize, prot: ProtFlags) -> Result<*mut c_void, Error> {
        let mut access = 0;
        if prot.contains(ProtFlags::READ) {
            access |= FILE_MAP_READ;
        }
        if prot.contains(ProtFlags::WRITE) {
            access |= FILE_MAP_WRITE;
        }
        if prot.contains(ProtFlags::EXEC) {
            access |= FILE_MAP_EXECUTE;
        }

        let ptr = unsafe { MapViewOfFile(*fd, access, 0, 0, size).Value };

        if ptr.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(ptr)
        }
    }

    pub unsafe fn unmap(ptr: *mut c_void, _len: usize) -> Result<(), Error> {
        let addr = MEMORY_MAPPED_VIEW_ADDRESS { Value: ptr };
        // Failure on zero value.
        if unsafe { UnmapViewOfFile(addr) } == 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::memfd::{Memfd, ProtFlags};

    #[test]
    fn memfd_rw_and_rx() {
        let memfd = Memfd::new(8).unwrap();

        unsafe {
            let rw = memfd.map(8, ProtFlags::READ | ProtFlags::WRITE).unwrap();
            let rx = memfd.map(8, ProtFlags::WRITE | ProtFlags::EXEC).unwrap();

            rw.cast::<[u8; 8]>().write([0, 1, 2, 3, 4, 5, 6, 7]);
            assert_eq!(rx.cast::<[u8; 8]>().read(), [0, 1, 2, 3, 4, 5, 6, 7]);

            memfd.unmap(rw, 8).unwrap();
            memfd.unmap(rx, 8).unwrap();
        }
    }
}
