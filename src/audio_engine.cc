#include "audio_engine.h"

#include <string>

#include <SDL.h>

namespace fallout {
struct AudioEngineSoundBuffer;
} // namespace fallout

extern "C" {
    bool c_audio_engine_is_initialized();

    fallout::AudioEngineSoundBuffer* c_get_locked_audio_engine_sound_buffers(unsigned int index);
    void c_release_audio_engine_sound_buffers(unsigned int index);

    bool c_sound_buffer_is_valid(int);

    bool rust_audio_engine_init();
    void rust_audio_engine_exit();
    void rust_audio_engine_pause();
    void rust_audio_engine_resume();
    int rust_audio_engine_create_sound_buffer(unsigned int size, int bitsPerSample, int channels, int rate);
    bool rust_audio_engine_sound_release(int soundBufferIndex);
    bool rust_audio_engine_sound_buffer_set_volume(int soundBufferIndex, int volume);
    bool rust_audio_engine_sound_buffer_get_volume(int soundBufferIndex, int* volumePtr);
    bool rust_audio_engine_sound_buffer_set_pan(int soundBufferIndex, int volume);
    bool rust_audio_engine_sound_buffer_play(int soundBufferIndex, unsigned int flags);
    bool rust_audio_engine_sound_buffer_stop(int soundBufferIndex);
    bool rust_audio_engine_sound_buffer_get_current_position(int soundBufferIndex, unsigned int* readPosPtr, unsigned int* writePosPtr);
    // rust_audio_engine_sound_buffer_get_current_position
}

namespace fallout {

class OnExit {
public:
    OnExit(std::function<void(void)>&& onExit): onExit_(onExit) {}

    ~OnExit() {
        onExit_();
    }
private:
    std::function<void(void)> onExit_;
};

struct AudioEngineSoundBuffer {
    bool active;
    unsigned int size;
    int _reserved;
    int channels;
    int rate;
    int volume;
    bool playing;
    bool looping;
    unsigned int pos;
    void* data;
    SDL_AudioStream* stream;
};

bool audioEngineInit()
{
    return rust_audio_engine_init();
}

void audioEngineExit()
{
    rust_audio_engine_exit();
}

void audioEnginePause()
{
    rust_audio_engine_pause();
}

void audioEngineResume()
{
    rust_audio_engine_resume();
}

int audioEngineCreateSoundBuffer(unsigned int size, int bitsPerSample, int channels, int rate)
{
    return rust_audio_engine_create_sound_buffer(size, bitsPerSample, channels, rate);
}

bool audioEngineSoundBufferRelease(int soundBufferIndex)
{
    return rust_audio_engine_sound_release(soundBufferIndex);
}

bool audioEngineSoundBufferSetVolume(int soundBufferIndex, int volume)
{
    return rust_audio_engine_sound_buffer_set_volume(soundBufferIndex, volume);
}

bool audioEngineSoundBufferGetVolume(int soundBufferIndex, int* volumePtr)
{
    return rust_audio_engine_sound_buffer_get_volume(soundBufferIndex, volumePtr);
}

bool audioEngineSoundBufferSetPan(int soundBufferIndex, int pan)
{
    return rust_audio_engine_sound_buffer_set_pan(soundBufferIndex, pan);
}

bool audioEngineSoundBufferPlay(int soundBufferIndex, unsigned int flags)
{
    return rust_audio_engine_sound_buffer_play(soundBufferIndex, flags);
}

bool audioEngineSoundBufferStop(int soundBufferIndex)
{
    return rust_audio_engine_sound_buffer_stop(soundBufferIndex);
}

bool audioEngineSoundBufferGetCurrentPosition(int soundBufferIndex, unsigned int* readPosPtr, unsigned int* writePosPtr)
{
    return rust_audio_engine_sound_buffer_get_current_position(soundBufferIndex, readPosPtr, writePosPtr);
}

bool audioEngineSoundBufferSetCurrentPosition(int soundBufferIndex, unsigned int pos)
{
    if (!c_audio_engine_is_initialized()) {
        return false;
    }

    if (!c_sound_buffer_is_valid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBufferIndex]() {
        c_release_audio_engine_sound_buffers(soundBufferIndex);
    });

    if (!soundBuffer->active) {
        return false;
    }

    soundBuffer->pos = pos % soundBuffer->size;

    return true;
}

bool audioEngineSoundBufferLock(int soundBufferIndex, unsigned int writePos, unsigned int writeBytes, void** audioPtr1, unsigned int* audioBytes1, void** audioPtr2, unsigned int* audioBytes2, unsigned int flags)
{
    if (!c_audio_engine_is_initialized()) {
        return false;
    }

    if (!c_sound_buffer_is_valid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBufferIndex]() {
        c_release_audio_engine_sound_buffers(soundBufferIndex);
    });

    if (!soundBuffer->active) {
        return false;
    }

    if (audioBytes1 == nullptr) {
        return false;
    }

    if ((flags & AUDIO_ENGINE_SOUND_BUFFER_LOCK_FROM_WRITE_POS) != 0) {
        if (!audioEngineSoundBufferGetCurrentPosition(soundBufferIndex, NULL, &writePos)) {
            return false;
        }
    }

    if ((flags & AUDIO_ENGINE_SOUND_BUFFER_LOCK_ENTIRE_BUFFER) != 0) {
        writeBytes = soundBuffer->size;
    }

    if (writePos + writeBytes <= soundBuffer->size) {
        *(unsigned char**)audioPtr1 = (unsigned char*)soundBuffer->data + writePos;
        *audioBytes1 = writeBytes;

        if (audioPtr2 != nullptr) {
            *audioPtr2 = nullptr;
        }

        if (audioBytes2 != nullptr) {
            *audioBytes2 = 0;
        }
    } else {
        *(unsigned char**)audioPtr1 = (unsigned char*)soundBuffer->data + writePos;
        *audioBytes1 = soundBuffer->size - writePos;

        if (audioPtr2 != nullptr) {
            *(unsigned char**)audioPtr2 = (unsigned char*)soundBuffer->data;
        }

        if (audioBytes2 != nullptr) {
            *audioBytes2 = writeBytes - (soundBuffer->size - writePos);
        }
    }

    // TODO: Mark range as locked.

    return true;
}

bool audioEngineSoundBufferUnlock(int soundBufferIndex)
{
    if (!c_audio_engine_is_initialized()) {
        return false;
    }

    if (!c_sound_buffer_is_valid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBufferIndex]() {
        c_release_audio_engine_sound_buffers(soundBufferIndex);
    });

    if (!soundBuffer->active) {
        return false;
    }

    // TODO: Mark range as unlocked.

    return true;
}

bool audioEngineSoundBufferGetStatus(int soundBufferIndex, unsigned int* statusPtr)
{
    if (!c_audio_engine_is_initialized()) {
        return false;
    }

    if (!c_sound_buffer_is_valid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBufferIndex]() {
        c_release_audio_engine_sound_buffers(soundBufferIndex);
    });

    if (!soundBuffer->active) {
        return false;
    }

    if (statusPtr == nullptr) {
        return false;
    }

    *statusPtr = 0;

    if (soundBuffer->playing) {
        *statusPtr |= AUDIO_ENGINE_SOUND_BUFFER_STATUS_PLAYING;

        if (soundBuffer->looping) {
            *statusPtr |= AUDIO_ENGINE_SOUND_BUFFER_STATUS_LOOPING;
        }
    }

    return true;
}

} // namespace fallout
