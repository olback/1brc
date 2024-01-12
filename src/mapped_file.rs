use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;

extern "C" {
    fn mmap(
        addr: *mut std::ffi::c_void,
        length: usize,
        prot: std::ffi::c_int,
        flags: std::ffi::c_int,
        fd: std::ffi::c_int,
        offset: std::ffi::c_int,
    ) -> *mut std::ffi::c_void;
    fn munmap(addr: *mut std::ffi::c_void, length: usize) -> std::ffi::c_int;
}

pub struct MemoryMappedFile {
    addr: *mut std::ffi::c_void,
    length: usize,
    // Keep file handle to prevent it from being closed
    _file: std::fs::File,
}

impl MemoryMappedFile {
    const PROT_READ: std::ffi::c_int = 0x1;
    const MAP_PRIVATE: std::ffi::c_int = 0x2;

    pub fn new(path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let meta = file.metadata()?;
        let size = meta.size();

        let addr = unsafe {
            mmap(
                std::ptr::null_mut(), // addr
                size as usize,        // length
                Self::PROT_READ,      // prot
                Self::MAP_PRIVATE,    // flags
                file.as_raw_fd(),     // fd
                0,                    // offset
            )
        };

        if addr.is_null() {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Self {
                addr,
                length: size as usize,
                _file: file,
            })
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }
}

impl Drop for MemoryMappedFile {
    fn drop(&mut self) {
        unsafe {
            munmap(self.addr, self.length);
        }
    }
}

impl core::ops::Deref for MemoryMappedFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.addr as *const u8, self.length) }
    }
}
