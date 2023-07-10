use std::ptr::null_mut;
use libc::{c_char, c_int};
use crate::xfile::{xbase_open, XList};

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
/*
int dbOpen(const char* filePath1, int a2, const char* filePath2, int a4)
{
}
 */
