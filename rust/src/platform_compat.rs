use std::ptr::null_mut;
use libc::{c_char, strncpy};

const COMPAT_MAX_DRIVE: u8 = 3;
const COMPAT_MAX_DIR: u16 = 256;
const COMPAT_MAX_FNAME: u16 = 256;
const COMPAT_MAX_EXT: u16 = 256;

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

    if drive != null_mut() {
        let mut drive_size = unsafe { path.offset_from(drive_start) };
        if drive_size > (COMPAT_MAX_DRIVE - 1).into() {
            drive_size = (COMPAT_MAX_DRIVE - 1).into();
        }
        unsafe {
            strncpy(drive, path, drive_size as usize);
            *drive.offset(drive_size) = '\0' as c_char;
        }
    }

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

    fn set_component(component: *mut c_char, path: *const c_char, start: *const c_char, end: *const c_char, max: usize) {
        let mut dir_size = unsafe { end.offset_from(start) };
        if dir_size > (max - 1) as isize {
            dir_size = (max - 1) as isize;
        }
        unsafe {
            strncpy(component, path, dir_size as usize);
            *component.offset(dir_size) = '\0' as c_char;
        }
    }

    set_component(dir, path, dir_start, fname_start, COMPAT_MAX_DIR.into());
    set_component(fname, path, fname_start, ext_start, COMPAT_MAX_FNAME.into());
    set_component(ext, path, ext_start, end, COMPAT_MAX_EXT.into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

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

        fn to_string(input: &mut [u8]) -> String {
            CStr::from_bytes_until_nul(input)
                .expect("REASON")
                .to_str()
                .expect("")
                .into()
        }

        assert_eq!("C:", to_string(drive.as_mut_slice()));
        assert_eq!("\\path1\\path2\\", to_string(dir.as_mut_slice()));
        assert_eq!("file", to_string(fname.as_mut_slice()));
        assert_eq!(".txt", to_string(ext.as_mut_slice()));
    }
}
