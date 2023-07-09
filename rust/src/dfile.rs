use std::cell::RefCell;
use crate::fpattern::fpattern_match;
use crate::platform_compat::{rust_compat_fopen, rust_compat_strdup, rust_get_file_size, COMPAT_MAX_PATH, compat_stricmp};
use libc::{
    c_char, c_int, c_long, c_uchar, c_uint, fclose, fgetc, fread, free, fseek, malloc, memset,
    size_t, strcpy, ungetc, FILE, SEEK_CUR, SEEK_END, SEEK_SET,
};
use libz_sys::{
    alloc_func, free_func, inflate, inflateEnd, inflateInit_, voidpf, z_stream, z_streamp, Bytef,
    Z_NO_FLUSH, Z_OK,
};
use std::ffi::{c_void, CString};
use std::mem;
use std::ptr::{null, null_mut};
use std::rc::Rc;

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

#[derive(Clone)]
struct DBaseEntry {
    path: Option<CString>,
    compressed: [i8; 1],
    uncompressed_size: [i32; 1],
    data_size: [i32; 1],
    data_offset: [i32; 1],
}

impl Default for DBaseEntry {
    fn default() -> Self {
        Self {
            path: None,
            compressed: [0 as i8; 1],
            uncompressed_size: [0 as i32; 1],
            data_size: [0 as i32; 1],
            data_offset: [0 as i32; 1],
        }
    }
}

impl DBaseEntry {
    fn get_path_cstr(&self) -> *const c_char {
        self.path.as_ref().map(|x| x.as_ptr()).unwrap_or(null())
    }
}

// A representation of .DAT file.
pub struct DBase {
    // The path of .DAT file that this structure represents.
    path: Option<CString>,

    // The offset to the beginning of data section of .DAT file.
    data_offset: i32,

    // The number of entries.
    entries_length: [i32; 1],

    // The array of entries.
    entries: Option<Vec<DBaseEntry>>,

    // The head of linked list of open file handles.
    dfile_head: Option<Rc<RefCell<DFile>>>,
}

impl DBase {
    fn get_path_cstr(&self) -> *const c_char {
        self.path.as_ref().map(|x| x.as_ptr()).unwrap_or(null())
    }
}

// A handle to open entry in .DAT file.
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
    next: Option<Rc<RefCell<DFile>>>,
}

impl Default for DFile {
    fn default() -> Self {
        Self {
            dbase: null_mut(),
            entry: null_mut(),
            flags: 0,
            stream: null_mut(),
            decompression_stream: z_streamp::from(null_mut()),
            decompression_buffer: null_mut(),
            ungotten: 0,
            compressed_ungotten: 0,
            compressed_bytes_read: 0,
            position: 0,
            next: None,
        }
    }
}

pub struct DFileFindData {
    // The name of file that was found during previous search.
    pub file_name: [c_char; COMPAT_MAX_PATH],

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

impl Default for DFileFindData {
    fn default() -> Self {
        Self {
            file_name: [0 as c_char; COMPAT_MAX_PATH],
            pattern: [0 as c_char; COMPAT_MAX_PATH],
            index: 0,
        }
    }
}

pub unsafe fn dfile_close(stream_rc: Rc<RefCell<DFile>>) -> c_int {
    let mut rc: c_int = 0;

    let stream = stream_rc.borrow();
    if (*stream.entry).compressed[0] == 1 {
        if inflateEnd(stream.decompression_stream) != Z_OK {
            rc = -1;
        }
    }

    if stream.decompression_stream != null_mut() {
        free(stream.decompression_stream as *mut c_void);
    }

    if stream.decompression_buffer != null_mut() {
        free(stream.decompression_buffer as *mut c_void);
    }

    if stream.stream != null_mut() {
        fclose(stream.stream);
    }

    // Loop thru open file handles and find previous to remove current handle
    // from linked list.
    //
    // NOTE: Compiled code is slightly different.
    let mut curr = (*stream.dbase).dfile_head.clone();
    let mut prev: Option<Rc<RefCell<DFile>>> = None;
    while curr.is_some() {
        if Rc::ptr_eq(curr.as_ref().expect(""), &stream_rc) {
            break;
        }

        prev = curr.clone();
        curr = curr.expect("").borrow().next.clone();
    }

    if curr.is_some() {
        if prev.is_none() {
            (*stream.dbase).dfile_head = stream.next.clone();
        } else {
            prev.expect("").borrow_mut().next = (*stream).next.clone();
        }
    }

    rc
}

pub unsafe fn rust_dfile_open(
    dbase: *mut DBase,
    file_path: *const c_char,
    mode: *const c_char,
) -> Option<Rc<RefCell<DFile>>> {
    assert_ne!(dbase, null_mut()); // dfile.c, 295
    assert_ne!(file_path, null()); // dfile.c, 296
    assert_ne!(mode, null()); // dfile.c, 297

    let entries = (*dbase).entries.as_mut().expect("");
    let optional_entry = entries.binary_search_by(|a| {
        compat_stricmp(a.get_path_cstr(), file_path)
    }).map(|i| &mut entries[i]);

    if optional_entry.is_err() {
        return None;
    }

    if *mode != 'r' as c_char {
        return None;
    }

    let dfile = Rc::new(RefCell::new(DFile::default()));

    dfile.borrow_mut().dbase = dbase;
    dfile.borrow_mut().next = (*dbase).dfile_head.clone();
    (*dbase).dfile_head = Some(dfile.clone());

    let entry = optional_entry.expect("valid entry") as *mut DBaseEntry;
    dfile.borrow_mut().entry = entry;

    // Open stream to .DAT file.
    let rb = CString::new("rb").expect("valid string");
    dfile.borrow_mut().stream = rust_compat_fopen((*dbase).get_path_cstr(), rb.as_ptr());
    if dfile.borrow().stream == null_mut() {
        dfile_close(dfile);
        return None;
    }

    // Relocate stream to the beginning of data for specified entry.
    if fseek(
        dfile.borrow().stream,
        ((*dbase).data_offset + (*entry).data_offset[0]) as c_long,
        SEEK_SET,
    ) != 0
    {
        dfile_close(dfile);
        return None;
    }

    if (*entry).compressed[0] == 1 {
        // Entry is compressed, setup decompression stream and decompression
        // buffer. This step is not needed when previous instance of dfile is
        // passed via parameter, which might already have stream and
        // buffer allocated.
        if dfile.borrow().decompression_stream == null_mut() {
            dfile.borrow_mut().decompression_stream = malloc(mem::size_of::<z_stream>()) as z_streamp;
            if dfile.borrow().decompression_stream == null_mut() {
                dfile_close(dfile);
                return None;
            }

            dfile.borrow_mut().decompression_buffer =
                malloc(DFILE_DECOMPRESSION_BUFFER_SIZE as size_t) as *mut c_uchar;
            if dfile.borrow().decompression_buffer == null_mut() {
                dfile_close(dfile);
                return None;
            }
        }

        (*dfile.borrow().decompression_stream).zalloc =
            mem::transmute::<*const c_void, alloc_func>(null());
        (*dfile.borrow().decompression_stream).zfree = mem::transmute::<*const c_void, free_func>(null());
        (*dfile.borrow().decompression_stream).opaque = mem::transmute::<*const c_void, voidpf>(null());
        (*dfile.borrow().decompression_stream).next_in = dfile.borrow().decompression_buffer;
        (*dfile.borrow().decompression_stream).avail_in = 0;

        // Used ZLIB_VERSION
        let version = CString::new("1.2.11").expect("valid string");
        if inflateInit_(
            dfile.borrow().decompression_stream,
            version.as_ptr(),
            mem::size_of::<z_stream>() as c_int,
        ) != Z_OK
        {
            dfile_close(dfile);
            return None;
        }
    } else {
        // Entry is not compressed, there is no need to keep decompression
        // stream and decompression buffer (in case [dfile] was passed via
        // parameter).
        if dfile.borrow().decompression_stream != null_mut() {
            free(dfile.borrow().decompression_stream as *mut c_void);
            dfile.borrow_mut().decompression_stream = null_mut();
        }

        if dfile.borrow().decompression_buffer != null_mut() {
            free(dfile.borrow().decompression_buffer as *mut c_void);
            dfile.borrow_mut().decompression_buffer = null_mut();
        }
    }

    if *mode.offset(1) == 't' as c_char {
        dfile.borrow_mut().flags |= DFILE_TEXT as c_int;
    }

    Some(dfile)
}

// 0x4E6078
unsafe fn dfile_read_compressed(
    stream: &mut DFile,
    mut ptr: *const c_void,
    mut size: size_t,
) -> bool {
    if ((*stream).flags & DFILE_HAS_COMPRESSED_UNGETC) != 0 {
        let mut byte_buffer = ptr as *mut c_uchar;
        *byte_buffer = ((*stream).compressed_ungotten & 0xFF) as c_uchar;
        byte_buffer = byte_buffer.offset(1);
        ptr = byte_buffer as *const c_void;

        size -= 1;

        stream.flags &= !DFILE_HAS_COMPRESSED_UNGETC;
        stream.position += 1;

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
            let bytes_to_read = DFILE_DECOMPRESSION_BUFFER_SIZE
                .min(((*(*stream).entry).data_size[0] - (*stream).compressed_bytes_read) as u32)
                as size_t;

            if fread(
                (*stream).decompression_buffer as *mut c_void,
                bytes_to_read,
                1,
                (*stream).stream,
            ) != 1
            {
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
unsafe fn dfile_unget_compressed(stream: &mut DFile, ch: c_int) {
    stream.compressed_ungotten = ch;
    stream.flags |= DFILE_HAS_COMPRESSED_UNGETC;
    stream.position -= 1;
}

// 0x4E5F9C
unsafe fn dfile_read_char_internal(stream: &mut DFile) -> c_int {
    if (*(*stream).entry).compressed[0] == 1 {
        let mut ch = ['\0' as c_char; 1];
        if !dfile_read_compressed(
            stream,
            ch.as_mut_ptr() as *const c_void,
            mem::size_of::<c_char>(),
        ) {
            return -1;
        }

        if ((*stream).flags & DFILE_TEXT as c_int) != 0 {
            // NOTE: I'm not sure if they are comparing as chars or ints. Since
            // character literals are ints, let's cast read characters to int as
            // well.
            if ch[0] == '\r' as c_char {
                let mut next_ch = ['\0' as c_char; 1];
                if dfile_read_compressed(
                    stream,
                    next_ch.as_mut_ptr() as *const c_void,
                    mem::size_of::<c_char>(),
                ) {
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

    if (*(*stream).entry).uncompressed_size[0] < 0
        || (*stream).position >= (*(*stream).entry).uncompressed_size[0] as c_long
    {
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

pub unsafe fn dbase_close(dbase: *mut DBase) -> bool {
    assert_ne!(dbase, null_mut()); // "dbase", "dfile.c", 173

    let mut curr = (*dbase).dfile_head.clone();
    while curr.is_some() {
        let next = curr.as_ref().expect("").clone().borrow().next.clone();
        dfile_close(curr.as_ref().expect("").clone());
        curr = next;
    }

    if (*dbase).entries.is_some() {
        for index in 0..((*dbase).entries_length[0]) {
            let entry = &mut (*dbase).entries.as_mut().expect("")[index as usize];
            if (*entry).path != None {
                (*entry).path = None;
            }
        }
        (*dbase).entries = None
    }

    if (*dbase).path != None {
        (*dbase).path = None;
    }

    memset(dbase as *mut c_void, 0, mem::size_of::<DBase>());

    free(dbase as *mut c_void);

    true
}

pub unsafe fn dbase_open(file_path: *const c_char) -> *mut DBase {
    assert_ne!(file_path, null()); // "filename", "dfile.c", 74

    let rb = CString::new("rb").expect("valid string");
    let str = rb.as_ptr();
    let stream = rust_compat_fopen(file_path, str);
    if stream == null_mut() {
        return null_mut();
    }

    let dbase = malloc(mem::size_of::<DBase>()) as *mut DBase;
    if dbase == null_mut() {
        fclose(stream);
        return null_mut();
    }

    memset(dbase as *mut c_void, 0, mem::size_of_val(&*dbase));

    unsafe fn close_on_error(dbase: *mut DBase, stream: *mut FILE) {
        dbase_close(dbase);
        fclose(stream);
    }

    // Get file size, and reposition stream to read footer, which contains two
    // 32-bits ints.
    let file_size = rust_get_file_size(stream) as c_int;

    if fseek(
        stream,
        (file_size - mem::size_of::<c_int>() as c_int * 2) as c_long,
        SEEK_SET,
    ) != 0
    {
        close_on_error(dbase, stream);
        return null_mut();
    }

    // Read the size of entries table.
    let mut entries_data_size = [0 as c_int; 1];
    if fread(
        entries_data_size.as_mut_ptr() as *mut c_void,
        mem::size_of_val(&entries_data_size),
        1,
        stream,
    ) != 1
    {
        close_on_error(dbase, stream);
        return null_mut();
    }

    // Read the size of entire dbase content.
    //
    // NOTE: It appears that this approach allows existence of arbitrary data in
    // the beginning of the .DAT file.
    let mut dbase_data_size = [0 as c_int; 1];
    if fread(
        dbase_data_size.as_mut_ptr() as *mut c_void,
        mem::size_of_val(&dbase_data_size),
        1,
        stream,
    ) != 1
    {
        close_on_error(dbase, stream);
        return null_mut();
    }

    // Reposition stream to the beginning of the entries table.
    if fseek(
        stream,
        (file_size - entries_data_size[0] as c_int - mem::size_of::<c_int>() as c_int * 2)
            as c_long,
        SEEK_SET,
    ) != 0
    {
        close_on_error(dbase, stream);
        return null_mut();
    }

    if fread(
        (*dbase).entries_length.as_mut_ptr() as *mut c_void,
        mem::size_of_val(&(*dbase).entries_length),
        1,
        stream,
    ) != 1
    {
        close_on_error(dbase, stream);
        return null_mut();
    }

    let entries = Box::new(vec![DBaseEntry::default(); (*dbase).entries_length[0] as usize]);
    (*dbase).entries = Some(*entries);
    if (*dbase).entries.is_none() {
        close_on_error(dbase, stream);
        return null_mut();
    }

    // Read entries one by one, stopping on any error.
    let mut entry_index = 0;
    for i in 0..(*dbase).entries_length[0] {
        let entry = &mut (*dbase).entries.as_mut().expect("")[i as usize];

        let mut path_length = [0 as c_int; 1];
        if fread(
            path_length.as_mut_ptr() as *mut c_void,
            mem::size_of_val(&path_length),
            1,
            stream,
        ) != 1
        {
            break;
        }

        let path = malloc(path_length[0] as size_t + 1) as *mut c_char;
        if path == null_mut() {
            break;
        }

        if fread(path as *mut c_void, path_length[0] as size_t, 1, stream) != 1 {
            break;
        }

        *path.offset(path_length[0] as isize) = '\0' as c_char;
        (*entry).path = Some(CString::from_raw(path));

        if fread(
            (*entry).compressed.as_mut_ptr() as *mut c_void,
            mem::size_of_val(&(*entry).compressed),
            1,
            stream,
        ) != 1
        {
            break;
        }

        if fread(
            (*entry).uncompressed_size.as_mut_ptr() as *mut c_void,
            mem::size_of_val(&(*entry).uncompressed_size),
            1,
            stream,
        ) != 1
        {
            break;
        }

        if fread(
            (*entry).data_size.as_mut_ptr() as *mut c_void,
            mem::size_of_val(&(*entry).data_size),
            1,
            stream,
        ) != 1
        {
            break;
        }

        if fread(
            (*entry).data_offset.as_mut_ptr() as *mut c_void,
            mem::size_of_val(&(*entry).data_offset),
            1,
            stream,
        ) != 1
        {
            break;
        }

        entry_index = i + 1;
    }

    if entry_index < (*dbase).entries_length[0] {
        // We haven't reached the end, which means there was an error while
        // reading entries.
        close_on_error(dbase, stream);
        return null_mut();
    }

    (*dbase).path = Some(CString::from_raw(rust_compat_strdup(file_path)));
    (*dbase).data_offset = file_size as c_int - dbase_data_size[0] as c_int;

    fclose(stream);

    dbase
}

pub unsafe fn dbase_find_first_entry(
    dbase: *const DBase,
    find_file_data: *mut DFileFindData,
    pattern: *const c_char,
) -> bool {
    for index in 0..(*dbase).entries_length[0] {
        let entry = &(*dbase).entries.as_ref().expect("")[index as usize];
        if fpattern_match(pattern, (*entry).get_path_cstr()) {
            strcpy(
                (*find_file_data).file_name.as_mut_ptr() as *mut c_char,
                (*entry).get_path_cstr(),
            );
            strcpy(
                (*find_file_data).pattern.as_mut_ptr() as *mut c_char,
                pattern,
            );
            (*find_file_data).index = index as c_int;
            return true;
        }
    }

    false
}

pub unsafe fn dbase_find_next_entry(
    dbase: *const DBase,
    find_file_data: *mut DFileFindData,
) -> bool {
    for index in ((*find_file_data).index + 1)..(*dbase).entries_length[0] {
        let entry = &(*dbase).entries.as_ref().expect("")[index as usize];
        if fpattern_match(
            (*find_file_data).pattern.as_mut_ptr() as *mut c_char,
            (*entry).get_path_cstr(),
        ) {
            strcpy(
                (*find_file_data).file_name.as_mut_ptr() as *mut c_char,
                (*entry).get_path_cstr(),
            );
            (*find_file_data).index = index;
            return true;
        }
    }

    false
}

pub unsafe fn dfile_read_char(stream: &mut DFile) -> c_int {
    // assert_ne!(stream, null_mut()); // "stream", "dfile.c", 384

    if ((*stream).flags & DFILE_EOF as c_int) != 0 || ((*stream).flags & DFILE_ERROR as c_int) != 0
    {
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

pub unsafe fn dfile_read_string(
    string: *mut c_char,
    mut size: c_int,
    stream: &mut DFile,
) -> *const c_char {
    assert_ne!(string, null_mut()); // "s", "dfile.c", 407
    assert_ne!(size, 0); // "n", "dfile.c", 408
    // assert_ne!(stream, null_mut()); // "stream", "dfile.c", 409

    if ((*stream).flags & DFILE_EOF as c_int) != 0 || ((*stream).flags & DFILE_ERROR as c_int) != 0
    {
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

pub unsafe fn dfile_read(
    mut ptr: *const c_void,
    size: size_t,
    count: size_t,
    stream: &mut DFile,
) -> size_t {
    assert_ne!(ptr, null_mut()); // "ptr", "dfile.c", 499
    // assert_ne!(stream, null_mut()); // "stream", dfile.c, 500

    if ((*stream).flags & DFILE_EOF as c_int) != 0 || ((*stream).flags & DFILE_ERROR as c_int) != 0
    {
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
        bytes_read =
            fread(ptr as *mut c_void, 1, bytes_to_read, (*stream).stream) + extra_bytes_read;
        (*stream).position += bytes_read as c_long;
    }

    bytes_read / size
}

pub unsafe fn dfile_write(
    ptr: *const c_void,
    _size: size_t,
    count: size_t,
    _stream: &DFile,
) -> size_t {
    assert_ne!(ptr, null()); // "ptr", "dfile.c", 538

    count - 1
}

pub unsafe fn dfile_seek(stream: &mut DFile, offset: c_long, origin: c_int) -> c_int {
    // assert_ne!(stream, null_mut()); // "stream", "dfile.c", 569

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
        _ => return 1,
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
                dfile_rewind(stream);
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

    if fseek(
        (*stream).stream,
        ((*(*stream).dbase).data_offset + (*(*stream).entry).data_offset[0]) as c_long,
        SEEK_SET,
    ) != 0
    {
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
    if inflateInit_(
        (*stream).decompression_stream,
        version.as_ptr(),
        mem::size_of::<z_stream>() as c_int,
    ) != Z_OK
    {
        (*stream).flags |= DFILE_ERROR as c_int;
        return 1;
    }

    (*stream).position = 0;
    (*stream).compressed_bytes_read = 0;
    (*stream).flags &= !(DFILE_HAS_UNGETC | DFILE_EOF) as c_int;

    0
}

pub unsafe fn dfile_rewind(stream: &mut DFile) {
    // assert_ne!(stream, null_mut()); // "stream", "dfile.c", 664

    dfile_seek(stream, 0, SEEK_SET);

    (*stream).flags &= !DFILE_ERROR as c_int;
}

pub unsafe fn dfile_print_formatted_args(
    _stream: &DFile,
    format: *const c_char,
    _args: *mut c_void,
) -> c_int {
    assert_ne!(format, null()); // "format", "dfile.c", 369

    -1
}

pub unsafe fn dfile_write_char(_ch: c_int, _stream: &DFile) -> c_int {
    -1
}

pub unsafe fn dfile_write_string(string: *const c_char, _stream: &DFile) -> c_int {
    assert_ne!(string, null()); // "s", "dfile.c", 448
    // assert_ne!(stream, null()); // "stream", "dfile.c", 449

    -1
}

pub unsafe fn dfile_tell(stream: &DFile) -> c_long {
    // assert_ne!(stream, null()); // "stream", "dfile.c", 654

    stream.position
}

pub unsafe fn dfile_eof(stream: &DFile) -> c_int {
    // assert_ne!(stream, null()); // "stream", "dfile.c", 685

    stream.flags & DFILE_EOF as c_int
}

pub unsafe fn dfile_get_size(stream: &DFile) -> c_long {
    (*(*stream).entry).uncompressed_size[0] as c_long
}

pub unsafe fn dbase_find_close(
    _dbase: *const DBase,
    _find_file_data: *const DFileFindData,
) -> bool {
    true
}
