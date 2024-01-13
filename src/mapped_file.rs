use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;

extern "C" {
    fn mmap64(
        addr: *mut std::ffi::c_void,
        length: usize,
        prot: std::ffi::c_int,
        flags: std::ffi::c_int,
        fd: std::ffi::c_int,
        offset: i64,
    ) -> *mut std::ffi::c_void;
    // fn munmap(addr: *mut std::ffi::c_void, length: usize) -> std::ffi::c_int;
}

pub struct MemoryMappedFile {
    addr: *mut std::ffi::c_void,
    length: usize,
}

impl MemoryMappedFile {
    const PROT_READ: std::ffi::c_int = 0x1;
    const MAP_FILE: std::ffi::c_int = 0x0;
    const MAP_PRIVATE: std::ffi::c_int = 0x2;
    const MAP_NONBLOCK: std::ffi::c_int = 0x10000;
    const MAP_FAILED: *mut std::ffi::c_void = !0 as *mut std::ffi::c_void;

    pub fn new(path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let meta = file.metadata()?;
        let size = meta.size();

        Self::new_with_file_size_offset(file, size as usize, 0)
    }

    pub fn new_with_file_size_offset(
        file: std::fs::File,
        size: usize,
        offset: i64,
    ) -> std::io::Result<Self> {
        let addr = unsafe {
            mmap64(
                std::ptr::null_mut(),                                    // addr
                size,                                                    // length
                Self::PROT_READ,                                         // prot
                Self::MAP_PRIVATE | Self::MAP_NONBLOCK | Self::MAP_FILE, // flags
                file.as_raw_fd(),                                        // fd
                offset,                                                  // offset
            )
        };

        if addr.is_null() || addr == Self::MAP_FAILED {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Self { addr, length: size })
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }
}

impl Drop for MemoryMappedFile {
    fn drop(&mut self) {
        // What if we don't unmap? :shrug: I mean the OS will do it for us...
        // eprintln!("Dropping MemoryMappedFile {:p}", self.addr);
        // unsafe {
        //     munmap(self.addr, self.length);
        // }
    }
}

impl core::ops::Deref for MemoryMappedFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.addr as *const u8, self.length) }
    }
}

// This is fine.. :shrug:
unsafe impl Send for MemoryMappedFile {}
