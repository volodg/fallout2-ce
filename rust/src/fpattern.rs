use std::ffi::CStr;
use libc::c_char;
use nu_glob::Pattern;

#[no_mangle]
pub extern "C" fn rust_fpattern_match(pat: *const c_char, fname: *const c_char) -> bool {
    let pat = unsafe { CStr::from_ptr(pat) }.to_str().expect("valid input string");
    let fname = unsafe { CStr::from_ptr(fname) }.to_str().expect("valid input string");

    match Pattern::new(pat) {
        Ok(pattern) => pattern.matches(fname),
        Err(_) => false,
    }
}
