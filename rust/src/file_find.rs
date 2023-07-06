#[cfg(target_family = "windows")]
#[cfg(not(target_family = "windows"))]
use libc::DIR;
#[cfg(not(target_family = "windows"))]
use libc::{DIR, dirent};
use libc::c_char;
#[cfg(not(target_family = "windows"))]
use libc::{closedir, opendir, readdir, strcpy};
#[cfg(target_family = "windows")]
use std::os::windows::raw::HANDLE;
#[cfg(not(target_family = "windows"))]
use std::ptr::{null, null_mut};
#[cfg(target_family = "windows")]
use windows::core::PCSTR;
#[cfg(target_family = "windows")]
use windows::Win32::Foundation::{INVALID_HANDLE_VALUE};
#[cfg(target_family = "windows")]
use windows::Win32::Storage::FileSystem::{FindClose, FindFileHandle, FindFirstFileA, FindNextFileA};
#[cfg(target_family = "windows")]
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAA;
#[cfg(not(target_family = "windows"))]
use crate::fpattern::fpattern_match;
#[cfg(not(target_family = "windows"))]
use crate::platform_compat::COMPAT_MAX_PATH;
#[cfg(not(target_family = "windows"))]
use crate::platform_compat::{COMPAT_MAX_DIR, COMPAT_MAX_DRIVE, rust_compat_makepath, rust_compat_splitpath};

// NOTE: This structure is significantly different from what was in the
// original code. Watcom provides opendir/readdir/closedir implementations,
// that use Win32 FindFirstFile/FindNextFile under the hood, which in turn
// is designed to deal with patterns.
//
// The first attempt was to use `dirent` implementation by Toni Ronkko
// (https://github.com/tronkko/dirent), however it appears to be incompatible
// with what is provided by Watcom. Toni's implementation adds `*` wildcard
// unconditionally implying `opendir` accepts directory name only, which I
// guess is fine when your goal is compliance with POSIX implementation.
// However in Watcom `opendir` can handle file patterns gracefully. The problem
// can be seen during game startup when cleaning MAPS directory using
// MAPS\*.SAV pattern. Toni's implementation tries to convert that to pattern
// for Win32 API, thus making it MAPS\*.SAV\*, which is obviously incorrect
// path/pattern for any implementation.
//
// Eventually I've decided to go with compiler-specific implementation, keeping
// original implementation for Watcom (not tested). I'm not sure it will work
// in other compilers, so for now just stick with the error.

#[repr(C)]
#[cfg(target_family = "windows")]
pub struct DirectoryFileFindData {
    h_find: HANDLE,
    ffd: WIN32_FIND_DATAA
}

#[repr(C)]
#[cfg(not(target_family = "windows"))]
pub struct DirectoryFileFindData {
    dir: *mut DIR,
    entry: *const dirent,
    path: [c_char; COMPAT_MAX_PATH]
}

#[no_mangle]
#[cfg(target_family = "windows")]
pub unsafe extern "C" fn rust_file_find_first(path: *const c_char, find_data: *mut DirectoryFileFindData) -> bool {
    let path = PCSTR::from_raw(path as *const u8);
    match FindFirstFileA(path, &mut (*find_data).ffd) {
        Ok(FindFileHandle(handler)) => (*find_data).h_find = handler as HANDLE,
        Err(_) => (*find_data).h_find = INVALID_HANDLE_VALUE.0 as HANDLE,
    }

    if (*find_data).h_find == INVALID_HANDLE_VALUE.0 as HANDLE {
        return false;
    }

    true
}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
pub unsafe extern "C" fn rust_file_find_first(path: *const c_char, find_data: *mut DirectoryFileFindData) -> bool {
    strcpy((*find_data).path.as_mut_ptr(), path);

    let mut drive = [0 as c_char; COMPAT_MAX_DRIVE as usize];
    let mut dir = [0 as c_char; COMPAT_MAX_DIR as usize];
    rust_compat_splitpath(path, drive.as_mut_ptr(), dir.as_mut_ptr(), null_mut(), null_mut());

    let mut base_path = [0 as c_char; COMPAT_MAX_PATH];
    rust_compat_makepath(base_path.as_mut_ptr(), drive.as_ptr(), dir.as_ptr(), null_mut(), null());

    (*find_data).dir = opendir(base_path.as_ptr());
    if (*find_data).dir == null_mut() {
        return false;
    }

    (*find_data).entry = readdir((*find_data).dir);
    while (*find_data).entry != null() {
        let mut entry_path = [0 as c_char; COMPAT_MAX_PATH];
        rust_compat_makepath(entry_path.as_mut_ptr(), drive.as_ptr(), dir.as_ptr(), rust_file_find_get_name(find_data), null());
        if fpattern_match((*find_data).path.as_ptr(), entry_path.as_ptr()) {
            break;
        }
        (*find_data).entry = readdir((*find_data).dir);
    }

    if (*find_data).entry == null() {
        closedir((*find_data).dir);
        (*find_data).dir = null_mut();
        return false
    }

    true
}

#[no_mangle]
#[cfg(target_family = "windows")]
pub unsafe extern "C" fn rust_file_find_next(find_data: *mut DirectoryFileFindData) -> bool {
    let handle = FindFileHandle((*find_data).h_find as isize);
    FindNextFileA(handle, &mut (*find_data).ffd).into()
}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
pub unsafe extern "C" fn rust_file_find_next(find_data: *mut DirectoryFileFindData) -> bool {
    let mut drive =[0 as c_char; COMPAT_MAX_DRIVE as usize];
    let mut dir =[0 as c_char; COMPAT_MAX_DIR as usize];
    rust_compat_splitpath((*find_data).path.as_mut_ptr(), drive.as_mut_ptr(), dir.as_mut_ptr(), null_mut(), null_mut());

    (*find_data).entry = readdir((*find_data).dir);
    while (*find_data).entry != null() {
        let mut entry_path = [0 as c_char; COMPAT_MAX_PATH];
        rust_compat_makepath(entry_path.as_mut_ptr(), drive.as_ptr(), dir.as_ptr(), rust_file_find_get_name(find_data), null());
        if fpattern_match((*find_data).path.as_ptr(), entry_path.as_ptr()) {
            break;
        }
        (*find_data).entry = readdir((*find_data).dir)
    }

    if (*find_data).entry == null() {
        closedir((*find_data).dir);
        (*find_data).dir = null_mut();
        return false;
    }

    true
}

#[no_mangle]
#[cfg(target_family = "windows")]
pub unsafe extern "C" fn rust_file_find_get_name(find_data: *mut DirectoryFileFindData) -> *const c_char {
    (*find_data).ffd.cFileName.as_ptr() as *const c_char
}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
pub unsafe extern "C" fn rust_file_find_get_name(find_data: *mut DirectoryFileFindData) -> *const c_char {
    (*(*find_data).entry).d_name.as_ptr()
}

#[no_mangle]
#[cfg(target_family = "windows")]
pub unsafe extern "C" fn rust_file_find_close(find_data: *mut DirectoryFileFindData) -> bool {
    let handle = FindFileHandle((*find_data).h_find as isize);
    FindClose(handle).into()
}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
pub unsafe extern "C" fn rust_file_find_close(find_data: *mut DirectoryFileFindData) -> bool {
    if (*find_data).dir != null_mut() {
        if closedir((*find_data).dir) != 0 {
            return false;
        }
    }

    true
}
