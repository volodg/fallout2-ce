use libc::{c_char, c_int, c_long, c_ulong, closedir, lseek, opendir, readdir, strlen, SEEK_CUR};
#[cfg(not(target_family = "windows"))]
use libc::{strchr, strcpy, strncpy};
use sdl2_sys::{SDL_itoa, SDL_strcasecmp, SDL_strlwr, SDL_strncasecmp, SDL_strupr};
use std::ffi::CString;
#[cfg(not(target_family = "windows"))]
use std::ptr::null_mut;

#[cfg(not(target_family = "windows"))]
const COMPAT_MAX_DRIVE: u8 = 3;
#[cfg(not(target_family = "windows"))]
const COMPAT_MAX_DIR: u16 = 256;
#[cfg(not(target_family = "windows"))]
const COMPAT_MAX_FNAME: u16 = 256;
#[cfg(not(target_family = "windows"))]
const COMPAT_MAX_EXT: u16 = 256;

const COMPAT_MAX_PATH: u16 = 260;

#[no_mangle]
pub extern "C" fn rust_compat_stricmp(string1: *const c_char, string2: *const c_char) -> c_int {
    unsafe { SDL_strcasecmp(string1, string2) }
}

#[no_mangle]
pub extern "C" fn rust_compat_strnicmp(
    string1: *const c_char,
    string2: *const c_char,
    size: c_ulong,
) -> c_int {
    unsafe { SDL_strncasecmp(string1, string2, size) }
}

#[no_mangle]
pub extern "C" fn rust_compat_strupr(string: *mut c_char) -> *const c_char {
    unsafe { SDL_strupr(string) }
}

#[no_mangle]
pub extern "C" fn rust_compat_strlwr(string: *mut c_char) -> *const c_char {
    unsafe { SDL_strlwr(string) }
}

#[no_mangle]
pub extern "C" fn rust_compat_itoa(
    value: c_int,
    buffer: *mut c_char,
    radix: c_int,
) -> *const c_char {
    unsafe { SDL_itoa(value, buffer, radix) }
}

#[cfg(target_family = "windows")]
extern "C" {
    fn _makepath(
        path: *mut c_char,
        drive: *const c_char,
        dir: *const c_char,
        fname: *const c_char,
        ext: *const c_char,
    );
}

#[no_mangle]
#[cfg(target_family = "windows")]
pub extern "C" fn rust_compat_makepath(
    path: *mut c_char,
    drive: *const c_char,
    dir: *const c_char,
    fname: *const c_char,
    ext: *const c_char,
) {
    unsafe { _makepath(path, drive, dir, fname, ext) }
}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
pub extern "C" fn rust_compat_makepath(
    mut path: *mut c_char,
    drive: *const c_char,
    dir: *const c_char,
    fname: *const c_char,
    ext: *const c_char,
) {
    unsafe {
        *path = '\0' as c_char;
    }

    if drive != null_mut() {
        unsafe {
            if *drive != '\0' as c_char {
                strcpy(path, drive);
                path = strchr(path, '\0' as c_int);

                if *path.offset(-1) == '/' as c_char {
                    path = path.offset(-1);
                } else {
                    *path = '/' as c_char;
                }
            }
        }
    }

    if dir != null_mut() {
        unsafe {
            if *dir != '\0' as c_char {
                if *dir != '/' as c_char && *path == '/' as c_char {
                    path = path.offset(1);
                }

                strcpy(path, dir);
                path = strchr(path, '\0' as c_int);

                if *path.offset(-1) == '/' as c_char {
                    path = path.offset(-1);
                } else {
                    *path = '/' as c_char;
                }
            }
        }
    }

    unsafe {
        if fname != null_mut() && *fname != '\0' as c_char {
            if *fname != '/' as c_char && *path == '/' as c_char {
                path = path.offset(1);
            }

            strcpy(path, fname);
            path = strchr(path, '\0' as c_int);
        } else {
            if *path == '/' as c_char {
                path = path.offset(1);
            }
        }
    }

    if ext != null_mut() {
        unsafe {
            if *ext != '\0' as c_char {
                if *ext != '.' as c_char {
                    *path = '.' as c_char;
                    path = path.offset(1);
                }

                strcpy(path, ext);
                path = strchr(path, '\0' as c_int);
            }
        }
    }

    unsafe {
        *path = '\0' as c_char;
    }
}

#[cfg(target_family = "windows")]
extern "C" {
    fn _splitpath(
        path: *const c_char,
        drive: *mut c_char,
        dir: *mut c_char,
        fname: *mut c_char,
        ext: *mut c_char,
    );
}

#[no_mangle]
#[cfg(target_family = "windows")]
pub extern "C" fn rust_compat_splitpath(
    path: *const c_char,
    drive: *mut c_char,
    dir: *mut c_char,
    fname: *mut c_char,
    ext: *mut c_char,
) {
    unsafe { _splitpath(path, drive, dir, fname, ext) }
}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
pub extern "C" fn rust_compat_splitpath(
    mut path: *const c_char,
    drive: *mut c_char,
    dir: *mut c_char,
    fname: *mut c_char,
    ext: *mut c_char,
) {
    let drive_start = path;

    unsafe {
        if *path == '/' as c_char && *path.offset(1) == '/' as c_char {
            path = path.offset(2);
            let curr = *path;
            while curr != '\0' as c_char && curr != '/' as c_char && curr != '.' as c_char {
                path = path.offset(1);
            }
        }
    }

    fn set_component(component: *mut c_char, start: *const c_char, end: *const c_char, max: usize) {
        if component == null_mut() {
            return;
        }

        let mut dir_size = unsafe { end.offset_from(start) };
        if dir_size > (max - 1) as isize {
            dir_size = (max - 1) as isize;
        }
        unsafe {
            strncpy(component, start, dir_size as usize);
            *component.offset(dir_size) = '\0' as c_char;
        }
    }

    set_component(drive, drive_start, path, COMPAT_MAX_DRIVE.into());

    let dir_start = path;
    let mut fname_start = path;
    let mut ext_start: *const c_char = null_mut();

    let mut end = path;
    unsafe {
        while *end != '\0' as c_char {
            if *end == '/' as c_char {
                fname_start = end.offset(1);
            } else if *end == '.' as c_char {
                ext_start = end;
            }
            end = end.offset(1);
        }
    }

    if ext_start == null_mut() {
        ext_start = end;
    }

    set_component(dir, dir_start, fname_start, COMPAT_MAX_DIR.into());
    set_component(fname, fname_start, ext_start, COMPAT_MAX_FNAME.into());
    set_component(ext, ext_start, end, COMPAT_MAX_EXT.into());
}

#[no_mangle]
pub extern "C" fn rust_compat_tell(fd: c_int) -> c_long {
    unsafe { lseek(fd, 0, SEEK_CUR) }
}

#[no_mangle]
#[cfg(target_family = "windows")]
pub extern "C" fn rust_compat_windows_path_to_native(path: *mut c_char) {}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
pub unsafe extern "C" fn rust_compat_windows_path_to_native(path: *mut c_char) {
    let mut pch = path;
    while *pch != '\0' as c_char {
        if *pch == '\\' as c_char {
            *pch = '/' as c_char;
        }
        pch = pch.offset(1);
    }
}

#[no_mangle]
#[cfg(target_family = "windows")]
pub extern "C" fn rust_compat_resolve_path(path: *mut c_char) {}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
pub unsafe extern "C" fn rust_compat_resolve_path(path: *mut c_char) {
    let mut pch = path;

    let mut dir;
    if *pch == '/' as c_char {
        let str = CString::new("/").expect("valid c string");
        dir = opendir(str.as_ptr());
        pch = pch.offset(1);
    } else {
        let str = CString::new(".").expect("valid c string");
        dir = opendir(str.as_ptr());
    }

    while dir != null_mut() {
        let sep = unsafe { strchr(pch, '/' as c_int) };
        let length = if sep != null_mut() {
            (unsafe { sep.offset_from(pch) }) as usize
        } else {
            unsafe { strlen(pch) }
        };

        let mut found = false;

        let mut entry = unsafe { readdir(dir) };
        while entry != null_mut() {
            if strlen((*entry).d_name.as_ptr()) == length
                && rust_compat_strnicmp(pch, (*entry).d_name.as_ptr(), length as c_ulong) == 0
            {
                strncpy(pch, (*entry).d_name.as_ptr(), length);
                found = true;
                break;
            }
            entry = readdir(dir);
        }

        closedir(dir);

        if !found {
            break;
        }

        if sep == null_mut() {
            break;
        }

        *sep = '\0' as c_char;
        dir = opendir(path);
        *sep = '/' as c_char;
        pch = sep.offset(1);
    }
}

#[cfg(target_family = "windows")]
extern "C" {
    fn mkdir(path: *const c_char) -> c_int;
}

#[no_mangle]
#[cfg(target_family = "windows")]
fn native_mkdir(path: *const c_char) -> c_int {
    mkdir(path)
}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
unsafe fn native_mkdir(path: *const c_char) -> c_int {
    libc::mkdir(path, 0755)
}

#[no_mangle]
pub unsafe extern "C" fn rust_compat_mkdir(path: *const c_char) -> c_int {
    let mut native_path = ['\0' as c_char; COMPAT_MAX_PATH as usize];
    strcpy(native_path.as_mut_ptr(), path);
    rust_compat_windows_path_to_native(native_path.as_mut_ptr());
    rust_compat_resolve_path(native_path.as_mut_ptr());
    native_mkdir(native_path.as_ptr())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    fn to_string(input: &mut [u8]) -> String {
        CStr::from_bytes_until_nul(input)
            .expect("REASON")
            .to_str()
            .expect("")
            .into()
    }

    #[cfg(target_family = "windows")]
    #[test]
    fn test_compat_splitpath() {
        let ctring = CString::new("C:\\path1\\path2\\file.txt").expect("");
        let path = ctring.as_ptr();

        let mut drive = [0 as u8; 4];
        let mut dir = [0 as u8; 20];
        let mut fname = [0 as u8; 10];
        let mut ext = [0 as u8; 10];

        rust_compat_splitpath(
            path,
            drive.as_mut_ptr() as *mut c_char,
            dir.as_mut_ptr() as *mut c_char,
            fname.as_mut_ptr() as *mut c_char,
            ext.as_mut_ptr() as *mut c_char,
        );

        assert_eq!("C:", to_string(drive.as_mut_slice()));
        assert_eq!("\\path1\\path2\\", to_string(dir.as_mut_slice()));
        assert_eq!("file", to_string(fname.as_mut_slice()));
        assert_eq!(".txt", to_string(ext.as_mut_slice()));
    }

    #[cfg(target_family = "windows")]
    #[test]
    fn test_compat_makepath() {
        let mut path = [0 as u8; 100];

        let drive = CString::new("C").expect("");
        let dir = CString::new("tmp1\\tmp2").expect("");
        let filename = CString::new("filename").expect("");
        let extension = CString::new(".txt").expect("");

        rust_compat_makepath(
            path.as_mut_ptr() as *mut c_char,
            drive.as_ptr(),
            dir.as_ptr(),
            filename.as_ptr(),
            extension.as_ptr(),
        );

        assert_eq!("C:tmp1\\tmp2\\filename.txt", to_string(path.as_mut_slice()));
    }

    #[cfg(not(target_family = "windows"))]
    #[test]
    fn test_compat_makepath() {
        let mut path = [0 as u8; 100];

        let drive = CString::new("media").expect("");
        let dir = CString::new("tmp1/tmp2").expect("");
        let filename = CString::new("filename").expect("");
        let extension = CString::new(".txt").expect("");

        rust_compat_makepath(
            path.as_mut_ptr() as *mut c_char,
            drive.as_ptr(),
            dir.as_ptr(),
            filename.as_ptr(),
            extension.as_ptr(),
        );

        assert_eq!(
            "media/tmp1/tmp2/filename.txt",
            to_string(path.as_mut_slice())
        );
    }

    #[cfg(not(target_family = "windows"))]
    #[test]
    fn test_compat_splitpath_1() {
        let ctring = CString::new("MAPS/*.SAV").expect("");
        let path = ctring.as_ptr();

        let mut drive = [0 as u8; 4];
        let mut dir = [0 as u8; 20];
        let mut fname = [0 as u8; 10];
        let mut ext = [0 as u8; 10];

        rust_compat_splitpath(
            path,
            drive.as_mut_ptr() as *mut c_char,
            dir.as_mut_ptr() as *mut c_char,
            fname.as_mut_ptr() as *mut c_char,
            ext.as_mut_ptr() as *mut c_char,
        );

        assert_eq!("", to_string(drive.as_mut_slice()));
        assert_eq!("MAPS/", to_string(dir.as_mut_slice()));
        assert_eq!("*", to_string(fname.as_mut_slice()));
        assert_eq!(".SAV", to_string(ext.as_mut_slice()));
    }

    #[cfg(not(target_family = "windows"))]
    #[test]
    fn test_compat_splitpath_2() {
        let ctring = CString::new("proto/critters/*.pro").expect("");
        let path = ctring.as_ptr();

        let mut drive = [0 as u8; 4];
        let mut dir = [0 as u8; 20];
        let mut fname = [0 as u8; 10];
        let mut ext = [0 as u8; 10];

        rust_compat_splitpath(
            path,
            drive.as_mut_ptr() as *mut c_char,
            dir.as_mut_ptr() as *mut c_char,
            fname.as_mut_ptr() as *mut c_char,
            ext.as_mut_ptr() as *mut c_char,
        );

        assert_eq!("", to_string(drive.as_mut_slice()));
        assert_eq!("proto/critters/", to_string(dir.as_mut_slice()));
        assert_eq!("*", to_string(fname.as_mut_slice()));
        assert_eq!(".pro", to_string(ext.as_mut_slice()));
    }
}
