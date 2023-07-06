#[cfg(target_family = "windows")]
#[cfg(not(target_family = "windows"))]
use libc::DIR;
#[cfg(not(target_family = "windows"))]
use libc::{DIR, dirent};
use libc::c_char;
#[cfg(target_family = "windows")]
use std::os::windows::raw::HANDLE;
#[cfg(target_family = "windows")]
use windows::core::PCSTR;
#[cfg(target_family = "windows")]
use windows::Win32::Foundation::{INVALID_HANDLE_VALUE};
#[cfg(target_family = "windows")]
use windows::Win32::Storage::FileSystem::{FindFileHandle, FindFirstFileA};
#[cfg(target_family = "windows")]
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAA;
#[cfg(not(target_family = "windows"))]
use crate::platform_compat::COMPAT_MAX_PATH;

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

#[cfg(target_family = "windows")]
pub struct DirectoryFileFindData {
    h_find: HANDLE,
    ffd: WIN32_FIND_DATAA
}

#[cfg(not(target_family = "windows"))]
pub struct DirectoryFileFindData {
    dir: *const DIR,
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
