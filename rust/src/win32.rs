use std::sync::atomic::{AtomicBool, Ordering};

static PROGRAM_IS_ACTIVE: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "C" fn rust_c_set_program_is_active(value: bool) {
    PROGRAM_IS_ACTIVE.store(value, Ordering::Relaxed)
}

pub fn program_is_active() -> bool {
    PROGRAM_IS_ACTIVE.load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn rust_c_get_program_is_active() -> bool {
    program_is_active()
}
