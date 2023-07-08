use libc::c_char;
use nu_glob::Pattern;
use std::ffi::CStr;

pub unsafe fn fpattern_match(pat: *const c_char, fname: *const c_char) -> bool {
    let pat = CStr::from_ptr(pat).to_str().expect("valid input string");
    let fname = CStr::from_ptr(fname).to_str().expect("valid input string");

    match Pattern::new(pat) {
        Ok(pattern) => pattern.matches(fname),
        Err(_) => false,
    }
}
