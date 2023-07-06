use std::ffi::{c_int, c_void, CString};
use std::mem;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicPtr, Ordering};
use libc::{c_char, fclose, fgetc, FILE, free, malloc, memset, rewind, snprintf};
use libz_sys::{gzclose, gzFile};
use crate::dfile::{DBase, DFile, dfile_close, rust_dfile_open};
use crate::platform_compat::{COMPAT_MAX_DIR, COMPAT_MAX_DRIVE, COMPAT_MAX_PATH, rust_compat_fopen, compat_gzopen, rust_compat_splitpath};

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
    path: *const c_char,

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

// 0x6B24D0
static G_X_BASE_HEAD: AtomicPtr<XBase> = AtomicPtr::new(null_mut());

#[no_mangle]
pub unsafe extern "C" fn rust_get_g_xbase_head() -> *const XBase {
    G_X_BASE_HEAD.load(Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn rust_set_g_xbase_head(value: *mut XBase) {
    G_X_BASE_HEAD.store(value, Ordering::Relaxed)
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
