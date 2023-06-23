use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static PROGRAM_IS_ACTIVE: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "C" fn c_set_program_is_active(value: bool) {
    PROGRAM_IS_ACTIVE.store(value, Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn c_get_program_is_active() -> bool {
    PROGRAM_IS_ACTIVE.load(Ordering::Relaxed)
}
