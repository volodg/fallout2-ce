use std::ffi::{c_int, c_void};
use std::mem;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicPtr, Ordering};
use libc::{c_char, fclose, fgetc, FILE, free, malloc, memset, rewind, snprintf};
use libz_sys::{gzclose, gzFile};
use sdl2_sys::__pthread_cond_s;
use crate::dfile::{DBase, DFile, rust_dfile_close};
// use crate::platform_compat::{COMPAT_MAX_DIR, COMPAT_MAX_DRIVE, COMPAT_MAX_PATH, rust_compat_splitpath};

#[repr(C)]
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
    dbase: *const DBase,

    // A flag used to denote that this xbase represents .DAT file (true), or
    // a directory (false).
    //
    // NOTE: Original type is 1 byte, likely unsigned char.
    is_dbase: bool,

    // Next [XBase] in linked list.
    next: *const XBase,
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
        XFileType::XfileTypeDfile => rust_dfile_close((*stream).file.dfile),
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

    let stream = malloc(mem::size_of::<XFile>()) as *const XFile;
    /*if stream == null_mut() {
        return null();
    }

    memset(stream, 0, mem::size_of_val(&*stream));

    // NOTE: Compiled code uses different lengths.
    let mut drive = [0 as c_char; COMPAT_MAX_DRIVE];
    let mut dir = [0 as c_char; COMPAT_MAX_DIR];
    rust_compat_splitpath(filePath, drive.as_mut_ptr(), dir.as_mut_ptr(), null_mut(), null_mut());

    let path = [0 as c_char; COMPAT_MAX_PATH];
    if drive[0] != '\0' as c_char || dir[0] == '\\' as c_char || dir[0] == '/' as c_char || dir[0] == '.' as c_char {
        // [filePath] is an absolute path. Attempt to open as plain stream.
        (*stream).file = compat_fopen(filePath, mode);
        if (*stream).file == nullptr {
            free(stream);
            return null();
        }

        (*stream)._type = XFILE_TYPE_FILE;
        snprintf(path, sizeof(path), "%s", filePath);
    } else {
        // [filePath] is a relative path. Loop thru open xbases and attempt to
        // open [filePath] from appropriate xbase.
        let curr = gXbaseHead;
        while curr != nullptr {
            if curr->isDbase {
                // Attempt to open dfile stream from dbase.
                (*stream).dfile = rust_dfile_open(curr->dbase, filePath, mode);
                if (*stream).dfile != nullptr {
                    (*stream)._type = XFILE_TYPE_DFILE;
                    snprintf(path, sizeof(path), "%s", filePath);
                    break;
                }
            } else {
                // Build path relative to directory-based xbase.
                snprintf(path, sizeof(path), "%s\\%s", curr->path, filePath);

                // Attempt to open plain stream.
                (*stream).file = compat_fopen(path, mode);
                if (*stream).file != nullptr {
                    (*stream)._type = XFILE_TYPE_FILE;
                    break;
                }
            }
            curr = curr->next;
        }

        if (*stream).file == nullptr {
            // File was not opened during the loop above. Attempt to open file
            // relative to the current working directory.
            (*stream).file = compat_fopen(filePath, mode);
            if (*stream).file == nullptr {
                free(stream);
                return null();
            }

            (*stream)._type = XFILE_TYPE_FILE;
            snprintf(path, sizeof(path), "%s", filePath);
        }
    }

    if (*stream)._type == XFILE_TYPE_FILE {
        // Opened file is a plain stream, which might be gzipped. In this case
        // first two bytes will contain magic numbers.
        int ch1 = fgetc(stream->file);
        int ch2 = fgetc(stream->file);
        if ch1 == 0x1F && ch2 == 0x8B {
            // File is gzipped. Close plain stream and reopen this file as
            // gzipped stream.
            fclose(stream->file);

            (*stream)._type = XFILE_TYPE_GZFILE;
            (*stream).gzfile = compat_gzopen(path, mode);
        } else {
            // File is not gzipped.
            rewind(stream->file);
        }
    }*/

    stream
}

/*
XFile* xfileOpen(const char* filePath, const char* mode)
{

}
 */
