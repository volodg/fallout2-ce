use std::ffi::{c_uint, c_void, CString};
use std::mem;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};
use libc::{c_char, c_int, c_long, c_short, c_uchar, c_ushort, free, malloc, memmove, memset, qsort, size_t, snprintf, strchr, strlen};
use crate::platform_compat::{COMPAT_MAX_DIR, COMPAT_MAX_EXT, COMPAT_MAX_FNAME, COMPAT_MAX_PATH, rust_compat_splitpath, rust_compat_strdup, rust_compat_stricmp, rust_compat_windows_path_to_native};
use crate::xfile::{rust_xfile_close, rust_xfile_get_size, rust_xfile_open, xfile_read, xfile_read_char, xbase_open, XFile, XList, xfile_read_string, xfile_write_char, rust_xlist_init};

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
    next: *const FileList,
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
            let bytes_read = xfile_read(byte_buffer, mem::size_of_val(&byte_buffer), chunk_size as size_t, stream);
            byte_buffer = byte_buffer.offset(bytes_read as isize);
            remaining_size -= bytes_read as c_long;

            set_g_file_read_progress_bytes_read(0);
            get_g_file_read_progress_handler()();

            chunk_size = get_g_file_read_progress_chunk_size();
        }

        if remaining_size != 0 {
            let file_read_progress_bytes_read = get_g_file_read_progress_bytes_read();
            let read_size = xfile_read(byte_buffer, mem::size_of_val(&byte_buffer), remaining_size as size_t, stream);
            set_g_file_read_progress_bytes_read(file_read_progress_bytes_read + read_size as c_int);
        }
    } else {
        xfile_read(ptr, 1, size as size_t, stream);
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
            let bytes_read = xfile_read(byte_buffer, 1, chunk_size as size_t, stream);
            byte_buffer = byte_buffer.offset(bytes_read as isize);
            total_bytes_read += bytes_read;
            remaining_size -= bytes_read;

            set_g_file_read_progress_bytes_read(0);
            get_g_file_read_progress_handler()();

            chunk_size = get_g_file_read_progress_chunk_size();
        }

        if remaining_size != 0 {
            let bytes_read = xfile_read(byte_buffer, 1, remaining_size, stream);
            set_g_file_read_progress_bytes_read(get_g_file_read_progress_bytes_read() + bytes_read as c_int);
            total_bytes_read += bytes_read;
        }

        return total_bytes_read / size;
    }

    xfile_read(ptr, size, count, stream)
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

    if xfile_read(value.as_mut_ptr() as *mut c_void, mem::size_of_val(&value), 1, stream) == 0 {
        return -1;
    }

    let part1 = (value[0] as c_uint & 0xFF000000) >> 24;
    let part2 = (value[0] as c_uint & 0xFF0000) >> 8;
    let part3 = (value[0] as c_uint & 0xFF00) << 8;
    let part4 = (value[0] as c_uint & 0xFF) << 24;
    *value_ptr = (part1 | part2 | part3 | part4) as c_int;

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read_bool(stream: *const XFile, value_ptr: *mut bool) -> c_int {
    let mut value = 0;
    if rust_file_read_int32(stream, &mut value) == -1 {
        return -1;
    }

    *value_ptr = value != 0;

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_write_uint8(stream: *const XFile, value: c_short) -> c_int {
    xfile_write_char(value as c_int, stream)
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_write_int16(stream: *const XFile, value: c_short) -> c_int {
    // NOTE: Uninline.
    if rust_file_write_uint8(stream, (value >> 8) & 0xFF) == -1 {
        return -1;
    }

    // NOTE: Uninline.
    if rust_file_write_uint8(stream, value & 0xFF) == -1 {
        return -1;
    }

    0
}

// NOTE: Can either be signed vs. unsigned variant of [fileWriteInt32],
// or int vs. long.
//
// 0x4C6244
#[no_mangle]
pub unsafe extern "C" fn rust_db_fwrite_long(stream: *const XFile, value: c_int) -> c_int {
    if rust_file_write_int16(stream, ((value >> 16) as c_ushort & 0xFFFF) as c_short) == -1 {
        return -1;
    }

    if rust_file_write_int16(stream, ((value as c_ushort) & 0xFFFF) as c_short) == -1 {
        return -1;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read_uint8_list(stream: *const XFile, arr: *mut c_uchar, count: c_int) -> c_int {
    for index in 0..count {
        let mut ch = 0;
        // NOTE: Uninline.
        if rust_file_read_uint8(stream, &mut ch) == -1 {
            return -1;
        }

        *arr.offset(index as isize) = ch;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read_int16_list(stream: *const XFile, arr: *mut c_short, count: c_int) -> c_int {
    for index in 0..count {
        let mut ch = 0;
        // NOTE: Uninline.
        if rust_file_read_int16(stream, &mut ch) == -1 {
            return -1;
        }

        *arr.offset(index as isize) = ch;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_read_int32_list(stream: *const XFile, arr: *mut c_int, count: c_int) -> c_int {
    if count == 0 {
        return 0;
    }

    if rust_file_read(arr as *mut c_void, mem::size_of_val(&*arr) * count as size_t, 1, stream) < 1 {
        return -1;
    }

    for index in 0..count {
        let value = *arr.offset(index as isize);
        let part1 = (value as c_uint & 0xFF000000) >> 24;
        let part2 = (value as c_uint & 0xFF0000) >> 8;
        let part3 = (value as c_uint & 0xFF00) << 8;
        let part4 = (value as c_uint & 0xFF) << 24;
        *arr.offset(index as isize) = (part1 | part2 | part3 | part4) as c_int;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_write_uint8_list(stream: *const XFile, arr: *mut c_uchar, count: c_int) -> c_int {
    for index in 0..count {
        // NOTE: Uninline.
        if rust_file_write_uint8(stream, *arr.offset(index as isize) as c_short) == -1 {
            return -1;
        }
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_write_int16_list(stream: *const XFile, arr: *mut c_short, count: c_int) -> c_int {
    for index in 0..count {
        // NOTE: Uninline.
        if rust_file_write_int16(stream, *arr.offset(index as isize) as c_short) == -1 {
            return -1;
        }
    }

    0
}

// NOTE: Can be either signed/unsigned + int/long variant.
//
// 0x4C64F8
#[no_mangle]
pub unsafe extern "C" fn rust_file_write_int32_list(stream: *const XFile, arr: *mut c_int, count: c_int) -> c_int {
    for index in 0..count {
        // NOTE: Uninline.
        if rust_db_fwrite_long(stream, *arr.offset(index as isize) as c_int) == -1 {
            return -1;
        }
    }

    0
}

// NOTE: Not sure about signed/unsigned int/long.
//
// 0x4C6550
#[no_mangle]
pub unsafe extern "C" fn rust_db_fwrite_long_count(stream: *const XFile, arr: *mut c_int, count: c_int) -> c_int {
    for index in 0..count {
        let value = *arr.offset(index as isize);

        // NOTE: Uninline.
        if rust_file_write_int16(stream, ((value >> 16) & 0xFFFF) as c_short) == -1 {
            return -1;
        }

        // NOTE: Uninline.
        if rust_file_write_int16(stream, (value & 0xFFFF) as c_short) == -1 {
            return -1;
        }
    }

    0
}

unsafe extern "C" fn db_list_compare(p1: *const c_void, p2: *const c_void) -> c_int {
    return rust_compat_stricmp(p1 as *const c_char, p2 as *const c_char);
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_name_list_init(pattern: *const c_char, file_name_list_ptr: *mut *mut *mut c_char, _a3: c_int, _a4: c_int) -> c_int {
    let file_list = malloc(mem::size_of::<FileList>()) as *mut FileList;

    if file_list == null_mut() {
        return 0;
    }

    memset(file_list as *mut c_void, 0, mem::size_of::<FileList>());

    let xlist = &mut (*file_list).xlist;
    if !rust_xlist_init(pattern, xlist) {
        free(file_list as *mut c_void);
        return 0;
    }

    let mut length = 0;
    if (*xlist).file_names_length != 0 {
        qsort((*xlist).file_names as *mut c_void, (*xlist).file_names_length as size_t, mem::size_of_val(&*(*xlist).file_names), Some(db_list_compare));

        let mut file_names_length = (*xlist).file_names_length;
        let mut index = 0;
        while index < file_names_length - 1 {
            if rust_compat_stricmp((*xlist).file_names.offset(index as isize) as *const c_char, (*xlist).file_names.offset(index as isize + 1) as *const c_char) == 0 {
                let temp = *(*xlist).file_names.offset(index as isize + 1);
                memmove((*xlist).file_names.offset(index as isize + 1) as *mut c_void,
                        (*xlist).file_names.offset(index as isize + 2) as *mut c_void,
                        mem::size_of_val(&*(*xlist).file_names) * ((*xlist).file_names_length - index - 1) as usize);
                *(*xlist).file_names.offset((*xlist).file_names_length as isize - 1) = temp;

                file_names_length -= 1;
            } else {
                index += 1;
            }
        }

        let is_wildcard = *pattern == '*' as c_char;

        let sformat_sformat = CString::new("%s%s").expect("valid string");
        for index in 0..file_names_length {
            let name = *(*xlist).file_names.offset(index as isize);
            let mut dir = [0 as c_char; COMPAT_MAX_DIR as usize];
            let mut file_name = [0 as c_char; COMPAT_MAX_FNAME as usize];
            let mut extension = [0 as c_char; COMPAT_MAX_EXT as usize];
            rust_compat_windows_path_to_native(name);
            rust_compat_splitpath(name, null_mut(), dir.as_mut_ptr(), file_name.as_mut_ptr(), extension.as_mut_ptr());

            if !is_wildcard || dir[0] == '\0' as c_char || (strchr(dir.as_ptr(), '\\' as c_int) == null_mut() && strchr(dir.as_ptr(), '/' as c_int) == null_mut()) {
                // NOTE: Quick and dirty fix to buffer overflow. See RE to
                // understand the problem.
                let mut path = [0 as c_char; COMPAT_MAX_PATH];
                snprintf(path.as_mut_ptr(), mem::size_of_val(&path), sformat_sformat.as_ptr(), file_name, extension);
                free(*(*xlist).file_names.offset(length as isize) as *mut c_void);
                *(*xlist).file_names.offset(length as isize) = rust_compat_strdup(path.as_ptr());
                length += 1;
            }
        }
    }

    (*file_list).next = rust_g_get_file_list_head();
    rust_g_set_file_list_head(file_list);

    *file_name_list_ptr = (*xlist).file_names;

    length
}
/*
int fileNameListInit(const char* pattern, char*** fileNameListPtr, int a3, int a4)
{
    return length;
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