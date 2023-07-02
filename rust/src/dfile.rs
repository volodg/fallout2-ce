use crate::platform_compat::rust_compat_stricmp;
use libc::{c_char, c_int, c_uint};
use std::ffi::c_void;

#[repr(C)]
struct DBaseEntry {
    path: *const c_char,
    compressed: c_uint,
    uncompressed_size: c_int,
    data_size: c_int,
    data_offset: c_int,
}

// The [bsearch] comparison callback, which is used to find [DBaseEntry] for
// specified [filePath].
//
// 0x4E5D70
#[no_mangle]
pub unsafe extern "C" fn rust_dbase_find_entry_my_file_path(
    a1: *const c_void,
    a2: *const c_void,
) -> c_int {
    let file_path = a1 as *const c_char;
    let entry = a2 as *const DBaseEntry;

    return rust_compat_stricmp(file_path, (*entry).path);
}
