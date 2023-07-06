use crate::platform_compat::{COMPAT_MAX_PATH, rust_compat_fopen, rust_compat_strdup, rust_compat_stricmp, rust_get_file_size};
#[cfg(not(target_family = "windows"))]
use libc::bsearch;
use libc::{c_char, c_int, c_long, c_uchar, c_uint, fclose, fgetc, FILE, fread, free, fseek, malloc, memset, SEEK_CUR, SEEK_END, SEEK_SET, size_t, strcpy, ungetc};
use std::ffi::{c_void, CString};
use std::mem;
use std::ptr::{null, null_mut};
use libz_sys::{alloc_func, Bytef, free_func, inflate, inflateEnd, inflateInit_, voidpf, Z_NO_FLUSH, Z_OK, z_stream, z_streamp};
use crate::fpattern::fpattern_match;

// The size of decompression buffer for reading compressed [DFile]s.
const DFILE_DECOMPRESSION_BUFFER_SIZE: u32 = 0x400;

// Specifies that [DFile] has unget compressed character.
const DFILE_HAS_COMPRESSED_UNGETC: c_int = 0x10;

// Specifies that [DFile] has unget character.
//
// NOTE: There is an unused function at 0x4E5894 which ungets one character and
// stores it in [ungotten]. Since that function is not used, this flag will
// never be set.
const DFILE_HAS_UNGETC: u32 = 0x01;

// Specifies that [DFile] has reached end of stream.
const DFILE_EOF: u32 = 0x02;

// Specifies that [DFile] is in error state.
//
// [dfileRewind] can be used to clear this flag.
const DFILE_ERROR: u32 = 0x04;

// Specifies that [DFile] was opened in text mode.
const DFILE_TEXT: u32 = 0x08;

#[repr(C)]
pub struct DBaseEntry {
    path: *mut c_char,
    compressed: [c_char; 1],
    uncompressed_size: [c_int; 1],
    data_size: [c_int; 1],
    data_offset: [c_int; 1],
}

// A representation of .DAT file.
#[repr(C)]
pub struct DBase {
    // The path of .DAT file that this structure represents.
    path: *mut c_char,

    // The offset to the beginning of data section of .DAT file.
    data_offset: c_int,

    // The number of entries.
    entries_length: [c_int; 1],

    // The array of entries.
    entries: *mut DBaseEntry,

    // The head of linked list of open file handles.
    dfile_head: *mut DFile,
}

// A handle to open entry in .DAT file.
#[repr(C)]
pub struct DFile {
    dbase: *mut DBase,
    entry: *mut DBaseEntry,
    #[allow(dead_code)]
    flags: c_int,

    // The stream of .DAT file opened for reading in binary mode.
    //
    // This stream is not shared across open handles. Instead every [DFile]
    // opens it's own stream via [fopen], which is then closed via [fclose] in
    // [dfileClose].
    stream: *mut FILE,

    // The inflate stream used to decompress data.
    //
    // This value is NULL if entry is not compressed.
    decompression_stream: z_streamp,

    // The decompression buffer of size [DFILE_DECOMPRESSION_BUFFER_SIZE].
    //
    // This value is NULL if entry is not compressed.
    decompression_buffer: *mut c_uchar,

    // The last ungot character.
    //
    // See [DFILE_HAS_UNGETC] notes.
    #[allow(dead_code)]
    ungotten: c_int,

    // The last ungot compressed character.
    //
    // This value is used when reading compressed text streams to detect
    // Windows end of line sequence \r\n.
    #[allow(dead_code)]
    compressed_ungotten: c_int,

    // The number of bytes read so far from compressed stream.
    //
    // This value is only used when reading compressed streams. The range is
    // 0..entry->dataSize.
    #[allow(dead_code)]
    compressed_bytes_read: c_int,

    // The position in read stream.
    //
    // This value is tracked in terms of uncompressed data (even in compressed
    // streams). The range is 0..entry->uncompressedSize.
    #[allow(dead_code)]
    position: c_long,

    // Next [DFile] in linked list.
    //
    // [DFile]s are stored in [DBase] in reverse order, so it's actually a
    // previous opened file, not next.
    next: *mut DFile,
}

#[repr(C)]
pub struct DFileFindData {
    // The name of file that was found during previous search.
    file_name: [c_char; COMPAT_MAX_PATH],

    // The pattern to search.
    //
    // This value is set automatically when [dbaseFindFirstEntry] succeeds so
    // that subsequent calls to [dbaseFindNextEntry] know what to look for.
    pattern: [c_char; COMPAT_MAX_PATH],

    // The index of entry that was found during previous search.
    //
    // This value is set automatically when [dbaseFindFirstEntry] and
    // [dbaseFindNextEntry] succeed so that subsequent calls to [dbaseFindNextEntry]
    // knows where to start search from.
    index: c_int,
}

// The [bsearch] comparison callback, which is used to find [DBaseEntry] for
// specified [filePath].
//
// 0x4E5D70
unsafe extern "C" fn rust_dbase_find_entry_my_file_path(
    a1: *const c_void,
    a2: *const c_void,
) -> c_int {
    let file_path = a1 as *const c_char;
    let entry = a2 as *const DBaseEntry;

    rust_compat_stricmp(file_path, (*entry).path)
}

#[no_mangle]
pub unsafe extern "C" fn rust_dfile_close(
    stream: *mut DFile
) -> c_int {
    assert_ne!(stream, null_mut()); // "stream", "dfile.c", 253

    let mut rc: c_int = 0;

    if (*(*stream).entry).compressed[0] == 1 {
        if inflateEnd((*stream).decompression_stream) != Z_OK {
            rc = -1;
        }
    }

    if (*stream).decompression_stream != null_mut() {
        free((*stream).decompression_stream as *mut c_void);
    }

    if (*stream).decompression_buffer != null_mut() {
        free((*stream).decompression_buffer as *mut c_void);
    }

    if (*stream).stream != null_mut() {
        fclose((*stream).stream);
    }

    // Loop thru open file handles and find previous to remove current handle
    // from linked list.
    //
    // NOTE: Compiled code is slightly different.
    let mut curr = (*(*stream).dbase).dfile_head;
    let mut prev = null_mut();
    while curr != null_mut() {
        if curr == stream {
            break;
        }

        prev = curr;
        curr = (*curr).next;
    }

    if curr != null_mut() {
        if prev == null_mut() {
            (*(*stream).dbase).dfile_head = (*stream).next;
        } else {
            (*prev).next = (*stream).next;
        }
    }

    memset(stream as *mut c_void, 0, mem::size_of::<DFile>());

    free(stream as *mut c_void);

    rc
}

#[cfg(target_family = "windows")]
extern "C" {
    fn bsearch(
        key: *const c_void,
        base: *const c_void,
        num: size_t,
        size: size_t,
        compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
    ) -> *mut c_void;
}

#[no_mangle]
// 0x4E5D9C
pub unsafe extern "C" fn rust_dfile_open_internal(
    dbase: *mut DBase, file_path: *const c_char, mode: *const c_char, mut dfile: *mut DFile,
) -> *mut DFile {
    assert_ne!(dbase, null_mut()); // dfile.c, 295
    assert_ne!(file_path, null()); // dfile.c, 296
    assert_ne!(mode, null()); // dfile.c, 297

    let entry = bsearch(file_path as *const c_void, (*dbase).entries as *const c_void, (*dbase).entries_length[0] as size_t, mem::size_of::<DBaseEntry>(), Some(rust_dbase_find_entry_my_file_path)) as *mut DBaseEntry;

    unsafe fn cleanup(dfile: *mut DFile) {
        if dfile != null_mut() {
            rust_dfile_close(dfile);
        }
    }

    if entry == null_mut() {
        cleanup(dfile);
        return null_mut();
    }

    if *mode != 'r' as c_char {
        cleanup(dfile);
        return null_mut();
    }

    if dfile == null_mut() {
        dfile = malloc(mem::size_of::<DFile>()) as *mut DFile;
        if dfile == null_mut() {
            return null_mut();
        }

        memset(dfile as *mut c_void, 0, mem::size_of::<DFile>());
        (*dfile).dbase = dbase;
        (*dfile).next = (*dbase).dfile_head;
        (*dbase).dfile_head = dfile;
    } else {
        if dbase != (*dfile).dbase {
            cleanup(dfile);
            return null_mut();
        }

        if (*dfile).stream != null_mut() {
            fclose((*dfile).stream);
            (*dfile).stream = null_mut();
        }

        (*dfile).compressed_bytes_read = 0;
        (*dfile).position = 0;
        (*dfile).flags = 0;
    }

    (*dfile).entry = entry;

    // Open stream to .DAT file.
    let rb = CString::new("rb").expect("valid string");
    (*dfile).stream = rust_compat_fopen((*dbase).path, rb.as_ptr());
    if (*dfile).stream == null_mut() {
        cleanup(dfile);
        return null_mut();
    }

    // Relocate stream to the beginning of data for specified entry.
    if fseek((*dfile).stream, ((*dbase).data_offset + (*entry).data_offset[0]) as c_long, SEEK_SET) != 0 {
        cleanup(dfile);
        return null_mut();
    }

    if (*entry).compressed[0] == 1 {
        // Entry is compressed, setup decompression stream and decompression
        // buffer. This step is not needed when previous instance of dfile is
        // passed via parameter, which might already have stream and
        // buffer allocated.
        if (*dfile).decompression_stream == null_mut() {
            (*dfile).decompression_stream = malloc(mem::size_of::<z_stream>()) as z_streamp;
            if (*dfile).decompression_stream == null_mut() {
                cleanup(dfile);
                return null_mut();
            }

            (*dfile).decompression_buffer = malloc(DFILE_DECOMPRESSION_BUFFER_SIZE as size_t) as *mut c_uchar;
            if (*dfile).decompression_buffer == null_mut() {
                cleanup(dfile);
                return null_mut();
            }
        }

        (*(*dfile).decompression_stream).zalloc = mem::transmute::<*const c_void, alloc_func>(null());
        (*(*dfile).decompression_stream).zfree = mem::transmute::<*const c_void, free_func>(null());
        (*(*dfile).decompression_stream).opaque = mem::transmute::<*const c_void, voidpf>(null());
        (*(*dfile).decompression_stream).next_in = (*dfile).decompression_buffer;
        (*(*dfile).decompression_stream).avail_in = 0;

        // Used ZLIB_VERSION
        let version = CString::new("1.2.11").expect("valid string");
        if inflateInit_((*dfile).decompression_stream, version.as_ptr(), mem::size_of::<z_stream>() as c_int) != Z_OK {
            cleanup(dfile);
            return null_mut();
        }
    } else {
        // Entry is not compressed, there is no need to keep decompression
        // stream and decompression buffer (in case [dfile] was passed via
        // parameter).
        if (*dfile).decompression_stream != null_mut() {
            free((*dfile).decompression_stream as *mut c_void);
            (*dfile).decompression_stream = null_mut();
        }

        if (*dfile).decompression_buffer != null_mut() {
            free((*dfile).decompression_buffer as *mut c_void);
            (*dfile).decompression_buffer = null_mut();
        }
    }

    if *mode.offset(1) == 't' as c_char {
        (*dfile).flags |= DFILE_TEXT as c_int;
    }

    dfile
}

pub unsafe extern "C" fn rust_dfile_open(
    dbase: *mut DBase, file_path: *const c_char, mode: *const c_char,
) -> *mut DFile {
    rust_dfile_open_internal(dbase, file_path, mode, null_mut())
}

// 0x4E6078
unsafe fn dfile_read_compressed(stream: *mut DFile, mut ptr: *const c_void, mut size: size_t) -> bool {
    if ((*stream).flags & DFILE_HAS_COMPRESSED_UNGETC) != 0 {
        let mut byte_buffer = ptr as *mut c_uchar;
        *byte_buffer = ((*stream).compressed_ungotten & 0xFF) as c_uchar;
        byte_buffer = byte_buffer.offset(1);
        ptr = byte_buffer as *const c_void;

        size -= 1;

        (*stream).flags &= !DFILE_HAS_COMPRESSED_UNGETC;
        (*stream).position += 1;

        if size == 0 {
            return true;
        }
    }

    (*(*stream).decompression_stream).next_out = ptr as *mut Bytef;
    (*(*stream).decompression_stream).avail_out = size as c_uint;

    loop {
        if (*(*stream).decompression_stream).avail_out == 0 {
            // Everything was decompressed.
            break;
        }

        if (*(*stream).decompression_stream).avail_in == 0 {
            // No more unprocessed data, request next chunk.
            let bytes_to_read = DFILE_DECOMPRESSION_BUFFER_SIZE.min(((*(*stream).entry).data_size[0] - (*stream).compressed_bytes_read) as u32) as size_t;

            if fread((*stream).decompression_buffer as *mut c_void, bytes_to_read, 1, (*stream).stream) != 1 {
                break;
            }

            (*(*stream).decompression_stream).avail_in = bytes_to_read as c_uint;
            (*(*stream).decompression_stream).next_in = (*stream).decompression_buffer;

            (*stream).compressed_bytes_read += bytes_to_read as c_int;
        }
        if inflate((*stream).decompression_stream, Z_NO_FLUSH) != Z_OK {
            break;
        }
    }

    if (*(*stream).decompression_stream).avail_out != 0 {
        // There are some data still waiting, which means there was in error
        // during decompression loop above.
        return false;
    }

    (*stream).position += size as c_long;

    true
}

// NOTE: Inlined.
//
// 0x4E613C
#[no_mangle]
unsafe fn dfile_unget_compressed(stream: *mut DFile, ch: c_int) {
    (*stream).compressed_ungotten = ch;
    (*stream).flags |= DFILE_HAS_COMPRESSED_UNGETC;
    (*stream).position -= 1;
}

// 0x4E5F9C
unsafe fn dfile_read_char_internal(stream: *mut DFile) -> c_int {
    if (*(*stream).entry).compressed[0] == 1 {
        let mut ch = ['\0' as c_char; 1];
        if !dfile_read_compressed(stream, ch.as_mut_ptr() as *const c_void, mem::size_of::<c_char>()) {
            return -1;
        }

        if ((*stream).flags & DFILE_TEXT as c_int) != 0 {
            // NOTE: I'm not sure if they are comparing as chars or ints. Since
            // character literals are ints, let's cast read characters to int as
            // well.
            if ch[0] == '\r' as c_char {
                let mut next_ch = ['\0' as c_char; 1];
                if dfile_read_compressed(stream, next_ch.as_mut_ptr() as *const c_void, mem::size_of::<c_char>()) {
                    if next_ch[0] == '\n' as c_char as c_char {
                        ch[0] = next_ch[0];
                    } else {
                        // NOTE: Uninline.
                        dfile_unget_compressed(stream, (next_ch[0] as c_int) & 0xFF);
                    }
                }
            }
        }

        return (ch[0] as c_int) & 0xFF;
    }

    if (*(*stream).entry).uncompressed_size[0] < 0 || (*stream).position >= (*(*stream).entry).uncompressed_size[0] as c_long {
        return -1;
    }

    let mut ch = fgetc((*stream).stream);
    if ch != -1 {
        if ((*stream).flags & DFILE_TEXT as c_int) != 0 {
            // This is a text stream, attempt to detect \r\n sequence.
            if ch == '\r' as c_int {
                if (*stream).position + 1 < ((*(*stream).entry).uncompressed_size[0] as c_long) {
                    let next_ch = fgetc((*stream).stream);
                    if next_ch == '\n' as c_int {
                        ch = next_ch;
                        (*stream).position += 1;
                    } else {
                        ungetc(next_ch, (*stream).stream);
                    }
                }
            }
        }

        (*stream).position += 1;
    }

    ch
}

#[no_mangle]
pub unsafe extern "C" fn rust_dbase_close(dbase: *const DBase) -> bool {
    assert_ne!(dbase, null()); // "dbase", "dfile.c", 173

    let mut curr = (*dbase).dfile_head;
    while curr != null_mut() {
        let next = (*curr).next;
        rust_dfile_close(curr);
        curr = next;
    }

    if (*dbase).entries != null_mut() {
        for index in 0..((*dbase).entries_length[0]) {
            let entry = (*dbase).entries.offset(index as isize);
            let entry_name = (*entry).path;
            if entry_name != null_mut() {
                free(entry_name as *mut c_void);
            }
        }
        free((*dbase).entries as *mut c_void);
    }

    if (*dbase).path != null_mut() {
        free((*dbase).path as *mut c_void);
    }

    memset(dbase as *mut c_void, 0, mem::size_of::<DBase>());

    free(dbase as *mut c_void);

    true
}

#[no_mangle]
pub unsafe extern "C" fn rust_dbase_open_part(file_path: *const c_char) -> *const DBase {
    assert_ne!(file_path, null()); // "filename", "dfile.c", 74

    let rb = CString::new("rb").expect("valid string");
    let str = rb.as_ptr();
    let stream = rust_compat_fopen(file_path, str);
    if stream == null_mut() {
        return null();
    }

    let dbase = malloc(mem::size_of::<DBase>()) as *mut DBase;
    if dbase == null_mut() {
        fclose(stream);
        return null();
    }

    memset(dbase as *mut c_void, 0, mem::size_of_val(&*dbase));

    unsafe fn close_on_error(dbase: *mut DBase, stream: *mut FILE) {
        rust_dbase_close(dbase);
        fclose(stream);
    }

    // Get file size, and reposition stream to read footer, which contains two
    // 32-bits ints.
    let file_size = rust_get_file_size(stream) as c_int;

    if fseek(stream, (file_size - mem::size_of::<c_int>() as c_int * 2) as c_long, SEEK_SET) != 0 {
        close_on_error(dbase, stream);
        return null();
    }

    // Read the size of entries table.
    let mut entries_data_size = [0 as c_int; 1];
    if fread(entries_data_size.as_mut_ptr() as *mut c_void, mem::size_of_val(&entries_data_size), 1, stream) != 1 {
        close_on_error(dbase, stream);
        return null();
    }

    // Read the size of entire dbase content.
    //
    // NOTE: It appears that this approach allows existence of arbitrary data in
    // the beginning of the .DAT file.
    let mut dbase_data_size = [0 as c_int; 1];
    if fread(dbase_data_size.as_mut_ptr() as *mut c_void, mem::size_of_val(&dbase_data_size), 1, stream) != 1 {
        close_on_error(dbase, stream);
        return null();
    }

    // Reposition stream to the beginning of the entries table.
    if fseek(stream, (file_size - entries_data_size[0] as c_int - mem::size_of::<c_int>() as c_int * 2) as c_long, SEEK_SET) != 0 {
        close_on_error(dbase, stream);
        return null();
    }

    if fread((*dbase).entries_length.as_mut_ptr() as *mut c_void, mem::size_of_val(&(*dbase).entries_length), 1, stream) != 1 {
        close_on_error(dbase, stream);
        return null();
    }

    let entries_allocation_size = mem::size_of_val(&*(*dbase).entries) * (*dbase).entries_length[0] as usize;
    (*dbase).entries = malloc(entries_allocation_size) as *mut DBaseEntry;
    if (*dbase).entries == null_mut() {
        close_on_error(dbase, stream);
        return null();
    }

    memset((*dbase).entries as *mut c_void, 0, entries_allocation_size);

    // Read entries one by one, stopping on any error.
    let mut entry_index = 0;
    for i in 0..(*dbase).entries_length[0] {
        let entry = (*dbase).entries.offset(i as isize);

        let mut path_length = [0 as c_int; 1];
        if fread(path_length.as_mut_ptr() as *mut c_void, mem::size_of_val(&path_length), 1, stream) != 1 {
            break;
        }

        (*entry).path = malloc(path_length[0] as size_t + 1) as *mut c_char;
        if (*entry).path == null_mut() {
            break;
        }

        if fread((*entry).path as *mut c_void, path_length[0] as size_t, 1, stream) != 1 {
            break;
        }

        *(*entry).path.offset(path_length[0] as isize) = '\0' as c_char;

        if fread((*entry).compressed.as_mut_ptr() as *mut c_void, mem::size_of_val(&(*entry).compressed), 1, stream) != 1 {
            break;
        }

        if fread((*entry).uncompressed_size.as_mut_ptr() as *mut c_void, mem::size_of_val(&(*entry).uncompressed_size), 1, stream) != 1 {
            break;
        }

        if fread((*entry).data_size.as_mut_ptr() as *mut c_void, mem::size_of_val(&(*entry).data_size), 1, stream) != 1 {
            break;
        }

        if fread((*entry).data_offset.as_mut_ptr() as *mut c_void, mem::size_of_val(&(*entry).data_offset), 1, stream) != 1 {
            break;
        }

        entry_index = i + 1;
    }

    if entry_index < (*dbase).entries_length[0] {
        // We haven't reached the end, which means there was an error while
        // reading entries.
        close_on_error(dbase, stream);
        return null();
    }

    (*dbase).path = rust_compat_strdup(file_path);
    (*dbase).data_offset = file_size as c_int - dbase_data_size[0] as c_int;

    fclose(stream);

    dbase
}

#[no_mangle]
pub unsafe extern "C" fn rust_dbase_find_first_entry(dbase: *const DBase, find_file_data: *mut DFileFindData, pattern: *const c_char) -> bool {
    for index in 0..(*dbase).entries_length[0] {
        let entry = (*dbase).entries.offset(index as isize);
        if fpattern_match(pattern, (*entry).path) {
            strcpy((*find_file_data).file_name.as_mut_ptr() as *mut c_char, (*entry).path);
            strcpy((*find_file_data).pattern.as_mut_ptr() as *mut c_char, pattern);
            (*find_file_data).index = index as c_int;
            return true;
        }
    }

    false
}

#[no_mangle]
pub unsafe extern "C" fn rust_dbase_find_next_entry(dbase: *const DBase, find_file_data: *mut DFileFindData) -> bool {
    for index in ((*find_file_data).index + 1)..(*dbase).entries_length[0] {
        let entry = (*dbase).entries.offset(index as isize);
        if fpattern_match((*find_file_data).pattern.as_mut_ptr() as *mut c_char, (*entry).path) {
            strcpy((*find_file_data).file_name.as_mut_ptr() as *mut c_char, (*entry).path);
            (*find_file_data).index = index;
            return true;
        }
    }

    false
}

#[no_mangle]
pub unsafe extern "C" fn rust_dfile_read_char(stream: *mut DFile) -> c_int {
    assert_ne!(stream, null_mut()); // "stream", "dfile.c", 384

    if ((*stream).flags & DFILE_EOF as c_int) != 0 || ((*stream).flags & DFILE_ERROR as c_int) != 0 {
        return -1;
    }

    if ((*stream).flags & DFILE_HAS_UNGETC as c_int) != 0 {
        (*stream).flags &= !DFILE_HAS_UNGETC as c_int;
        return (*stream).ungotten;
    }

    let ch = dfile_read_char_internal(stream);
    if ch == -1 {
        (*stream).flags |= DFILE_EOF as c_int;
    }

    ch
}

#[no_mangle]
pub unsafe extern "C" fn rust_dfile_read_string(string: *mut c_char, mut size: c_int, stream: *mut DFile) -> *const c_char {
    assert_ne!(string, null_mut()); // "s", "dfile.c", 407
    assert_ne!(size, 0); // "n", "dfile.c", 408
    assert_ne!(stream, null_mut()); // "stream", "dfile.c", 409

    if ((*stream).flags & DFILE_EOF as c_int) != 0 || ((*stream).flags & DFILE_ERROR as c_int) != 0 {
        return null();
    }

    let mut pch = string;

    if ((*stream).flags & DFILE_HAS_UNGETC as c_int) != 0 {
        *pch = ((*stream).ungotten & 0xFF as c_int) as c_char;
        pch = pch.offset(1);
        size -= 1;
        (*stream).flags &= !DFILE_HAS_UNGETC as c_int;
    }

    // Read up to size - 1 characters one by one saving space for the null
    // terminator.
    for _ in 0..(size - 1) {
        let ch = dfile_read_char_internal(stream);
        if ch == -1 {
            break;
        }

        *pch = (ch & 0xFF as c_int) as c_char;
        pch = pch.offset(1);

        if ch == '\n' as c_int {
            break;
        }
    }

    if pch == string {
        // No character was set into the buffer.
        return null();
    }

    *pch = '\0' as c_char;

    string
}

#[no_mangle]
pub unsafe extern "C" fn rust_dfile_read(mut ptr: *const c_void, size: size_t, count: size_t, stream: *mut DFile) -> size_t {
    assert_ne!(ptr, null_mut()); // "ptr", "dfile.c", 499
    assert_ne!(stream, null_mut()); // "stream", dfile.c, 500

    if ((*stream).flags & DFILE_EOF as c_int) != 0 || ((*stream).flags & DFILE_ERROR as c_int) != 0 {
        return 0;
    }

    let mut remaining_size = (*(*stream).entry).uncompressed_size[0] as c_long - (*stream).position;
    if ((*stream).flags & DFILE_HAS_UNGETC as c_int) != 0 {
        remaining_size += 1;
    }

    let mut bytes_to_read = size * count;
    if remaining_size < bytes_to_read as c_long {
        bytes_to_read = remaining_size as size_t;
        (*stream).flags |= DFILE_EOF as c_int;
    }

    let mut extra_bytes_read = 0;
    if ((*stream).flags & DFILE_HAS_UNGETC as c_int) != 0 {
        let mut byte_buffer = ptr as *mut c_uchar;
        *byte_buffer = ((*stream).ungotten & 0xFF as c_int) as c_uchar;
        byte_buffer = byte_buffer.offset(1);
        ptr = byte_buffer as *const c_void;

        bytes_to_read -= 1;

        (*stream).flags &= !DFILE_HAS_UNGETC as c_int;
        extra_bytes_read = 1;
    }

    let bytes_read;
    if (*(*stream).entry).compressed[0] == 1 {
        if !dfile_read_compressed(stream, ptr, bytes_to_read) {
            (*stream).flags |= DFILE_ERROR as c_int;
            return 0;
        }

        bytes_read = bytes_to_read;
    } else {
        bytes_read = fread(ptr as *mut c_void, 1, bytes_to_read, (*stream).stream) + extra_bytes_read;
        (*stream).position += bytes_read as c_long;
    }

    bytes_read / size
}

#[no_mangle]
pub unsafe extern "C" fn rust_dfile_seek(stream: *mut DFile, offset: c_long, origin: c_int) -> c_int {
    assert_ne!(stream, null_mut()); // "stream", "dfile.c", 569

    if ((*stream).flags & DFILE_ERROR as c_int) != 0 {
        return 1;
    }

    if ((*stream).flags & DFILE_TEXT as c_int) != 0 {
        if offset != 0 && origin != SEEK_SET {
            // NOTE: For unknown reason this function does not allow arbitrary
            // seeks in text streams, whether compressed or not. It only
            // supports rewinding. Probably because of reading functions which
            // handle \r\n sequence as \n.
            return 1;
        }
    }

    let offset_from_beginning = match origin {
        SEEK_SET => offset,
        SEEK_CUR => (*stream).position + offset,
        SEEK_END => (*(*stream).entry).uncompressed_size[0] as c_long + offset,
        _ => return 1
    };

    if offset_from_beginning >= (*(*stream).entry).uncompressed_size[0] as c_long {
        return 1;
    }

    let pos = (*stream).position;
    if offset_from_beginning == pos {
        (*stream).flags &= !(DFILE_HAS_UNGETC | DFILE_EOF) as c_int;
        return 0;
    }

    if offset_from_beginning != 0 {
        if (*(*stream).entry).compressed[0] == 1 {
            if offset_from_beginning < pos {
                // We cannot go backwards in compressed stream, so the only way
                // is to start from the beginning.
                rust_dfile_rewind(stream);
            }

            // Consume characters one by one until we reach specified offset.
            while offset_from_beginning > (*stream).position {
                if dfile_read_char_internal(stream) == -1 {
                    return 1;
                }
            }
        } else {
            if fseek((*stream).stream, offset_from_beginning - pos, SEEK_CUR) != 0 {
                (*stream).flags |= DFILE_ERROR as c_int;
                return 1;
            }

            // FIXME: I'm not sure what this assignment means. This field is
            // only meaningful when reading compressed streams.
            (*stream).compressed_bytes_read = offset_from_beginning as c_int;
        }

        (*stream).flags &= !(DFILE_HAS_UNGETC | DFILE_EOF) as c_int;
        return 0;
    }

    if fseek((*stream).stream, ((*(*stream).dbase).data_offset + (*(*stream).entry).data_offset[0]) as c_long, SEEK_SET) != 0 {
        (*stream).flags |= DFILE_ERROR as c_int;
        return 1;
    }

    if inflateEnd((*stream).decompression_stream) != Z_OK {
        (*stream).flags |= DFILE_ERROR as c_int;
        return 1;
    }

    (*(*stream).decompression_stream).zalloc = mem::transmute::<*const c_void, alloc_func>(null());
    (*(*stream).decompression_stream).zfree = mem::transmute::<*const c_void, free_func>(null());
    (*(*stream).decompression_stream).opaque = mem::transmute::<*const c_void, voidpf>(null());
    (*(*stream).decompression_stream).next_in = (*stream).decompression_buffer;
    (*(*stream).decompression_stream).avail_in = 0;

    //inflateInit_((strm), ZLIB_VERSION, (int)sizeof(z_stream))
    // Used ZLIB_VERSION
    let version = CString::new("1.2.11").expect("valid string");
    if inflateInit_((*stream).decompression_stream, version.as_ptr(), mem::size_of::<z_stream>() as c_int) != Z_OK {
        (*stream).flags |= DFILE_ERROR as c_int;
        return 1;
    }

    (*stream).position = 0;
    (*stream).compressed_bytes_read = 0;
    (*stream).flags &= !(DFILE_HAS_UNGETC | DFILE_EOF) as c_int;

    0
}

#[no_mangle]
// 0x4E5D9C
pub unsafe extern "C" fn rust_dfile_rewind(stream: *mut DFile)
{
    assert_ne!(stream, null_mut()); // "stream", "dfile.c", 664

    rust_dfile_seek(stream, 0, SEEK_SET);

    (*stream).flags &= !DFILE_ERROR as c_int;
}

