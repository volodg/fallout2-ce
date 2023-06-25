use lazy_static::lazy_static;
use libc::{c_uchar, c_uint, c_ulong, memset, size_t};
use std::ffi::{c_int, c_void};
use std::mem::forget;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU32, Ordering};use std::sync::Mutex;
use sdl2_sys::{SDL_AudioFormat, SDL_AudioSpec, SDL_AudioStreamGet, SDL_AudioStreamPut, SDL_MixAudioFormat, u_char};
use parking_lot::ReentrantMutex;
use sdl2::sys::{SDL_AudioDeviceID, SDL_AudioStream};
use crate::win32::program_is_active;

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
    stream: *mut SDL_AudioStream,
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
            stream: null_mut(),
            mutex: ReentrantMutex::new(())
        }
    }
}

const AUDIO_ENGINE_SOUND_BUFFERS: usize = 8;

unsafe impl Send for AudioEngineSoundBuffer {}
unsafe impl Sync for AudioEngineSoundBuffer {}

struct SdlAudioSpecHolder {
    obj: SDL_AudioSpec
}

impl Default for SdlAudioSpecHolder {
    fn default() -> Self {
        SdlAudioSpecHolder {
            obj: SDL_AudioSpec {
                freq: 0,
                format: 0 as SDL_AudioFormat,
                channels: 0,
                silence: 0,
                samples: 0,
                padding: 0,
                size: 0,
                callback: None,
                userdata: null_mut()
            }
        }
    }
}

unsafe impl Send for SdlAudioSpecHolder {}
unsafe impl Sync for SdlAudioSpecHolder {}

lazy_static! {
    static ref AUDIO_ENGINE_SOUND_BUFFER: Mutex<[AudioEngineSoundBuffer; AUDIO_ENGINE_SOUND_BUFFERS]> = Mutex::new([
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
        AudioEngineSoundBuffer::default(),
    ]);

    static ref AUDIO_ENGINE_SPEC: SdlAudioSpecHolder = SdlAudioSpecHolder::default();
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
pub extern "C" fn c_audio_engine_is_initialized() -> bool {
    AUDIO_ENGINE_DEVICE_ID.load(Ordering::Relaxed) != u32::MAX
}

#[no_mangle]
pub extern "C" fn c_get_locked_audio_engine_sound_buffers(index: c_uint) -> *const AudioEngineSoundBuffer {
    let buffer = &AUDIO_ENGINE_SOUND_BUFFER.lock().expect("locked")[index as usize];

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

#[no_mangle]
pub extern "C" fn c_get_audio_engine_spec() -> *const SDL_AudioSpec {
    &AUDIO_ENGINE_SPEC.obj
}

#[no_mangle]
pub extern "C" fn c_get_audio_engine_sound_buffers_count() -> c_ulong {
    AUDIO_ENGINE_SOUND_BUFFERS as c_ulong
}

#[no_mangle]
pub extern "C" fn c_sound_buffer_is_valid(sound_buffer_index: c_int) -> bool {
    sound_buffer_index >= 0 && (sound_buffer_index as usize) < AUDIO_ENGINE_SOUND_BUFFERS
}

#[no_mangle]
pub extern "C" fn c_audio_engine_mixin(_user_data: *const c_void, stream: *mut u8, length: c_int) {
    unsafe {
        memset(stream as *mut c_void, AUDIO_ENGINE_SPEC.obj.silence as c_int, length as size_t);
    }

    if !program_is_active() {
        return;
    }

    for sound_buffer in AUDIO_ENGINE_SOUND_BUFFER.lock().expect("locked").iter_mut() {
        let _lock = sound_buffer.mutex.lock();

        if sound_buffer.active && sound_buffer.playing {
            let src_frame_size = sound_buffer.bits_per_sample / 8 * sound_buffer.channels;

            let mut buffer: [c_uchar; 1024] = ['\0' as u_char; 1024];
            let mut pos = 0;
            while pos < length {
                let mut remaining = length - pos;
                if remaining > buffer.len() as c_int {
                    remaining = buffer.len() as c_int;
                }

                // TODO: Make something better than frame-by-frame convertion.
                unsafe {
                    SDL_AudioStreamPut(sound_buffer.stream, sound_buffer.data.add(sound_buffer.pos as usize), src_frame_size);
                }
                sound_buffer.pos += src_frame_size as u32;

                let bytes_read = unsafe {
                    SDL_AudioStreamGet(sound_buffer.stream, buffer.as_mut_ptr() as *mut c_void, remaining)
                };
                if bytes_read == -1 {
                    break;
                }

                unsafe {
                    SDL_MixAudioFormat(stream.add(pos as usize), buffer.as_mut_ptr(), AUDIO_ENGINE_SPEC.obj.format, bytes_read as u32, sound_buffer.volume);
                }

                if sound_buffer.pos >= sound_buffer.size {
                    if sound_buffer.looping {
                        sound_buffer.pos %= sound_buffer.size;
                    } else {
                        sound_buffer.playing = false;
                        break;
                    }
                }

                pos += bytes_read;
            }
        }
    }
}

/*
static void audioEngineMixin(void* userData, Uint8* stream, int length)
{
    for (int index = 0; index < c_get_audio_engine_sound_buffers_count(); index++) {
        if (soundBuffer->active && soundBuffer->playing) {
        }
    }
}
 */