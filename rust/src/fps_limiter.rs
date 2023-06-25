use std::ffi::c_uint;
use std::mem::forget;
use sdl2_sys::{SDL_Delay, SDL_GetTicks};

pub struct FpsLimiter {
    fps: c_uint,
    ticks: c_uint,
}

impl Default for FpsLimiter {
    fn default() -> Self {
        Self {
            fps: 60,
            ticks: 0,
        }
    }
}

impl FpsLimiter {
    fn mark(&mut self) {
        self.ticks = unsafe { SDL_GetTicks() };
    }

    fn throttle(&self) {
        if 1000 / self.fps > unsafe { SDL_GetTicks() } - self.ticks {
            unsafe { SDL_Delay(1000 / self.fps - (SDL_GetTicks() - self.ticks)) };
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_create_default_fps_limiter() -> *const FpsLimiter {
    let result = Box::new(FpsLimiter::default());
    Box::into_raw(result)
}

#[no_mangle]
pub extern "C" fn fps_limiter_mark(fps_limiter: *mut FpsLimiter) {
    let mut fps_limiter = unsafe { Box::from_raw(fps_limiter) };
    fps_limiter.mark();
    forget(fps_limiter)
}

#[no_mangle]
pub extern "C" fn fps_limiter_throttle(fps_limiter: *mut FpsLimiter) {
    let fps_limiter = unsafe { Box::from_raw(fps_limiter) };
    fps_limiter.throttle();
    forget(fps_limiter)
}
