use std::sync::atomic::{AtomicU32, Ordering};
use sdl2::sys::SDL_AudioDeviceID;

static AUDIO_ENGINE_DEVICE_ID: AtomicU32 = AtomicU32::new(u32::MAX);

#[no_mangle]
pub extern "C" fn c_set_audio_engine_device_id(value: SDL_AudioDeviceID) {
    AUDIO_ENGINE_DEVICE_ID.store(value, Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn c_get_audio_engine_device_id() -> SDL_AudioDeviceID {
    AUDIO_ENGINE_DEVICE_ID.load(Ordering::Relaxed)
}
