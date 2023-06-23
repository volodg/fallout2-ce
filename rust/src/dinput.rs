use sdl2::sys::{SDL_INIT_EVENTS, SDL_InitSubSystem, SDL_QuitSubSystem, SDL_SetRelativeMouseMode};
use sdl2::sys::SDL_bool::SDL_TRUE;

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
