use std::ffi::{c_uint, c_void, CString};
use std::mem;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};
use libc::{c_char, c_int, c_long, c_short, c_uchar, size_t, strlen};
use crate::xfile::{rust_xfile_close, rust_xfile_get_size, rust_xfile_open, rust_xfile_read, xfile_read_char, xbase_open, XFile, XList, xfile_read_string};

type FileReadProgressHandler = unsafe extern "C" fn();

// Generic file progress report handler.
//
// 0x51DEEC
static G_FILE_READ_PROGRESS_HANDLER: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

unsafe fn get_g_file_read_progress_handler() -> FileReadProgressHandler {
    let result = G_FILE_READ_PROGRESS_HANDLER.load(Ordering::Relaxed);
    mem::transmute(result)
}

#[no_mangle]
pub unsafe extern "C" fn rust_set_g_file_read_progress_handler(value: FileReadProgressHandler) {
    G_FILE_READ_PROGRESS_HANDLER.store(std::mem::transmute(value), Ordering::Relaxed)
}

// Bytes read so far while tracking progress.
//
// Once this value reaches [gFileReadProgressChunkSize] the handler is called
// and this value resets to zero.
//
// 0x51DEF0
static G_FILE_READ_PROGRESS_BYTES_READ: AtomicI32 = AtomicI32::new(0);

fn get_g_file_read_progress_bytes_read() -> c_int {
    G_FILE_READ_PROGRESS_BYTES_READ.load(Ordering::Relaxed)
}

fn set_g_file_read_progress_bytes_read(value: c_int) {
    G_FILE_READ_PROGRESS_BYTES_READ.store(value, Ordering::Relaxed)
}

// The number of bytes to read between calls to progress handler.
//
// 0x673040
static G_FILE_READ_PROGRESS_CHUNK_SIZE: AtomicI32 = AtomicI32::new(0);

fn get_g_file_read_progress_chunk_size() -> c_int {
    G_FILE_READ_PROGRESS_CHUNK_SIZE.load(Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn rust_set_g_file_read_progress_chunk_size(value: c_int) {
    G_FILE_READ_PROGRESS_CHUNK_SIZE.store(value, Ordering::Relaxed)
}

// 0x673044
static G_FILE_LIST_HEAD: AtomicPtr<FileList> = AtomicPtr::new(null_mut());

#[no_mangle]
pub unsafe extern "C" fn rust_g_get_file_list_head() -> *mut FileList {
    G_FILE_LIST_HEAD.load(Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn rust_g_set_file_list_head(value: *mut FileList) {
    G_FILE_LIST_HEAD.store(value, Ordering::Relaxed)
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
pub struct FileList {
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

#[no_mangle]
pub unsafe extern "C" fn rust_db_get_file_contents(file_path: *const c_char, ptr: *mut c_void) -> c_int {
    assert_ne!(file_path, null()); // "filename", "db.c", 141
    assert_ne!(ptr, null_mut()); // "buf", "db.c", 142

    let rb = CString::new("rb").expect("valid string");
    let stream = rust_xfile_open(file_path, rb.as_ptr());
    if stream == null_mut() {
        return -1;
    }

    let size = rust_xfile_get_size(stream);
    if mem::transmute::<unsafe extern "C" fn(), *const c_void>(get_g_file_read_progress_handler()) != null() {
        let mut byte_buffer = ptr;// as *mut c_uchar;

        let mut remaining_size = size;
        let mut chunk_size = get_g_file_read_progress_chunk_size() - get_g_file_read_progress_bytes_read();

        while remaining_size >= chunk_size as c_long {
            let bytes_read = rust_xfile_read(byte_buffer, mem::size_of_val(&byte_buffer), chunk_size as size_t, stream);
            byte_buffer = byte_buffer.offset(bytes_read as isize);
            remaining_size -= bytes_read as c_long;

            set_g_file_read_progress_bytes_read(0);
            get_g_file_read_progress_handler()();

            chunk_size = get_g_file_read_progress_chunk_size();
        }

        if remaining_size != 0 {
            let file_read_progress_bytes_read = get_g_file_read_progress_bytes_read();
            let read_size = rust_xfile_read(byte_buffer, mem::size_of_val(&byte_buffer), remaining_size as size_t, stream);
            set_g_file_read_progress_bytes_read(file_read_progress_bytes_read + read_size as c_int);
        }
    } else {
        rust_xfile_read(ptr, 1, size as size_t, stream);
    }

    rust_xfile_close(stream);

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read_char(stream: *const XFile) -> c_int {
    if mem::transmute::<unsafe extern "C" fn(), *const c_void>(get_g_file_read_progress_handler()) != null() {
        let ch = xfile_read_char(stream);

        set_g_file_read_progress_bytes_read(get_g_file_read_progress_bytes_read() + 1);
        if get_g_file_read_progress_bytes_read() >= get_g_file_read_progress_chunk_size() {
            get_g_file_read_progress_handler()();
            set_g_file_read_progress_bytes_read(0);
        }

        return ch;
    }

    xfile_read_char(stream)
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read_string(string: *mut c_char, size: size_t, stream: *const XFile) -> *const c_char {
    if mem::transmute::<unsafe extern "C" fn(), *const c_void>(get_g_file_read_progress_handler()) != null() {
        if xfile_read_string(string, size as c_int, stream) == null() {
            return null();
        }

        set_g_file_read_progress_bytes_read(get_g_file_read_progress_bytes_read() + strlen(string) as c_int);
        while get_g_file_read_progress_bytes_read() >= get_g_file_read_progress_chunk_size() {
            get_g_file_read_progress_handler()();
            set_g_file_read_progress_bytes_read(get_g_file_read_progress_bytes_read() - get_g_file_read_progress_chunk_size());
        }

        return string;
    }

    xfile_read_string(string, size as c_int, stream)
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read(ptr: *mut c_void, size: size_t, count: size_t, stream: *const XFile) -> size_t {
    if mem::transmute::<unsafe extern "C" fn(), *const c_void>(get_g_file_read_progress_handler()) != null() {
        let mut byte_buffer = ptr;

        let mut total_bytes_read = 0;
        let mut remaining_size = size * count;
        let mut chunk_size = get_g_file_read_progress_chunk_size() - get_g_file_read_progress_bytes_read();

        while remaining_size >= chunk_size as size_t {
            let bytes_read = rust_xfile_read(byte_buffer, 1, chunk_size as size_t, stream);
            byte_buffer = byte_buffer.offset(bytes_read as isize);
            total_bytes_read += bytes_read;
            remaining_size -= bytes_read;

            set_g_file_read_progress_bytes_read(0);
            get_g_file_read_progress_handler()();

            chunk_size = get_g_file_read_progress_chunk_size();
        }

        if remaining_size != 0 {
            let bytes_read = rust_xfile_read(byte_buffer, 1, remaining_size, stream);
            set_g_file_read_progress_bytes_read(get_g_file_read_progress_bytes_read() + bytes_read as c_int);
            total_bytes_read += bytes_read;
        }

        return total_bytes_read / size;
    }

    rust_xfile_read(ptr, size, count, stream)
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read_uint8(stream: *const XFile, value_ptr: *mut c_uchar) -> c_int {
    let value = rust_file_read_char(stream);
    if value == -1 {
        return -1;
    }

    *value_ptr = (value & 0xFF) as c_uchar;

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read_int16(stream: *const XFile, value_ptr: *mut c_short) -> c_int {
    let mut high = 0;
    // NOTE: Uninline.
    if rust_file_read_uint8(stream, &mut high) == -1 {
        return -1;
    }

    let mut low = 0;
    // NOTE: Uninline.
    if rust_file_read_uint8(stream, &mut low) == -1 {
        return -1;
    }

    *value_ptr = (((high as c_short) << 8) | low as c_short) as c_short;

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read_int32(stream: *const XFile, value_ptr: *mut c_int) -> c_int {
    let mut value = [0 as c_int; 1];

    if rust_xfile_read(value.as_mut_ptr() as *mut c_void, mem::size_of_val(&value), 1, stream) == 0 {
        return -1;
    }

    let part1 = (value[0] as c_uint & 0xFF000000) >> 24;
    let part2 = (value[0] as c_uint & 0xFF0000) >> 8;
    let part3 = (value[0] as c_uint & 0xFF00) << 8;
    let part4 = (value[0] as c_uint & 0xFF) << 24;
    *value_ptr = (part1 | part2 | part3 | part4) as c_int;

    0
}
/*
int fileReadInt32(File* stream, int* valuePtr)
{

    return 0;
}
 */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_get_g_file_read_progress_handler() {
        let pointer = unsafe { get_g_file_read_progress_handler() };
        assert_eq!(pointer as *const c_void, null())
    }
}