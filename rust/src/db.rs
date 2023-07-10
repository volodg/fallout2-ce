use std::ffi::{c_void, CString};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicPtr, Ordering};
use libc::{c_char, c_int};
use crate::xfile::{rust_xfile_close, rust_xfile_get_size, rust_xfile_open, xbase_open, XList};

type FileReadProgressHandler = unsafe extern "C" fn();

// Generic file progress report handler.
//
// 0x51DEEC
static G_FILE_READ_PROGRESS_HANDLER: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

#[no_mangle]
pub unsafe extern "C" fn rust_get_g_file_read_progress_handler() -> FileReadProgressHandler {
    let result = G_FILE_READ_PROGRESS_HANDLER.load(Ordering::Relaxed);
    std::mem::transmute(result)
}

#[no_mangle]
pub unsafe extern "C" fn rust_set_g_file_read_progress_handler(value: FileReadProgressHandler) {
    G_FILE_READ_PROGRESS_HANDLER.store(std::mem::transmute(value), Ordering::Relaxed)
}

/*

// Bytes read so far while tracking progress.
//
// Once this value reaches [gFileReadProgressChunkSize] the handler is called
// and this value resets to zero.
//
// 0x51DEF0
static int gFileReadProgressBytesRead = 0;

// The number of bytes to read between calls to progress handler.
//
// 0x673040
static int gFileReadProgressChunkSize;

// 0x673044
static FileList* gFileListHead;
 */

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_get_g_file_read_progress_handler() {
        let pointer = unsafe { rust_get_g_file_read_progress_handler() };
        assert_eq!(pointer as *const c_void, null())
    }
}