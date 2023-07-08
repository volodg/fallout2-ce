use libc::{c_int, c_uchar};
use sdl2::sys::SDL_EventType::{SDL_KEYDOWN, SDL_MOUSEWHEEL, SDL_TEXTINPUT};
use sdl2::sys::SDL_bool::SDL_TRUE;
use sdl2::sys::{
    SDL_Event, SDL_FlushEvents, SDL_GetRelativeMouseState, SDL_InitSubSystem, SDL_PumpEvents,
    SDL_QuitSubSystem, SDL_SetRelativeMouseMode, SDL_BUTTON_LEFT, SDL_BUTTON_RIGHT,
    SDL_INIT_EVENTS,
};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicI32, Ordering};

static MOUSE_WHEEL_DELTA_X: AtomicI32 = AtomicI32::new(0);
static MOUSE_WHEEL_DELTA_Y: AtomicI32 = AtomicI32::new(0);

#[no_mangle]
pub extern "C" fn rust_c_direct_input_init() -> bool {
    unsafe {
        if SDL_InitSubSystem(SDL_INIT_EVENTS) != 0 {
            return false;
        }
    }

    if !mouse_device_init() || !keyboard_device_init() {
        rust_c_direct_input_free();
        return false;
    }

    true
}

fn mouse_device_init() -> bool {
    unsafe { SDL_SetRelativeMouseMode(SDL_TRUE) == 0 }
}

fn keyboard_device_init() -> bool {
    true
}

#[no_mangle]
pub extern "C" fn rust_c_direct_input_free() {
    unsafe {
        SDL_QuitSubSystem(SDL_INIT_EVENTS);
    }
}

#[no_mangle]
pub extern "C" fn rust_c_mouse_device_acquire() -> bool {
    true
}

#[no_mangle]
pub extern "C" fn rust_c_mouse_device_unacquire() -> bool {
    true
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
pub extern "C" fn rust_c_mouse_device_get_data(mouse_state: *mut MouseData) -> bool {
    if mouse_state == null_mut() {
        return false;
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

    let buttons =
        unsafe { SDL_GetRelativeMouseState(&mut (*mouse_state).x, &mut (*mouse_state).y) };

    fn sdl_button(x: u32) -> u32 {
        1 << (x - 1)
    }

    unsafe {
        (*mouse_state).buttons[0] = ((buttons & sdl_button(SDL_BUTTON_LEFT)) != 0) as c_uchar;
        (*mouse_state).buttons[1] = ((buttons & sdl_button(SDL_BUTTON_RIGHT)) != 0) as c_uchar;
        (*mouse_state).wheel_x = MOUSE_WHEEL_DELTA_X.load(Ordering::Relaxed);
        (*mouse_state).wheel_y = MOUSE_WHEEL_DELTA_Y.load(Ordering::Relaxed);
    }

    MOUSE_WHEEL_DELTA_X.store(0, Ordering::Relaxed);
    MOUSE_WHEEL_DELTA_Y.store(0, Ordering::Relaxed);

    true
}

#[no_mangle]
pub extern "C" fn rust_c_keyboard_device_reset() -> bool {
    unsafe {
        SDL_FlushEvents(
            SDL_KEYDOWN as sdl2::sys::Uint32,
            SDL_TEXTINPUT as sdl2::sys::Uint32,
        );
    }
    true
}

#[no_mangle]
pub extern "C" fn rust_c_handle_mouse_event(event: *mut SDL_Event) {
    // Mouse movement and buttons are accumulated in SDL itself and will be
    // processed later in `mouseDeviceGetData` via `SDL_GetRelativeMouseState`.

    unsafe {
        if (*event).type_ == SDL_MOUSEWHEEL as sdl2::sys::Uint32 {
            let x_value = MOUSE_WHEEL_DELTA_X.load(Ordering::Relaxed);
            MOUSE_WHEEL_DELTA_X.store(x_value + (*event).wheel.x, Ordering::Relaxed);
            let y_value = MOUSE_WHEEL_DELTA_Y.load(Ordering::Relaxed);
            MOUSE_WHEEL_DELTA_Y.store(y_value + (*event).wheel.y, Ordering::Relaxed);
        }
    }
}
