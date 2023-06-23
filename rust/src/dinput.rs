use std::cell::Cell;
use libc::c_int;
use sdl2::sys::{SDL_INIT_EVENTS, SDL_InitSubSystem, SDL_QuitSubSystem, SDL_SetRelativeMouseMode};
use sdl2::sys::SDL_bool::SDL_TRUE;

const MOUSE_WHEEL_DELTA_X: Cell<c_int> = Cell::new(0);
const MOUSE_WHEEL_DELTA_Y: Cell<c_int> = Cell::new(0);

#[no_mangle]
pub extern "C" fn c_direct_input_init() -> bool {
    unsafe {
        if SDL_InitSubSystem(SDL_INIT_EVENTS) != 0 {
            return false;
        }
    }

    if !mouse_device_init() || !keyboard_device_init() {
        c_direct_input_free();
        return false;
    }

    true
}

fn mouse_device_init() -> bool {
    unsafe {
        SDL_SetRelativeMouseMode(SDL_TRUE) == 0
    }
}

fn keyboard_device_init() -> bool {
    true
}

#[no_mangle]
pub extern "C" fn c_direct_input_free() {
    unsafe {
        SDL_QuitSubSystem(SDL_INIT_EVENTS);
    }
}

#[no_mangle]
pub extern "C" fn c_mouse_device_acquire() -> bool {
    true
}

#[no_mangle]
pub extern "C" fn c_mouse_device_unacquire() -> bool {
    true
}

#[no_mangle]
pub extern "C" fn c_set_g_mouse_wheel_delta_x(value: c_int) {
    MOUSE_WHEEL_DELTA_X.set(value)
}

#[no_mangle]
pub extern "C" fn c_get_g_mouse_wheel_delta_x() -> c_int {
    MOUSE_WHEEL_DELTA_X.get()
}

#[no_mangle]
pub extern "C" fn c_set_g_mouse_wheel_delta_y(value: c_int) {
    MOUSE_WHEEL_DELTA_Y.set(value)
}

#[no_mangle]
pub extern "C" fn c_get_g_mouse_wheel_delta_y() -> c_int {
    MOUSE_WHEEL_DELTA_Y.get()
}

