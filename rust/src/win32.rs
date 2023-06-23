use std::cell::Cell;
use std::sync::Mutex;

static PROGRAM_IS_ACTIVE: Mutex<Cell<bool>> = Mutex::new(Cell::new(false));

#[no_mangle]
pub extern "C" fn c_set_program_is_active(value: bool) {
    PROGRAM_IS_ACTIVE.lock().expect("locked").set(value)
}

#[no_mangle]
pub extern "C" fn c_get_program_is_active() -> bool {
    PROGRAM_IS_ACTIVE.lock().expect("locked").get()
}
