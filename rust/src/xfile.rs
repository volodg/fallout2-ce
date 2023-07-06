use std::ffi::{c_int, c_void};
use std::mem;
use std::ptr::null_mut;
use libc::{fclose, FILE, free, memset};
use libz_sys::{gzclose, gzFile};
use crate::dfile::{DFile, rust_dfile_close};

#[repr(C)]
enum XFileType {
    #[allow(dead_code)]
    XfileTypeFile = 0,
    #[allow(dead_code)]
    XfileTypeDfile = 1,
    #[allow(dead_code)]
    XfileTypeGzfile = 2,
}

#[repr(C)]
union XFileTypeUnion {
    file: *mut FILE,
    dfile: *mut DFile,
    gzfile: gzFile,
}

#[repr(C)]
pub struct XFile {
    _type: XFileType,
    file: XFileTypeUnion,
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_close(stream: *mut XFile) -> c_int {
    assert_ne!(stream, null_mut()); // "stream", "xfile.c", 112

    let rc = match (*stream)._type {
        XFileType::XfileTypeDfile => rust_dfile_close((*stream).file.dfile),
        XFileType::XfileTypeGzfile => gzclose((*stream).file.gzfile),
        XFileType::XfileTypeFile => fclose((*stream).file.file),
    };

    memset(stream as *mut c_void, 0, mem::size_of_val(&*stream));

    free(stream as *mut c_void);

    rc
}
