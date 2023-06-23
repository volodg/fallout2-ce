use std::cell::Cell;
use std::ptr::{null, null_mut};
use libc::{c_int, c_uchar};
use sdl2::sys::{SDL_BUTTON_LEFT, SDL_BUTTON_RIGHT, SDL_BUTTON_X1, SDL_GetRelativeMouseState, SDL_INIT_EVENTS, SDL_InitSubSystem, SDL_PumpEvents, SDL_QuitSubSystem, SDL_SetRelativeMouseMode};
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

#[repr(C)]
pub struct MouseData {
    x: c_int,
    y: c_int,
    buttons: [c_uchar; 2],
    wheel_x: c_int,
    wheel_y: c_int,
}

#[no_mangle]
pub extern "C" fn c_mouse_device_get_data(mouse_state: *mut MouseData) -> bool {
    if mouse_state == null_mut() {
        return false
    }

    // CE: This function is sometimes called outside loops calling `get_input`
    // and subsequently `GNW95_process_message`, so mouse events might not be
    // handled by SDL yet.
    //
    // TODO: Move mouse events processing into `GNW95_process_message` and
    // update mouse position manually.
    unsafe {
        SDL_PumpEvents();
    }

    let buttons = unsafe {
        SDL_GetRelativeMouseState(&mut (*mouse_state).x, &mut (*mouse_state).y)
    };

    fn sdl_button(x: u32) -> u32 {
        1 << (x - 1)
    }

    unsafe {
        (*mouse_state).buttons[0] = ((buttons & sdl_button(SDL_BUTTON_LEFT)) != 0) as c_uchar;
        (*mouse_state).buttons[1] = ((buttons & sdl_button(SDL_BUTTON_RIGHT)) != 0) as c_uchar;
        (*mouse_state).wheel_x = MOUSE_WHEEL_DELTA_X.get();
        (*mouse_state).wheel_y = MOUSE_WHEEL_DELTA_Y.get();
    }

    MOUSE_WHEEL_DELTA_X.set(0);
    MOUSE_WHEEL_DELTA_Y.set(0);

    true
}

