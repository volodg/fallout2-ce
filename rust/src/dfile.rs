use crate::platform_compat::rust_compat_stricmp;
use libc::{c_char, c_int, c_long, c_uchar, c_uint, fclose, FILE, free, memset};
use std::ffi::c_void;
use std::mem;
use std::ptr::null_mut;
use libz_sys::{inflateEnd, Z_OK, z_streamp};

#[repr(C)]
struct DBaseEntry {
    path: *const c_char,
    compressed: c_uint,
    uncompressed_size: c_int,
    data_size: c_int,
    data_offset: c_int,
}

// A representation of .DAT file.
#[repr(C)]
struct DBase {
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
#[no_mangle]
pub unsafe extern "C" fn rust_dbase_find_entry_my_file_path(
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

/*
#[no_mangle]
pub unsafe extern "C" fn rust_dfileOpenInternal(
    dbase: *const DBase, file_path: *const c_char, mode: *const c_char, dfile: *const DFile
) -> *const DFile {
    let entry = bsearch(filePath, dbase->entries, dbase->entriesLength, sizeof(*dbase->entries), rust_dbase_find_entry_my_file_path) as *const DBaseEntry;

    fn cleanup(dfile: *const DFile) {
        if dfile != null() {
            dfileClose(dfile);
        }
    }

    if entry == nullptr {
        goto err;
    }

    if (mode[0] != 'r') {
        goto err;
    }

    if (dfile == nullptr) {
        dfile = (DFile*)malloc(sizeof(*dfile));
        if (dfile == nullptr) {
            return nullptr;
        }

        memset(dfile, 0, sizeof(*dfile));
        dfile->dbase = dbase;
        dfile->next = dbase->dfileHead;
        dbase->dfileHead = dfile;
    } else {
        if (dbase != dfile->dbase) {
            goto err;
        }

        if (dfile->stream != nullptr) {
            fclose(dfile->stream);
            dfile->stream = nullptr;
        }

        dfile->compressedBytesRead = 0;
        dfile->position = 0;
        dfile->flags = 0;
    }

    dfile->entry = entry;

    // Open stream to .DAT file.
    dfile->stream = compat_fopen(dbase->path, "rb");
    if (dfile->stream == nullptr) {
        goto err;
    }

    // Relocate stream to the beginning of data for specified entry.
    if (fseek(dfile->stream, dbase->dataOffset + entry->dataOffset, SEEK_SET) != 0) {
        goto err;
    }

    if (entry->compressed == 1) {
        // Entry is compressed, setup decompression stream and decompression
        // buffer. This step is not needed when previous instance of dfile is
        // passed via parameter, which might already have stream and
        // buffer allocated.
        if (dfile->decompressionStream == nullptr) {
            dfile->decompressionStream = (z_streamp)malloc(sizeof(*dfile->decompressionStream));
            if (dfile->decompressionStream == nullptr) {
                goto err;
            }

            dfile->decompressionBuffer = (unsigned char*)malloc(DFILE_DECOMPRESSION_BUFFER_SIZE);
            if (dfile->decompressionBuffer == nullptr) {
                goto err;
            }
        }

        dfile->decompressionStream->zalloc = Z_NULL;
        dfile->decompressionStream->zfree = Z_NULL;
        dfile->decompressionStream->opaque = Z_NULL;
        dfile->decompressionStream->next_in = dfile->decompressionBuffer;
        dfile->decompressionStream->avail_in = 0;

        if (inflateInit(dfile->decompressionStream) != Z_OK) {
            goto err;
        }
    } else {
        // Entry is not compressed, there is no need to keep decompression
        // stream and decompression buffer (in case [dfile] was passed via
        // parameter).
        if (dfile->decompressionStream != nullptr) {
            free(dfile->decompressionStream);
            dfile->decompressionStream = nullptr;
        }

        if (dfile->decompressionBuffer != nullptr) {
            free(dfile->decompressionBuffer);
            dfile->decompressionBuffer = nullptr;
        }
    }

    if (mode[1] == 't') {
        dfile->flags |= DFILE_TEXT;
    }

    return dfile;

    err:

    if dfile != nullptr {
        dfileClose(dfile);
    }

    null()
}
*/
