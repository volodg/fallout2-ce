use std::ptr::null_mut;
use libc::{c_char, c_int, c_ulong, strncpy};
use sdl2_sys::{SDL_itoa, SDL_strcasecmp, SDL_strlwr, SDL_strncasecmp, SDL_strupr};

const COMPAT_MAX_DRIVE: u8 = 3;
const COMPAT_MAX_DIR: u16 = 256;
const COMPAT_MAX_FNAME: u16 = 256;
const COMPAT_MAX_EXT: u16 = 256;

#[no_mangle]
pub extern "C" fn rust_compat_stricmp(
    string1: *const c_char,
    string2: *const c_char,
) -> c_int {
    unsafe { SDL_strcasecmp(string1, string2) }
}

#[no_mangle]
pub extern "C" fn rust_compat_strnicmp(string1: *const c_char, string2: *const c_char, size: c_ulong) -> c_int {
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
pub extern "C" fn rust_compat_itoa(value: c_int, buffer: *mut c_char, radix: c_int) -> *const c_char {
    unsafe { SDL_itoa(value, buffer, radix) }
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
pub extern "C" fn rust_compat_makepath(path: *mut c_char, drive: *const c_char, dir: *const c_char, fname: *const c_char, ext: *const c_char) {
    unsafe { _makepath(path, drive, dir, fname, ext) }
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
