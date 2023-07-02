use crate::platform_compat::{rust_compat_fopen, rust_compat_stricmp};
use libc::{bsearch, c_char, c_int, c_long, c_uchar, c_uint, fclose, fgetc, FILE, fread, free, fseek, malloc, memset, SEEK_SET, size_t, ungetc};
use std::ffi::{c_void, CString};
use std::mem;
use std::ptr::{null, null_mut};
use libz_sys::{alloc_func, Bytef, free_func, inflate, inflateEnd, inflateInit_, voidpf, Z_NO_FLUSH, Z_OK, z_stream, z_streamp};

const DFILE_DECOMPRESSION_BUFFER_SIZE: u32 = 0x400;
const DFILE_TEXT: c_int = 0x08;

// Specifies that [DFile] has unget compressed character.
const DFILE_HAS_COMPRESSED_UNGETC: c_int = 0x10;

#[repr(C)]
pub struct DBaseEntry {
    path: *const c_char,
    compressed: c_uint,
    uncompressed_size: c_int,
    data_size: c_int,
    data_offset: c_int,
}

// A representation of .DAT file.
#[repr(C)]
pub struct DBase {
    // The path of .DAT file that this structure represents.
    path: *mut c_char,

    // The offset to the beginning of data section of .DAT file.
    data_offset: c_int,

    // The number of entries.
    entries_length: c_int,

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

    if (*(*stream).entry).compressed == 1 {
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

#[no_mangle]
// 0x4E5D9C
pub unsafe extern "C" fn rust_dfile_open_internal(
    dbase: *mut DBase, file_path: *const c_char, mode: *const c_char, mut dfile: *mut DFile
) -> *const DFile {
    let entry = bsearch(file_path as *const c_void, (*dbase).entries as *const c_void, (*dbase).entries_length as size_t, mem::size_of::<DBaseEntry>(), Some(rust_dbase_find_entry_my_file_path)) as *mut DBaseEntry;

    unsafe fn cleanup(dfile: *mut DFile) {
        if dfile != null_mut() {
            rust_dfile_close(dfile);
        }
    }

    if entry == null_mut() {
        cleanup(dfile);
        return null()
    }

    if *mode != 'r' as c_char {
        cleanup(dfile);
        return null()
    }

    if dfile == null_mut() {
        dfile = malloc(mem::size_of::<DFile>()) as *mut DFile;
        if dfile == null_mut() {
            return null();
        }

        memset(dfile as *mut c_void, 0, mem::size_of::<DFile>());
        (*dfile).dbase = dbase;
        (*dfile).next = (*dbase).dfile_head;
        (*dbase).dfile_head = dfile;
    } else {
        if dbase != (*dfile).dbase {
            cleanup(dfile);
            return null()
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
        return null()
    }

    // Relocate stream to the beginning of data for specified entry.
    if fseek((*dfile).stream, ((*dbase).data_offset + (*entry).data_offset) as c_long, SEEK_SET) != 0 {
        cleanup(dfile);
        return null()
    }

    if (*entry).compressed == 1 {
        // Entry is compressed, setup decompression stream and decompression
        // buffer. This step is not needed when previous instance of dfile is
        // passed via parameter, which might already have stream and
        // buffer allocated.
        if (*dfile).decompression_stream == null_mut() {
            (*dfile).decompression_stream = malloc(mem::size_of::<z_stream>()) as z_streamp;
            if (*dfile).decompression_stream == null_mut() {
                cleanup(dfile);
                return null()
            }

            (*dfile).decompression_buffer = malloc(DFILE_DECOMPRESSION_BUFFER_SIZE as size_t) as *mut c_uchar;
            if (*dfile).decompression_buffer == null_mut() {
                cleanup(dfile);
                return null()
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
            return null()
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
        (*dfile).flags |= DFILE_TEXT;
    }

    dfile
}

#[no_mangle]
// 0x4E6078
pub unsafe extern "C" fn rust_dfile_read_compressed(stream: *mut DFile, mut ptr: *const c_void, mut size: size_t) -> bool {
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
            let bytes_to_read = DFILE_DECOMPRESSION_BUFFER_SIZE.min(((*(*stream).entry).data_size - (*stream).compressed_bytes_read) as u32) as size_t;

            if fread((*stream).decompression_buffer as *mut c_void, bytes_to_read, 1, (*stream).stream) != 1 {
                break;
            }

            (*(*stream).decompression_stream).avail_in = bytes_to_read as c_uint;
            (*(*stream).decompression_stream).next_in = (*stream).decompression_buffer;

            (*stream).compressed_bytes_read += bytes_to_read as c_int;
        }
        if inflate((*stream).decompression_stream, Z_NO_FLUSH) != Z_OK {
            break
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
unsafe fn rust_dfile_unget_compressed(stream: *mut DFile, ch: c_int) {
    (*stream).compressed_ungotten = ch;
    (*stream).flags |= DFILE_HAS_COMPRESSED_UNGETC;
    (*stream).position -= 1;
}

// 0x4E5F9C
#[no_mangle]
pub unsafe extern "C" fn rust_dfile_read_char_internal(stream: *mut DFile) -> c_int {
    if (*(*stream).entry).compressed == 1 {
        let mut ch = ['\0' as c_char; 1];
        if !rust_dfile_read_compressed(stream, ch.as_mut_ptr() as *const c_void, mem::size_of::<c_char>()) {
            return -1;
        }

        if ((*stream).flags & DFILE_TEXT) != 0 {
            // NOTE: I'm not sure if they are comparing as chars or ints. Since
            // character literals are ints, let's cast read characters to int as
            // well.
            if ch[0] == '\r' as c_char {
                let mut next_ch = ['\0' as c_char; 1];
                if rust_dfile_read_compressed(stream, next_ch.as_mut_ptr() as *const c_void, mem::size_of::<c_char>()) {
                    if next_ch[0] == '\n' as c_char as c_char {
                        ch[0] = next_ch[0];
                    } else {
                        // NOTE: Uninline.
                        rust_dfile_unget_compressed(stream, (next_ch[0] as c_int) & 0xFF);
                    }
                }
            }
        }

        return (ch[0] as c_int) & 0xFF;
    }

    if (*(*stream).entry).uncompressed_size < 0 || (*stream).position >= (*(*stream).entry).uncompressed_size as c_long {
        return -1;
    }

    let mut ch = fgetc((*stream).stream);
    if ch != -1 {
        if ((*stream).flags & DFILE_TEXT) != 0 {
            // This is a text stream, attempt to detect \r\n sequence.
            if ch == '\r' as c_int {
                if (*stream).position + 1 < ((*(*stream).entry).uncompressed_size as c_long) {
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
        for index in 0..((*dbase).entries_length) {
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

/*
bool dbaseClose(DBase* dbase)
{
}
 */

/*
#[no_mangle]
pub unsafe extern "C" fn rust_dbase_open(file_path: *const c_char) -> *const DBase {
    assert_ne!(file_path, null()); // "filename", "dfile.c", 74

    let rb = CString::new("rb").expect("valid string");
    let stream = rust_compat_fopen(filePath, rb.as_ptr());
    if stream == null_mut() {
        return null();
    }

    let dbase = malloc(mem::size_of::<DBase>()) as *mut DBase;
    if dbase == null_mut() {
        fclose(stream);
        return null();
    }

    memset(dbase as *mut c_void, 0, mem::size_of::<DBase>());

    // Get file size, and reposition stream to read footer, which contains two
    // 32-bits ints.
    let file_size = rust_get_file_size(stream);
    if (fseek(stream, file_size - sizeof(int) * 2, SEEK_SET) != 0) {
        goto err;
    }

    // Read the size of entries table.
    int entriesDataSize;
    if (fread(&entriesDataSize, sizeof(entriesDataSize), 1, stream) != 1) {
        goto err;
    }

    fn closeOnError(dbase: *mut DBase, stream: *mut FILE) {
        rust_dbase_close();
        // dbaseClose(dbase);

        fclose(stream);
    }

    // Read the size of entire dbase content.
    //
    // NOTE: It appears that this approach allows existence of arbitrary data in
    // the beginning of the .DAT file.
    int dbaseDataSize;
    if (fread(&dbaseDataSize, sizeof(dbaseDataSize), 1, stream) != 1) {
        goto err;
    }

    // Reposition stream to the beginning of the entries table.
    if (fseek(stream, fileSize - entriesDataSize - sizeof(int) * 2, SEEK_SET) != 0) {
        goto err;
    }

    if (fread(&(dbase->entriesLength), sizeof(dbase->entriesLength), 1, stream) != 1) {
        goto err;
    }

    dbase->entries = (DBaseEntry*)malloc(sizeof(*dbase->entries) * dbase->entriesLength);
    if (dbase->entries == nullptr) {
        goto err;
    }

    memset(dbase->entries, 0, sizeof(*dbase->entries) * dbase->entriesLength);

    // Read entries one by one, stopping on any error.
    int entryIndex;
    for (entryIndex = 0; entryIndex < dbase->entriesLength; entryIndex++) {
        DBaseEntry* entry = &(dbase->entries[entryIndex]);

        int pathLength;
        if (fread(&pathLength, sizeof(pathLength), 1, stream) != 1) {
            break;
        }

        entry->path = (char*)malloc(pathLength + 1);
        if (entry->path == nullptr) {
            break;
        }

        if (fread(entry->path, pathLength, 1, stream) != 1) {
            break;
        }

        entry->path[pathLength] = '\0';

        if (fread(&(entry->compressed), sizeof(entry->compressed), 1, stream) != 1) {
            break;
        }

        if (fread(&(entry->uncompressedSize), sizeof(entry->uncompressedSize), 1, stream) != 1) {
            break;
        }

        if (fread(&(entry->dataSize), sizeof(entry->dataSize), 1, stream) != 1) {
            break;
        }

        if (fread(&(entry->dataOffset), sizeof(entry->dataOffset), 1, stream) != 1) {
            break;
        }
    }

    if (entryIndex < dbase->entriesLength) {
        // We haven't reached the end, which means there was an error while
        // reading entries.
        goto err;
    }

    dbase->path = compat_strdup(filePath);
    dbase->dataOffset = fileSize - dbaseDataSize;

    fclose(stream);

    return dbase;

    err:

        dbaseClose(dbase);

    fclose(stream);

    return nullptr;
}
*/

/*
DBase* dbaseOpen(const char* filePath)
{
}
 */
