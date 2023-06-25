use std::ffi::{c_int, c_void};
use std::mem::forget;
use std::ptr::null;
use std::sync::atomic::{AtomicU32, Ordering};
use libc::c_uint;
use parking_lot::ReentrantMutex;
use sdl2::sys::{SDL_AudioDeviceID, SDL_AudioStream};
use lazy_static::lazy_static;

static AUDIO_ENGINE_DEVICE_ID: AtomicU32 = AtomicU32::new(u32::MAX);

#[repr(C)]
pub struct AudioEngineSoundBuffer {
    active: bool,
    size: c_uint,
    bits_per_sample: c_int,
    channels: c_int,
    rate: c_int,
    volume: c_int,
    playing: bool,
    looping: bool,
    pos: c_uint,
    data: *const c_void,
    stream: *const SDL_AudioStream,
    mutex: ReentrantMutex<()>
}

impl Default for AudioEngineSoundBuffer {
    fn default() -> Self {
        AudioEngineSoundBuffer {
            active: false,
            size: 0,
            bits_per_sample: 0,
            channels: 0,
            rate: 0,
            volume: 0,
            playing: false,
            looping: false,
            pos: 0,
            data: null(),
            stream: null(),
            mutex: ReentrantMutex::new(())
        }
    }
}

const AUDIO_ENGINE_SOUND_BUFFERS: usize = 8;

unsafe impl Send for AudioEngineSoundBuffer {}
unsafe impl Sync for AudioEngineSoundBuffer {}

lazy_static! {
    static ref AUDIO_ENGINE_SOUND_BUFFER: [AudioEngineSoundBuffer; AUDIO_ENGINE_SOUND_BUFFERS] = [
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
    ];
}

#[no_mangle]
pub extern "C" fn c_set_audio_engine_device_id(value: SDL_AudioDeviceID) {
    AUDIO_ENGINE_DEVICE_ID.store(value, Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn c_get_audio_engine_device_id() -> SDL_AudioDeviceID {
    AUDIO_ENGINE_DEVICE_ID.load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn c_audio_engine_ss_initialized() -> bool {
    AUDIO_ENGINE_DEVICE_ID.load(Ordering::Relaxed) != u32::MAX
}

#[no_mangle]
pub extern "C" fn c_get_locked_audio_engine_sound_buffers(index: c_uint) -> *const AudioEngineSoundBuffer {
    let buffer = &AUDIO_ENGINE_SOUND_BUFFER[index as usize];
    let _lock = buffer.mutex.lock();
    forget(_lock);

    buffer
}

#[no_mangle]
pub extern "C" fn c_release_audio_engine_sound_buffers(buffer: *const AudioEngineSoundBuffer) {
    unsafe {
        (*buffer).mutex.force_unlock()
    }
}
