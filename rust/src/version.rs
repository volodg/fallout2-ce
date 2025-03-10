extern crate libc;

use libc::c_char;
use libc::c_short;
use std::ffi::CString;

// The size of buffer for version string.
const VERSION_MAX: u8 = 32;

const VERSION_MAJOR: u8 = 1;
const VERSION_MINOR: u8 = 2;

const VERSION_RELEASE: c_char = 'R' as c_char;
const VERSION_BUILD_TIME: &str = "Dec 11 1998 16:54:30";

#[no_mangle]
pub unsafe extern "C" fn rust_get_version(dest: *mut c_char, size: usize) {
    let version = CString::new(get_version()).expect("valid version cstring");
    std::ptr::copy(version.as_ptr(), dest, size);
}

#[no_mangle]
pub extern "C" fn rust_c_get_major_version() -> c_short {
    VERSION_MAJOR as c_short
}

#[no_mangle]
pub extern "C" fn rust_c_get_minor_version() -> c_short {
    VERSION_MINOR as c_short
}

#[no_mangle]
pub extern "C" fn rust_get_version_max() -> c_short {
    VERSION_MAX as c_short
}

#[no_mangle]
pub extern "C" fn rust_get_version_release() -> c_char {
    VERSION_RELEASE
}

#[no_mangle]
pub extern "C" fn rust_get_version_build_time() -> *const c_char {
    VERSION_BUILD_TIME.as_ptr() as *const c_char
}

fn get_version() -> String {
    format!("FALLOUT II {}.{:02}", VERSION_MAJOR, VERSION_MINOR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        assert_eq!(get_version(), "FALLOUT II 1.02");
    }

    #[test]
    fn test_rust_get_version() {
        const BUFFER_SIZE: usize = 20;

        let buf = unsafe { libc::malloc(BUFFER_SIZE) as *mut c_char };

        let version_c_string = unsafe {
            rust_get_version(buf, BUFFER_SIZE);
            CString::from_raw(buf)
        };

        let version = version_c_string.to_str().expect("valid c string");

        assert_eq!(version, "FALLOUT II 1.02");
    }
}
