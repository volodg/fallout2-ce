use std::ffi::{c_int, c_void, CString};
use std::mem;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
#[cfg(not(target_family = "windows"))]
use libc::snprintf;
use libc::{atexit, c_char, c_long, c_uint, chdir, fclose, feof, fgetc, FILE, fputc, fputs, fread, free, fseek, ftell, fwrite, getcwd, malloc, memset, rewind, size_t, strcpy, strtok};
use libz_sys::{gzclose, gzeof, gzFile, gzgetc, gzputc, gzputs, gzread, gzrewind, gzseek, gztell, gzwrite, voidp, voidpc, z_off_t};
use vsprintf::vsprintf;
use crate::dfile::{DBase, DFile, dfile_close, rust_dfile_open, dfile_print_formatted_args, dfile_read_char, dfile_read_string, dfile_write_char, dfile_write_string, dfile_read, dfile_write, dfile_seek, dfile_tell, dfile_rewind, dfile_eof, dfile_get_size, dbase_close, rust_dbase_open};
use crate::file_find::DirectoryFileFindData;
use crate::platform_compat::{COMPAT_MAX_DIR, COMPAT_MAX_DRIVE, COMPAT_MAX_PATH, rust_compat_fopen, compat_gzopen, rust_compat_splitpath, compat_gzgets, rust_compat_fgets, rust_get_file_size, rust_compat_mkdir, rust_compat_stricmp, rust_compat_strdup, rust_compat_windows_path_to_native};

#[repr(C)]
#[derive(PartialEq)]
enum XFileType {
    #[allow(dead_code)]
    XfileTypeFile = 0,
    #[allow(dead_code)]
    XfileTypeDfile = 1,
    #[allow(dead_code)]
    XfileTypeGzfile = 2,
}

#[repr(C)]
union XFileTypeUnion {
    file: *mut FILE,
    dfile: *mut DFile,
    gzfile: gzFile,
}

#[repr(C)]
pub struct XFile {
    _type: XFileType,
    file: XFileTypeUnion,
}

// A universal database of files.
#[repr(C)]
pub struct XBase {
    // The path to directory or .DAT file that this xbase represents.
    path: *mut c_char,

    // The [DBase] instance that this xbase represents.
    dbase: *mut DBase,

    // A flag used to denote that this xbase represents .DAT file (true), or
    // a directory (false).
    //
    // NOTE: Original type is 1 byte, likely unsigned char.
    is_dbase: bool,

    // Next [XBase] in linked list.
    next: *mut XBase,
}

#[repr(C)]
pub struct XList {
    file_names_length: c_int,
    file_names: *const *const c_char
}

#[repr(u8)]
enum XFileEnumerationEntryType {
    XFILE_ENUMERATION_ENTRY_TYPE_FILE = 0,
    XFILE_ENUMERATION_ENTRY_TYPE_DIRECTORY = 1,
    XFILE_ENUMERATION_ENTRY_TYPE_DFILE = 2,
}

#[repr(C)]
pub struct XListEnumerationContext {
    name: [c_char; COMPAT_MAX_PATH],
    _type: XFileEnumerationEntryType,
    xlist: *const XList
}

impl Default for XListEnumerationContext {
    fn default() -> Self {
        Self {
            name: [0 as c_char; COMPAT_MAX_PATH],
            _type: XFileEnumerationEntryType::XFILE_ENUMERATION_ENTRY_TYPE_FILE,
            xlist: null()
        }
    }
}

// 0x6B24D0
static G_X_BASE_HEAD: AtomicPtr<XBase> = AtomicPtr::new(null_mut());
static G_X_BASE_EXIT_HANDLER_REGISTERED: AtomicBool = AtomicBool::new(false);

#[cfg(target_family = "windows")]
extern "C" {
    fn snprintf(s: *mut c_char, n: size_t, format: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn rust_get_g_xbase_head() -> *mut XBase {
    G_X_BASE_HEAD.load(Ordering::Relaxed)
}

pub unsafe fn set_g_xbase_head(value: *mut XBase) {
    G_X_BASE_HEAD.store(value, Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn get_g_xbase_exit_handler_registered() -> bool {
    G_X_BASE_EXIT_HANDLER_REGISTERED.load(Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn set_g_xbase_exit_handler_registered(value: bool) {
    G_X_BASE_EXIT_HANDLER_REGISTERED.store(value, Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_close(stream: *mut XFile) -> c_int {
    assert_ne!(stream, null_mut()); // "stream", "xfile.c", 112

    let rc = match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_close((*stream).file.dfile),
        XFileType::XfileTypeGzfile => gzclose((*stream).file.gzfile),
        XFileType::XfileTypeFile => fclose((*stream).file.file),
    };

    memset(stream as *mut c_void, 0, mem::size_of_val(&*stream));

    free(stream as *mut c_void);

    rc
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_open(file_path: *const c_char, mode: *const c_char) -> *const XFile {
    assert_ne!(file_path, null_mut()); // "filename", "xfile.c", 162
    assert_ne!(mode, null_mut()); // "mode", "xfile.c", 163

    let stream = malloc(mem::size_of::<XFile>()) as *mut XFile;
    if stream == null_mut() {
        return null();
    }

    memset(stream as *mut c_void, 0, mem::size_of_val(&*stream));

    // NOTE: Compiled code uses different lengths.
    let mut drive = [0 as c_char; COMPAT_MAX_DRIVE as usize];
    let mut dir = [0 as c_char; COMPAT_MAX_DIR as usize];
    rust_compat_splitpath(file_path, drive.as_mut_ptr(), dir.as_mut_ptr(), null_mut(), null_mut());

    let mut path = [0 as c_char; COMPAT_MAX_PATH];
    let sformat = CString::new("%s").expect("valid string");
    if drive[0] != '\0' as c_char || dir[0] == '\\' as c_char || dir[0] == '/' as c_char || dir[0] == '.' as c_char {
        // [filePath] is an absolute path. Attempt to open as plain stream.
        (*stream).file.file = rust_compat_fopen(file_path, mode);
        if (*stream).file.file == null_mut() {
            free(stream as *mut c_void);
            return null();
        }

        (*stream)._type = XFileType::XfileTypeFile;
        snprintf(path.as_mut_ptr(), mem::size_of_val(&path), sformat.as_ptr(), file_path);
    } else {
        // [filePath] is a relative path. Loop thru open xbases and attempt to
        // open [filePath] from appropriate xbase.
        let mut curr = G_X_BASE_HEAD.load(Ordering::Relaxed);
        let sformat_sformat = CString::new("%s\\%s").expect("valid string");
        while curr != null_mut() {
            if (*curr).is_dbase {
                // Attempt to open dfile stream from dbase.
                (*stream).file.dfile = rust_dfile_open((*curr).dbase, file_path, mode);
                if (*stream).file.dfile != null_mut() {
                    (*stream)._type = XFileType::XfileTypeDfile;
                    snprintf(path.as_mut_ptr(), mem::size_of_val(&path), sformat.as_ptr(), file_path);
                    break;
                }
            } else {
                // Build path relative to directory-based xbase.
                snprintf(path.as_mut_ptr(), mem::size_of_val(&path), sformat_sformat.as_ptr(), (*curr).path, file_path);

                // Attempt to open plain stream.
                (*stream).file.file = rust_compat_fopen(path.as_ptr(), mode);
                if (*stream).file.file != null_mut() {
                    (*stream)._type = XFileType::XfileTypeFile;
                    break;
                }
            }
            curr = (*curr).next;
        }

        if (*stream).file.file == null_mut() {
            // File was not opened during the loop above. Attempt to open file
            // relative to the current working directory.
            (*stream).file.file = rust_compat_fopen(file_path, mode);
            if (*stream).file.file == null_mut() {
                free(stream as *mut c_void);
                return null();
            }

            (*stream)._type = XFileType::XfileTypeFile;
            snprintf(path.as_mut_ptr(), mem::size_of_val(&path), sformat.as_ptr(), file_path);
        }
    }

    if (*stream)._type == XFileType::XfileTypeFile {
        // Opened file is a plain stream, which might be gzipped. In this case
        // first two bytes will contain magic numbers.
        let ch1 = fgetc((*stream).file.file);
        let ch2 = fgetc((*stream).file.file);
        if ch1 == 0x1F && ch2 == 0x8B {
            // File is gzipped. Close plain stream and reopen this file as
            // gzipped stream.
            fclose((*stream).file.file);

            (*stream)._type = XFileType::XfileTypeGzfile;
            (*stream).file.gzfile = compat_gzopen(path.as_ptr(), mode);
        } else {
            // File is not gzipped.
            rewind((*stream).file.file);
        }
    }

    stream
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_print_formatted_args(stream: *const XFile, format: *const c_char, args: *mut c_void) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 332
    assert_ne!(format, null()); // "format", "xfile.c", 333

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_print_formatted_args((*stream).file.dfile, format, args),
        XFileType::XfileTypeGzfile => {
            let str = vsprintf(format, args).expect("valid");
            gzwrite((*stream).file.gzfile, str.as_ptr() as voidpc, str.len() as c_uint)
        },
        XFileType::XfileTypeFile => {
            let str = vsprintf(format, args).expect("valid");
            fwrite(str.as_ptr() as *const c_void, str.len() as size_t, 1, (*stream).file.file) as c_int
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_read_char(stream: *const XFile) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 354

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_read_char((*stream).file.dfile),
        XFileType::XfileTypeGzfile => gzgetc((*stream).file.gzfile),
        XFileType::XfileTypeFile => fgetc((*stream).file.file)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_read_string(string: *mut c_char, size: c_int, stream: *const XFile) -> *const c_char {
    assert_ne!(string, null_mut()); // "s", "xfile.c", 375
    assert_ne!(size, 0); // "n", "xfile.c", 376
    assert_ne!(stream, null()); // "stream", "xfile.c", 377

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_read_string(string, size, (*stream).file.dfile),
        XFileType::XfileTypeGzfile => compat_gzgets((*stream).file.gzfile, string, size),
        XFileType::XfileTypeFile => rust_compat_fgets(string, size, (*stream).file.file)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_write_char(ch: c_int, stream: *const XFile) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 399

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_write_char(ch, (*stream).file.dfile),
        XFileType::XfileTypeGzfile => gzputc((*stream).file.gzfile, ch),
        XFileType::XfileTypeFile => fputc(ch, (*stream).file.file)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_write_string(string: *const c_char, stream: *const XFile) -> c_int {
    assert_ne!(string, null()); // "s", "xfile.c", 421
    assert_ne!(stream, null()); // "stream", "xfile.c", 422

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_write_string(string, (*stream).file.dfile),
        XFileType::XfileTypeGzfile => gzputs((*stream).file.gzfile, string),
        XFileType::XfileTypeFile => fputs(string, (*stream).file.file),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_read(ptr: *mut c_void, size: size_t, count: size_t, stream: *const XFile) -> size_t {
    assert_ne!(ptr, null_mut()); // "ptr", "xfile.c", 421
    assert_ne!(stream, null()); // "stream", "xfile.c", 422

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_read(ptr, size, count, (*stream).file.dfile),
        XFileType::XfileTypeGzfile => gzread((*stream).file.gzfile, ptr as voidp, (size * count) as c_uint) as size_t,
        XFileType::XfileTypeFile => fread(ptr, size, count, (*stream).file.file)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_write(ptr: *const c_void, size: size_t, count: size_t, stream: *const XFile) -> size_t {
    assert_ne!(ptr, null()); // "ptr", "xfile.c", 504
    assert_ne!(stream, null()); // "stream", "xfile.c", 505

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_write(ptr, size, count, (*stream).file.dfile),
        XFileType::XfileTypeGzfile => gzwrite((*stream).file.gzfile, ptr, (size * count) as c_uint) as size_t,
        XFileType::XfileTypeFile => fwrite(ptr, size, count, (*stream).file.file)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_seek(stream: *const XFile, offset: c_long, origin: c_int) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 547

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_seek((*stream).file.dfile, offset, origin),
        XFileType::XfileTypeGzfile => gzseek((*stream).file.gzfile, offset as z_off_t, origin) as c_int,
        XFileType::XfileTypeFile => fseek((*stream).file.file, offset, origin)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_tell(stream: *const XFile) -> c_long {
    assert_ne!(stream, null()); // "stream", "xfile.c", 588

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_tell((*stream).file.dfile),
        XFileType::XfileTypeGzfile => gztell((*stream).file.gzfile) as c_long,
        XFileType::XfileTypeFile => ftell((*stream).file.file)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_rewind(stream: *const XFile) {
    assert_ne!(stream, null()); // "stream", "xfile.c", 608

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_rewind((*stream).file.dfile),
        XFileType::XfileTypeGzfile => {
            gzrewind((*stream).file.gzfile);
        },
        XFileType::XfileTypeFile => rewind((*stream).file.file)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_eof(stream: *const XFile) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 648

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_eof((*stream).file.dfile),
        XFileType::XfileTypeGzfile => gzeof((*stream).file.gzfile),
        XFileType::XfileTypeFile => feof((*stream).file.file)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_get_size(stream: *const XFile) -> c_long {
    assert_ne!(stream, null()); // "stream", "xfile.c", 690

    match (*stream)._type {
        XFileType::XfileTypeDfile => dfile_get_size((*stream).file.dfile),
        XFileType::XfileTypeGzfile => 0,
        XFileType::XfileTypeFile => rust_get_file_size((*stream).file.file)
    }
}

// Closes all xbases.
extern "C" fn xbase_close_all() {
    unsafe {
        let mut curr = rust_get_g_xbase_head();
        set_g_xbase_head(null_mut());

        while curr != null_mut() {
            let next = (*curr).next;

            if (*curr).is_dbase {
                dbase_close((*curr).dbase);
            }

            free((*curr).path as *mut c_void);
            free(curr as *mut c_void);

            curr = next;
        }
    }
}

#[cfg(target_family = "windows")]
type GETCWD_SIZE = c_int;

#[cfg(not(target_family = "windows"))]
type GETCWD_SIZE = size_t;

// Recursively creates specified file path.
pub unsafe fn xbase_make_directory(file_path: *mut c_char) -> c_int {
    let mut working_directory = [0 as c_char; COMPAT_MAX_PATH];
    if getcwd(working_directory.as_mut_ptr(), COMPAT_MAX_PATH as GETCWD_SIZE) == null_mut() {
        return -1;
    }

    let mut drive = [0 as c_char; COMPAT_MAX_DRIVE as usize];
    let mut dir = [0 as c_char; COMPAT_MAX_DIR as usize];
    rust_compat_splitpath(file_path, drive.as_mut_ptr(), dir.as_mut_ptr(), null_mut(), null_mut());

    let mut path = [0 as c_char; COMPAT_MAX_PATH];
    let sformat_sformat = CString::new("%s\\%s").expect("valid string");

    if drive[0] != '\0' as c_char || dir[0] == '\\' as c_char || dir[0] == '/' as c_char || dir[0] == '.' as c_char {
        // [filePath] is an absolute path.
        strcpy(path.as_mut_ptr(), file_path);
    } else {
        // Find first directory-based xbase.
        let mut curr = rust_get_g_xbase_head();
        while curr != null_mut() {
            if !(*curr).is_dbase {
                snprintf(path.as_mut_ptr(), mem::size_of_val(&path), sformat_sformat.as_ptr(), (*curr).path, file_path);
                break;
            }
            curr = (*curr).next;
        }

        if curr == null_mut() {
            // Either there are no directory-based xbase, or there are no open
            // xbases at all - resolve path against current working directory.
            snprintf(path.as_mut_ptr(), mem::size_of_val(&path), sformat_sformat.as_ptr(), working_directory, file_path);
        }
    }

    let mut pch = path.as_mut_ptr();

    if *pch == '\\' as c_char || *pch == '/' as c_char {
        pch = pch.offset(1);
    }

    while *pch != '\0' as c_char {
        if *pch == '\\' as c_char || *pch == '/' as c_char {
            let temp = *pch;
            *pch = '\0' as c_char;

            if chdir(path.as_ptr()) != 0 {
                if rust_compat_mkdir(path.as_ptr()) != 0 {
                    chdir(working_directory.as_mut_ptr());
                    return -1;
                }
            } else {
                chdir(working_directory.as_ptr());
            }

            *pch = temp;
        }
        pch = pch.offset(1);
    }

    // Last path component.
    rust_compat_mkdir(path.as_ptr());

    chdir(working_directory.as_ptr());

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_xbase_open(path: *mut c_char) -> bool {
    assert_ne!(path, null_mut()); // "path", "xfile.c", 747

    // Register atexit handler so that underlying dbase (if any) can be
    // gracefully closed.
    if !get_g_xbase_exit_handler_registered() {
        atexit(xbase_close_all);
        set_g_xbase_exit_handler_registered(true);
    }

    let mut curr = rust_get_g_xbase_head();
    let mut prev = null_mut();
    while curr != null_mut() {
        if rust_compat_stricmp(path, (*curr).path) == 0 {
            break;
        }

        prev = curr;
        curr = (*curr).next;
    }

    if curr != null_mut() {
        if prev != null_mut() {
            // Move found xbase to the top.
            (*prev).next = (*curr).next;
            (*curr).next = rust_get_g_xbase_head();
            set_g_xbase_head(curr);
        }
        return true;
    }

    let xbase = malloc(mem::size_of::<XBase>()) as *mut XBase;
    if xbase == null_mut() {
        return false;
    }

    memset(xbase as *mut c_void, 0, mem::size_of::<XBase>());

    (*xbase).path = rust_compat_strdup(path);
    if (*xbase).path == null_mut() {
        free(xbase as *mut c_void);
        return false;
    }

    let dbase = rust_dbase_open(path);
    if dbase != null_mut() {
        (*xbase).is_dbase = true;
        (*xbase).dbase = dbase;
        (*xbase).next = rust_get_g_xbase_head();
        set_g_xbase_head(xbase);
        return true;
    }

    let mut working_directory = [0 as c_char; COMPAT_MAX_PATH];
    if getcwd(working_directory.as_mut_ptr(), COMPAT_MAX_PATH as GETCWD_SIZE) == null_mut() {
        // FIXME: Leaking xbase and path.
        return false;
    }

    if chdir(path) == 0 {
        chdir(working_directory.as_ptr());
        (*xbase).next = rust_get_g_xbase_head();
        set_g_xbase_head(xbase);
        return true;
    }

    if xbase_make_directory(path) != 0 {
        // FIXME: Leaking xbase and path.
        return false;
    }

    chdir(working_directory.as_ptr());

    (*xbase).next = rust_get_g_xbase_head();
    set_g_xbase_head(xbase);

    true
}

// Closes all open xbases and opens a set of xbases specified by [paths].
//
// [paths] is a set of paths separated by semicolon. Can be NULL, in this case
// all open xbases are simply closed.
//
// 0x4DF878
#[no_mangle]
pub unsafe extern "C" fn rust_xbase_reopen_all(paths: *mut c_char) -> bool {
    // NOTE: Uninline.
    xbase_close_all();

    let delimiter = CString::new(";").expect("valid string");
    if paths != null_mut() {
        let mut tok = strtok(paths, delimiter.as_ptr());
        while tok != null_mut() {
            if !rust_xbase_open(tok) {
                return false;
            }
            tok = strtok(null_mut(), delimiter.as_ptr());
        }
    }

    true
}

// typedef bool(XListEnumerationHandler)(XListEnumerationContext* context);
#[no_mangle]
pub unsafe extern "C" fn rust_xlist_enumerate(
    pattern: *const c_char,
    handler: extern "C" fn(*const XListEnumerationContext),
    xlist: *const XList) -> bool {

    assert_ne!(pattern, null()); // "filespec", "xfile.c", 845
    assert_ne!(handler, mem::transmute::<*const c_void, extern "C" fn(*const XListEnumerationContext)>(null())); // "enumfunc", "xfile.c", 846

    let mut directory_file_find_data = DirectoryFileFindData::default();
    let mut context = XListEnumerationContext::default();

    context.xlist = xlist;

    let mut native_pattern = [0 as c_char; COMPAT_MAX_PATH];
    strcpy(native_pattern.as_mut_ptr(), pattern);
    rust_compat_windows_path_to_native(native_pattern.as_mut_ptr());

    /*
    let mut drive = [0 as c_char; COMPAT_MAX_DRIVE as usize];
    let mut dir = [0 as c_char; COMPAT_MAX_DIR as usize];
    let mut file_name = [0 as c_char; COMPAT_MAX_FNAME as usize];
    let mut extension = [0 as c_char; COMPAT_MAX_EXT as usize];
    rust_compat_splitpath(native_pattern.as_mut_ptr(), drive.as_mut_ptr(), dir.as_mut_ptr(), file_name.as_mut_ptr(), extension.as_mut_ptr());
    if drive[0] != '\0' as c_char || dir[0] == '\\' as c_char || dir[0] == '/' as c_char || dir[0] == '.' as c_char {
        if rust_file_find_first(native_pattern.as_ptr(), &mut directory_file_find_data) {
            loop {
                let is_directory = fileFindIsDirectory(&directory_file_find_data);
                let entry_name = fileFindGetName(&directory_file_find_data);

                if is_directory {
                    let dot_dot = CString::new("..").expect("valid string");
                    let dot = CString::new(".").expect("valid string");
                    if strcmp(entry_name, dot_dot.as_ptr()) == 0 || strcmp(entry_name, dot.as_ptr()) == 0 {
                        continue;
                    }

                    context._type = XFileEnumerationEntryType::XFILE_ENUMERATION_ENTRY_TYPE_DIRECTORY;
                } else {
                    context._type = XFileEnumerationEntryType::XFILE_ENUMERATION_ENTRY_TYPE_FILE;
                }

                rust_compat_makepath(context.name.as_mut_ptr(), drive.as_ptr(), dir.as_ptr(), entry_name, null());

                if !handler(&context) {
                    break;
                }

                if !rust_file_find_next(&mut directory_file_find_data) {
                    break;
                }
            }
        }
        return rust_file_find_close(&mut directory_file_find_data);
    }

    XBase* xbase = rust_get_g_xbase_head();
    while (xbase != nullptr) {
        if (xbase->isDbase) {
            DFileFindData dbaseFindData;
            if (dbaseFindFirstEntry(xbase->dbase, &dbaseFindData, pattern)) {
                context.type = XFILE_ENUMERATION_ENTRY_TYPE_DFILE;

                do {
                    strcpy(context.name, dbaseFindData.fileName);
                    if (!handler(&context)) {
                    return dbaseFindClose(xbase->dbase, &dbaseFindData);
                    }
                } while (dbaseFindNextEntry(xbase->dbase, &dbaseFindData));

                dbaseFindClose(xbase->dbase, &dbaseFindData);
            }
        } else {
            char path[COMPAT_MAX_PATH];
            snprintf(path, sizeof(path), "%s\\%s", xbase->path, pattern);
            compat_windows_path_to_native(path);

            if (fileFindFirst(path, &directory_file_find_data)) {
                do {
                    bool isDirectory = fileFindIsDirectory(&directory_file_find_data);
                    char* entryName = fileFindGetName(&directory_file_find_data);

                    if (isDirectory) {
                    if (strcmp(entryName, "..") == 0 || strcmp(entryName, ".") == 0) {
                    continue;
                    }

                    context.type = XFILE_ENUMERATION_ENTRY_TYPE_DIRECTORY;
                    } else {
                    context.type = XFILE_ENUMERATION_ENTRY_TYPE_FILE;
                    }

                    compat_makepath(context.name, drive, dir, entryName, nullptr);

                    if (!handler(&context)) {
                    break;
                    }
                } while (fileFindNext(&directory_file_find_data));
            }
            findFindClose(&directory_file_find_data);
        }
        xbase = xbase->next;
    }

    compat_splitpath(nativePattern, drive, dir, fileName, extension);
    if (fileFindFirst(nativePattern, &directory_file_find_data)) {
        do {
            bool isDirectory = fileFindIsDirectory(&directory_file_find_data);
            char* entryName = fileFindGetName(&directory_file_find_data);

            if (isDirectory) {
            if (strcmp(entryName, "..") == 0 || strcmp(entryName, ".") == 0) {
            continue;
            }

            context.type = XFILE_ENUMERATION_ENTRY_TYPE_DIRECTORY;
            } else {
            context.type = XFILE_ENUMERATION_ENTRY_TYPE_FILE;
            }

            compat_makepath(context.name, drive, dir, entryName, nullptr);

            if (!handler(&context)) {
            break;
            }
        } while (fileFindNext(&directory_file_find_data));
    }

    findFindClose(&directory_file_find_data)*/
    true
}

/*
static bool xlistEnumerate(const char* pattern, XListEnumerationHandler* handler, XList* xlist)
{
}
 */
