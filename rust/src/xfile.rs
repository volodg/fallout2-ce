use crate::dfile::{
    dbase_find_close, dbase_find_first_entry, dbase_find_next_entry, dbase_open, dfile_eof,
    dfile_get_size, dfile_print_formatted_args, dfile_read, dfile_read_char, dfile_read_string,
    dfile_remove_node, dfile_rewind, dfile_seek, dfile_tell, dfile_write, dfile_write_char,
    dfile_write_string, rust_dfile_open, DBase, DFile, DFileFindData,
};
use crate::file_find::{
    file_find_close, file_find_first, file_find_get_name, file_find_is_directory, file_find_next,
    DirectoryFileFindData,
};
use crate::platform_compat::{
    compat_gzgets, compat_gzopen, rust_compat_fgets, rust_compat_fopen, rust_compat_makepath,
    rust_compat_mkdir, rust_compat_splitpath, rust_compat_strdup, rust_compat_stricmp,
    rust_compat_windows_path_to_native, rust_get_file_size, COMPAT_MAX_DIR, COMPAT_MAX_DRIVE,
    COMPAT_MAX_EXT, COMPAT_MAX_FNAME, COMPAT_MAX_PATH,
};
#[cfg(not(target_family = "windows"))]
use libc::snprintf;
use libc::{
    atexit, c_char, c_long, c_uint, chdir, fclose, feof, fgetc, fputc, fputs, fread, free, fseek,
    ftell, fwrite, getcwd, memset, realloc, rewind, size_t, strcmp, strcpy, strtok, FILE,
};
use libz_sys::{
    gzFile, gzclose, gzeof, gzgetc, gzputc, gzputs, gzread, gzrewind, gzseek, gztell, gzwrite,
    voidp, voidpc, z_off_t,
};
use spin::RwLock;
use std::cell::RefCell;
use std::ffi::{c_int, c_void, CString};
use std::mem;
use std::ptr::{null, null_mut};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vsprintf::vsprintf;

enum XFileType {
    File(*mut FILE),
    DFile(Rc<RefCell<DFile>>),
    GZFile(gzFile),
}

pub struct XFile {
    file: XFileType,
}

impl Default for XFile {
    fn default() -> Self {
        Self {
            file: XFileType::File(null_mut()),
        }
    }
}

// A universal database of files.
pub struct XBase {
    // The path to directory or .DAT file that this xbase represents.
    path: Option<CString>,

    // The [DBase] instance that this xbase represents.
    dbase: Option<Rc<RefCell<DBase>>>,

    // A flag used to denote that this xbase represents .DAT file (true), or
    // a directory (false).
    //
    // NOTE: Original type is 1 byte, likely unsigned char.
    is_dbase: bool,

    // Next [XBase] in linked list.
    next: Option<Arc<RwLock<XBase>>>,
}

impl Default for XBase {
    fn default() -> Self {
        Self {
            path: None,
            dbase: None,
            is_dbase: false,
            next: None,
        }
    }
}

unsafe impl Send for XBase {}

unsafe impl Sync for XBase {}

impl XBase {
    fn get_path_cstr(&self) -> *const c_char {
        self.path.as_ref().map(|x| x.as_ptr()).unwrap_or(null())
    }
}

#[repr(C)]
pub struct XList {
    pub file_names_length: c_int,
    pub file_names: *mut *mut c_char,
}

#[derive(PartialEq)]
enum XFileEnumerationEntryType {
    XfileEnumerationEntryTypeFile = 0,
    XfileEnumerationEntryTypeDirectory = 1,
    XfileEnumerationEntryTypeDfile = 2,
}

struct XListEnumerationContext {
    name: [c_char; COMPAT_MAX_PATH],
    _type: XFileEnumerationEntryType,
    xlist: *mut XList,
}

impl Default for XListEnumerationContext {
    fn default() -> Self {
        Self {
            name: [0 as c_char; COMPAT_MAX_PATH],
            _type: XFileEnumerationEntryType::XfileEnumerationEntryTypeFile,
            xlist: null_mut(),
        }
    }
}

// 0x6B24D0
static G_X_BASE_HEAD: RwLock<Option<Arc<RwLock<XBase>>>> = RwLock::new(None);
static G_X_BASE_EXIT_HANDLER_REGISTERED: AtomicBool = AtomicBool::new(false);

#[cfg(target_family = "windows")]
extern "C" {
    fn snprintf(s: *mut c_char, n: size_t, format: *const c_char, ...) -> c_int;
}

pub fn get_g_xbase_head_rc() -> Option<Arc<RwLock<XBase>>> {
    let read_binding = G_X_BASE_HEAD.read();
    read_binding.clone()
}

pub fn set_g_xbase_head(value: Option<Arc<RwLock<XBase>>>) {
    let mut lock = G_X_BASE_HEAD.write();
    *lock = value;
}

pub fn get_g_xbase_exit_handler_registered() -> bool {
    G_X_BASE_EXIT_HANDLER_REGISTERED.load(Ordering::Relaxed)
}

pub fn set_g_xbase_exit_handler_registered(value: bool) {
    G_X_BASE_EXIT_HANDLER_REGISTERED.store(value, Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_close(stream: *mut XFile) -> c_int {
    assert_ne!(stream, null_mut()); // "stream", "xfile.c", 112

    let stream = Box::from_raw(stream);

    let rc = match (*stream).file {
        XFileType::DFile(file) => dfile_remove_node(&file.borrow()),
        XFileType::GZFile(file) => gzclose(file),
        XFileType::File(file) => fclose(file),
    };

    rc
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_open(
    file_path: *const c_char,
    mode: *const c_char,
) -> *mut XFile {
    assert_ne!(file_path, null()); // "filename", "xfile.c", 162
    assert_ne!(mode, null()); // "mode", "xfile.c", 163

    let mut stream = Box::new(XFile::default());

    // NOTE: Compiled code uses different lengths.
    let mut drive = [0 as c_char; COMPAT_MAX_DRIVE as usize];
    let mut dir = [0 as c_char; COMPAT_MAX_DIR as usize];
    rust_compat_splitpath(
        file_path,
        drive.as_mut_ptr(),
        dir.as_mut_ptr(),
        null_mut(),
        null_mut(),
    );

    let mut path = [0 as c_char; COMPAT_MAX_PATH];
    let sformat = CString::new("%s").expect("valid string");
    if drive[0] != '\0' as c_char
        || dir[0] == '\\' as c_char
        || dir[0] == '/' as c_char
        || dir[0] == '.' as c_char
    {
        // [filePath] is an absolute path. Attempt to open as plain stream.
        let file = rust_compat_fopen(file_path, mode);
        if file == null_mut() {
            return null_mut();
        }

        (*stream).file = XFileType::File(file);

        snprintf(
            path.as_mut_ptr(),
            mem::size_of_val(&path),
            sformat.as_ptr(),
            file_path,
        );
    } else {
        // [filePath] is a relative path. Loop thru open xbases and attempt to
        // open [filePath] from appropriate xbase.
        let mut optional_curr = get_g_xbase_head_rc();
        let sformat_sformat = CString::new("%s\\%s").expect("valid string");
        while let Some(curr) = optional_curr {
            let curr = curr.read();
            if curr.is_dbase {
                // Attempt to open dfile stream from dbase.
                let optional_dfile =
                    rust_dfile_open(&curr.dbase.as_ref().expect(""), file_path, mode);
                if let Some(dfile) = optional_dfile {
                    (*stream).file = XFileType::DFile(dfile);
                    snprintf(
                        path.as_mut_ptr(),
                        mem::size_of_val(&path),
                        sformat.as_ptr(),
                        file_path,
                    );
                    break;
                }
            } else {
                // Build path relative to directory-based xbase.
                snprintf(
                    path.as_mut_ptr(),
                    mem::size_of_val(&path),
                    sformat_sformat.as_ptr(),
                    curr.get_path_cstr(),
                    file_path,
                );

                // Attempt to open plain stream.
                let file = rust_compat_fopen(path.as_ptr(), mode);
                if file != null_mut() {
                    (*stream).file = XFileType::File(rust_compat_fopen(path.as_ptr(), mode));
                    break;
                }
            }
            optional_curr = curr.next.clone();
        }

        match (*stream).file {
            XFileType::File(file) if file == null_mut() => {
                // File was not opened during the loop above. Attempt to open file
                // relative to the current working directory.
                let file = rust_compat_fopen(file_path, mode);
                if file == null_mut() {
                    return null_mut();
                }
                (*stream).file = XFileType::File(file);

                snprintf(
                    path.as_mut_ptr(),
                    mem::size_of_val(&path),
                    sformat.as_ptr(),
                    file_path,
                );
            }
            _ => (),
        }
    }

    match (*stream).file {
        XFileType::File(file) => {
            // Opened file is a plain stream, which might be gzipped. In this case
            // first two bytes will contain magic numbers.
            let ch1 = fgetc(file);
            let ch2 = fgetc(file);
            if ch1 == 0x1F && ch2 == 0x8B {
                // File is gzipped. Close plain stream and reopen this file as
                // gzipped stream.
                fclose(file);

                (*stream).file = XFileType::GZFile(compat_gzopen(path.as_ptr(), mode));
            } else {
                // File is not gzipped.
                rewind(file);
            }
        }
        _ => (),
    }

    Box::into_raw(stream)
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_print_formatted_args(
    stream: *const XFile,
    format: *const c_char,
    args: *mut c_void,
) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 332
    assert_ne!(format, null()); // "format", "xfile.c", 333

    match &(*stream).file {
        XFileType::DFile(file) => dfile_print_formatted_args(&file.borrow(), format, args),
        XFileType::GZFile(file) => {
            let str = vsprintf(format, args).expect("valid");
            gzwrite(file.clone(), str.as_ptr() as voidpc, str.len() as c_uint)
        }
        XFileType::File(file) => {
            let str = vsprintf(format, args).expect("valid");
            fwrite(str.as_ptr() as *const c_void, str.len() as size_t, 1, *file) as c_int
        }
    }
}

pub unsafe fn xfile_read_char(stream: *const XFile) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 354

    match &(*stream).file {
        XFileType::DFile(file) => dfile_read_char(&mut file.borrow_mut()),
        XFileType::GZFile(file) => gzgetc(file.clone()),
        XFileType::File(file) => fgetc(*file),
    }
}

pub unsafe fn xfile_read_string(
    string: *mut c_char,
    size: c_int,
    stream: *const XFile,
) -> *const c_char {
    assert_ne!(string, null_mut()); // "s", "xfile.c", 375
    assert_ne!(size, 0); // "n", "xfile.c", 376
    assert_ne!(stream, null()); // "stream", "xfile.c", 377

    match &(*stream).file {
        XFileType::DFile(file) => dfile_read_string(string, size, &mut file.borrow_mut()),
        XFileType::GZFile(file) => compat_gzgets(file.clone(), string, size),
        XFileType::File(file) => rust_compat_fgets(string, size, *file),
    }
}

pub unsafe fn xfile_write_char(ch: c_int, stream: *const XFile) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 399

    match &(*stream).file {
        XFileType::DFile(file) => dfile_write_char(ch, &file.borrow()),
        XFileType::GZFile(file) => gzputc(file.clone(), ch),
        XFileType::File(file) => fputc(ch, *file),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_write_string(
    string: *const c_char,
    stream: *const XFile,
) -> c_int {
    assert_ne!(string, null()); // "s", "xfile.c", 421
    assert_ne!(stream, null()); // "stream", "xfile.c", 422

    match &(*stream).file {
        XFileType::DFile(file) => dfile_write_string(string, &file.borrow()),
        XFileType::GZFile(file) => gzputs(file.clone(), string),
        XFileType::File(file) => fputs(string, *file),
    }
}

pub unsafe fn xfile_read(
    ptr: *mut c_void,
    size: size_t,
    count: size_t,
    stream: *const XFile,
) -> size_t {
    assert_ne!(ptr, null_mut()); // "ptr", "xfile.c", 421
    assert_ne!(stream, null()); // "stream", "xfile.c", 422

    match &(*stream).file {
        XFileType::DFile(file) => dfile_read(ptr, size, count, &mut file.borrow_mut()),
        XFileType::GZFile(file) => {
            gzread(file.clone(), ptr as voidp, (size * count) as c_uint) as size_t
        }
        XFileType::File(file) => fread(ptr, size, count, *file),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_write(
    ptr: *const c_void,
    size: size_t,
    count: size_t,
    stream: *const XFile,
) -> size_t {
    assert_ne!(ptr, null()); // "ptr", "xfile.c", 504
    assert_ne!(stream, null()); // "stream", "xfile.c", 505

    match &(*stream).file {
        XFileType::DFile(file) => dfile_write(ptr, size, count, &file.borrow()),
        XFileType::GZFile(file) => gzwrite(file.clone(), ptr, (size * count) as c_uint) as size_t,
        XFileType::File(file) => fwrite(ptr, size, count, *file),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_seek(
    stream: *const XFile,
    offset: c_long,
    origin: c_int,
) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 547

    match &(*stream).file {
        XFileType::DFile(file) => dfile_seek(&mut file.borrow_mut(), offset, origin),
        XFileType::GZFile(file) => gzseek(file.clone(), offset as z_off_t, origin) as c_int,
        XFileType::File(file) => fseek(*file, offset, origin),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_tell(stream: *const XFile) -> c_long {
    assert_ne!(stream, null()); // "stream", "xfile.c", 588

    match &(*stream).file {
        XFileType::DFile(file) => dfile_tell(&file.borrow()),
        XFileType::GZFile(file) => gztell(file.clone()) as c_long,
        XFileType::File(file) => ftell(*file),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_rewind(stream: *const XFile) {
    assert_ne!(stream, null()); // "stream", "xfile.c", 608

    match &(*stream).file {
        XFileType::DFile(file) => dfile_rewind(&mut file.borrow_mut()),
        XFileType::GZFile(file) => {
            gzrewind(file.clone());
        }
        XFileType::File(file) => rewind(*file),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_eof(stream: *const XFile) -> c_int {
    assert_ne!(stream, null()); // "stream", "xfile.c", 648

    match &(*stream).file {
        XFileType::DFile(file) => dfile_eof(&file.borrow()),
        XFileType::GZFile(file) => gzeof(file.clone()),
        XFileType::File(file) => feof(*file),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rust_xfile_get_size(stream: *const XFile) -> c_long {
    assert_ne!(stream, null()); // "stream", "xfile.c", 690

    match &(*stream).file {
        XFileType::DFile(file) => dfile_get_size(&file.borrow()),
        XFileType::GZFile(_) => 0,
        XFileType::File(file) => rust_get_file_size(*file),
    }
}

// Closes all xbases.
extern "C" fn xbase_close_all() {
    set_g_xbase_head(None);
}

#[cfg(target_family = "windows")]
type GetcwdSize = c_int;

#[cfg(not(target_family = "windows"))]
type GetcwdSize = size_t;

// Recursively creates specified file path.
pub unsafe fn xbase_make_directory(file_path: *mut c_char) -> c_int {
    let mut working_directory = [0 as c_char; COMPAT_MAX_PATH];
    if getcwd(
        working_directory.as_mut_ptr(),
        COMPAT_MAX_PATH as GetcwdSize,
    ) == null_mut()
    {
        return -1;
    }

    let mut drive = [0 as c_char; COMPAT_MAX_DRIVE as usize];
    let mut dir = [0 as c_char; COMPAT_MAX_DIR as usize];
    rust_compat_splitpath(
        file_path,
        drive.as_mut_ptr(),
        dir.as_mut_ptr(),
        null_mut(),
        null_mut(),
    );

    let mut path = [0 as c_char; COMPAT_MAX_PATH];
    let sformat_sformat = CString::new("%s\\%s").expect("valid string");

    if drive[0] != '\0' as c_char
        || dir[0] == '\\' as c_char
        || dir[0] == '/' as c_char
        || dir[0] == '.' as c_char
    {
        // [filePath] is an absolute path.
        strcpy(path.as_mut_ptr(), file_path);
    } else {
        // Find first directory-based xbase.
        let mut optional_curr = get_g_xbase_head_rc();
        while let Some(curr) = optional_curr.clone() {
            let curr = curr.read();
            if !curr.is_dbase {
                snprintf(
                    path.as_mut_ptr(),
                    mem::size_of_val(&path),
                    sformat_sformat.as_ptr(),
                    curr.get_path_cstr(),
                    file_path,
                );
                break;
            }
            optional_curr = curr.next.clone();
        }

        if optional_curr.is_none() {
            // Either there are no directory-based xbase, or there are no open
            // xbases at all - resolve path against current working directory.
            snprintf(
                path.as_mut_ptr(),
                mem::size_of_val(&path),
                sformat_sformat.as_ptr(),
                working_directory,
                file_path,
            );
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

pub unsafe fn xbase_open(path: *mut c_char) -> bool {
    assert_ne!(path, null_mut()); // "path", "xfile.c", 747

    // Register atexit handler so that underlying dbase (if any) can be
    // gracefully closed.
    if !get_g_xbase_exit_handler_registered() {
        atexit(xbase_close_all);
        set_g_xbase_exit_handler_registered(true);
    }

    let mut optional_curr = get_g_xbase_head_rc();
    let mut optional_prev = None;
    while let Some(curr) = optional_curr.clone() {
        let curr = curr.read();
        if rust_compat_stricmp(path, curr.get_path_cstr()) == 0 {
            break;
        }

        optional_prev = optional_curr.clone();
        optional_curr = curr.next.clone();
    }

    if let Some(curr) = optional_curr {
        if let Some(prev) = optional_prev {
            // Move found xbase to the top.
            prev.write().next = curr.read().next.clone();
            curr.write().next = get_g_xbase_head_rc();
            set_g_xbase_head(Some(curr));
        }
        return true;
    }

    let mut xbase = XBase::default();

    xbase.path = Some(CString::from_raw(rust_compat_strdup(path)));
    if xbase.path == None {
        return false;
    }

    let dbase = dbase_open(path);
    if dbase.is_some() {
        xbase.is_dbase = true;
        xbase.dbase = dbase;
        xbase.next = get_g_xbase_head_rc();
        let xbase = Some(Arc::new(RwLock::new(xbase)));
        set_g_xbase_head(xbase);
        return true;
    }

    let mut working_directory = [0 as c_char; COMPAT_MAX_PATH];
    if getcwd(
        working_directory.as_mut_ptr(),
        COMPAT_MAX_PATH as GetcwdSize,
    ) == null_mut()
    {
        // FIXME: Leaking xbase and path.
        return false;
    }

    if chdir(path) == 0 {
        chdir(working_directory.as_ptr());
        xbase.next = get_g_xbase_head_rc();
        let xbase = Some(Arc::new(RwLock::new(xbase)));
        set_g_xbase_head(xbase);
        return true;
    }

    if xbase_make_directory(path) != 0 {
        // FIXME: Leaking xbase and path.
        return false;
    }

    chdir(working_directory.as_ptr());

    xbase.next = get_g_xbase_head_rc();
    let xbase = Some(Arc::new(RwLock::new(xbase)));
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
            if !xbase_open(tok) {
                return false;
            }
            tok = strtok(null_mut(), delimiter.as_ptr());
        }
    }

    true
}

unsafe fn xlist_enumerate(
    pattern: *const c_char,
    handler: unsafe extern "C" fn(*const XListEnumerationContext) -> bool,
    xlist: *mut XList,
) -> bool {
    assert_ne!(pattern, null()); // "filespec", "xfile.c", 845
    assert_ne!(
        handler,
        mem::transmute::<*const c_void, extern "C" fn(*const XListEnumerationContext) -> bool>(
            null()
        )
    ); // "enumfunc", "xfile.c", 846

    let mut directory_file_find_data = DirectoryFileFindData::default();
    let mut context = XListEnumerationContext::default();

    context.xlist = xlist;

    let mut native_pattern = [0 as c_char; COMPAT_MAX_PATH];
    strcpy(native_pattern.as_mut_ptr(), pattern);
    rust_compat_windows_path_to_native(native_pattern.as_mut_ptr());

    let mut drive = [0 as c_char; COMPAT_MAX_DRIVE as usize];
    let mut dir = [0 as c_char; COMPAT_MAX_DIR as usize];
    let mut file_name = [0 as c_char; COMPAT_MAX_FNAME as usize];
    let mut extension = [0 as c_char; COMPAT_MAX_EXT as usize];
    rust_compat_splitpath(
        native_pattern.as_mut_ptr(),
        drive.as_mut_ptr(),
        dir.as_mut_ptr(),
        file_name.as_mut_ptr(),
        extension.as_mut_ptr(),
    );
    if drive[0] != '\0' as c_char
        || dir[0] == '\\' as c_char
        || dir[0] == '/' as c_char
        || dir[0] == '.' as c_char
    {
        if file_find_first(native_pattern.as_ptr(), &mut directory_file_find_data) {
            loop {
                let is_directory = file_find_is_directory(&directory_file_find_data);
                let entry_name = file_find_get_name(&directory_file_find_data);

                if is_directory {
                    let dot_dot = CString::new("..").expect("valid string");
                    let dot = CString::new(".").expect("valid string");
                    if strcmp(entry_name, dot_dot.as_ptr()) == 0
                        || strcmp(entry_name, dot.as_ptr()) == 0
                    {
                        continue;
                    }

                    context._type = XFileEnumerationEntryType::XfileEnumerationEntryTypeDirectory;
                } else {
                    context._type = XFileEnumerationEntryType::XfileEnumerationEntryTypeFile;
                }

                rust_compat_makepath(
                    context.name.as_mut_ptr(),
                    drive.as_ptr(),
                    dir.as_ptr(),
                    entry_name,
                    null(),
                );

                if !handler(&context) {
                    break;
                }

                if !file_find_next(&mut directory_file_find_data) {
                    break;
                }
            }
        }
        return file_find_close(&mut directory_file_find_data);
    }

    let mut optional_xbase = get_g_xbase_head_rc();
    while let Some(xbase) = optional_xbase {
        let xbase = xbase.read();
        if xbase.is_dbase {
            let mut dbase_find_data = DFileFindData::default();
            let dbase = &xbase.dbase.as_ref().expect("").borrow();
            if dbase_find_first_entry(dbase, &mut dbase_find_data, pattern) {
                context._type = XFileEnumerationEntryType::XfileEnumerationEntryTypeDfile;

                loop {
                    strcpy(
                        context.name.as_mut_ptr(),
                        dbase_find_data.file_name.as_ptr(),
                    );
                    if !handler(&context) {
                        return dbase_find_close(dbase, &dbase_find_data);
                    }
                    if !dbase_find_next_entry(dbase, &mut dbase_find_data) {
                        break;
                    }
                }

                dbase_find_close(dbase, &dbase_find_data);
            }
        } else {
            let mut path = [0 as c_char; COMPAT_MAX_PATH];
            let sformat_sformat = CString::new("%s\\%s").expect("valid string");
            snprintf(
                path.as_mut_ptr(),
                mem::size_of_val(&path),
                sformat_sformat.as_ptr(),
                xbase.get_path_cstr(),
                pattern,
            );
            rust_compat_windows_path_to_native(path.as_mut_ptr());

            if file_find_first(path.as_mut_ptr(), &mut directory_file_find_data) {
                loop {
                    let is_directory = file_find_is_directory(&directory_file_find_data);
                    let entry_name = file_find_get_name(&directory_file_find_data);

                    if is_directory {
                        let dot_dot = CString::new("..").expect("valid string");
                        let dot = CString::new(".").expect("valid string");
                        if strcmp(entry_name, dot_dot.as_ptr()) == 0
                            || strcmp(entry_name, dot.as_ptr()) == 0
                        {
                            continue;
                        }

                        context._type =
                            XFileEnumerationEntryType::XfileEnumerationEntryTypeDirectory;
                    } else {
                        context._type = XFileEnumerationEntryType::XfileEnumerationEntryTypeFile;
                    }

                    rust_compat_makepath(
                        context.name.as_mut_ptr(),
                        drive.as_ptr(),
                        dir.as_ptr(),
                        entry_name,
                        null(),
                    );

                    if !handler(&context) {
                        break;
                    }

                    if !file_find_next(&mut directory_file_find_data) {
                        break;
                    }
                }
            }
            file_find_close(&directory_file_find_data);
        }
        optional_xbase = xbase.next.clone();
    }

    rust_compat_splitpath(
        native_pattern.as_ptr(),
        drive.as_mut_ptr(),
        dir.as_mut_ptr(),
        file_name.as_mut_ptr(),
        extension.as_mut_ptr(),
    );
    if file_find_first(native_pattern.as_ptr(), &mut directory_file_find_data) {
        loop {
            let is_directory = file_find_is_directory(&directory_file_find_data);
            let entry_name = file_find_get_name(&directory_file_find_data);

            if is_directory {
                let dot_dot = CString::new("..").expect("valid string");
                let dot = CString::new(".").expect("valid string");
                if strcmp(entry_name, dot_dot.as_ptr()) == 0
                    || strcmp(entry_name, dot.as_ptr()) == 0
                {
                    continue;
                }

                context._type = XFileEnumerationEntryType::XfileEnumerationEntryTypeDirectory;
            } else {
                context._type = XFileEnumerationEntryType::XfileEnumerationEntryTypeFile;
            }

            rust_compat_makepath(
                context.name.as_mut_ptr(),
                drive.as_ptr(),
                dir.as_ptr(),
                entry_name,
                null(),
            );

            if !handler(&context) {
                break;
            }

            if !file_find_next(&mut directory_file_find_data) {
                break;
            }
        }
    }

    file_find_close(&directory_file_find_data)
}

#[no_mangle]
pub unsafe extern "C" fn rust_xlist_free(xlist: *mut XList) {
    assert_ne!(xlist, null_mut()); // "list", "xfile.c", 949

    let file_names = (*xlist).file_names;
    for index in 0..(*xlist).file_names_length {
        let file_name = *file_names.offset(index as isize);
        if file_name != null_mut() {
            free(file_name as *mut c_void);
        }
    }

    free(file_names as *mut c_void);

    memset(xlist as *mut c_void, 0, mem::size_of::<XList>());
}

unsafe extern "C" fn enumerate_handler(context: *const XListEnumerationContext) -> bool {
    if (*context)._type == XFileEnumerationEntryType::XfileEnumerationEntryTypeDirectory {
        return true;
    }

    let xlist = (*context).xlist;

    let file_names = realloc(
        (*xlist).file_names as *mut c_void,
        mem::size_of_val(&*(*xlist).file_names) * ((*xlist).file_names_length + 1) as usize,
    ) as *mut *mut c_char;
    if file_names == null_mut() {
        rust_xlist_free(xlist);
        (*xlist).file_names_length = -1;
        return false;
    }

    (*xlist).file_names = file_names;

    *file_names.offset((*xlist).file_names_length as isize) =
        rust_compat_strdup((*context).name.as_ptr());
    if *file_names.offset((*xlist).file_names_length as isize) == null_mut() {
        rust_xlist_free(xlist);
        (*xlist).file_names_length = -1;
        return false;
    }

    (*xlist).file_names_length += 1;

    true
}

#[no_mangle]
pub unsafe extern "C" fn rust_xlist_init(pattern: *const c_char, xlist: *mut XList) -> bool {
    xlist_enumerate(pattern, enumerate_handler, xlist);
    (*xlist).file_names_length != -1
}
