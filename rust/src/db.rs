use std::ffi::CString;
use std::ptr::{null, null_mut};
use libc::{c_char, c_int};
use crate::xfile::{rust_xfile_close, rust_xfile_get_size, rust_xfile_open, xbase_open, XList};

#[repr(C)]
#[allow(dead_code)]
struct FileList {
    xlist: XList,
    next: *const FileList
}

#[no_mangle]
pub unsafe extern "C" fn rust_db_open(file_path1: *mut c_char, _a2: c_int, file_path2: *mut c_char, _a4: c_int) -> c_int {
    if file_path1 != null_mut() {
        if !xbase_open(file_path1) {
            return -1;
        }
    }

    if file_path2 != null_mut() {
        xbase_open(file_path2);
    }

    0
}

// TODO: sizePtr should be long*.
#[no_mangle]
pub unsafe extern "C" fn rust_db_get_file_size(file_path: *const c_char, size_ptr: *mut c_int) -> c_int {
    assert_ne!(file_path, null()); // "filename", "db.c", 108
    assert_ne!(size_ptr, null_mut()); // "de", "db.c", 109

    let rb = CString::new("rb").expect("valid string");
    let stream = rust_xfile_open(file_path, rb.as_ptr());
    if stream == null_mut() {
        return -1;
    }

    *size_ptr = rust_xfile_get_size(stream) as c_int;

    rust_xfile_close(stream);

    0
}
/*
int dbGetFileSize(const char* filePath, int* sizePtr)
{

}
 */
