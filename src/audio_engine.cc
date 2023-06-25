#include "audio_engine.h"

#include <string>

#include <SDL.h>

namespace fallout {
struct AudioEngineSoundBuffer;
} // namespace fallout

extern "C" {
    bool c_get_program_is_active();

    void c_set_audio_engine_device_id(SDL_AudioDeviceID value);
    SDL_AudioDeviceID c_get_audio_engine_device_id();

    bool c_audio_engine_is_initialized();

    fallout::AudioEngineSoundBuffer* c_get_locked_audio_engine_sound_buffers(unsigned int index);
    void c_release_audio_engine_sound_buffers(unsigned int index);

    SDL_AudioSpec* c_get_audio_engine_spec();

    unsigned long c_get_audio_engine_sound_buffers_count();
    bool c_sound_buffer_is_valid(int);

    bool rust_audio_engine_init();
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
    int bitsPerSample;
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
    if (c_audio_engine_is_initialized()) {
        SDL_CloseAudioDevice(c_get_audio_engine_device_id());
        c_set_audio_engine_device_id(-1);
    }

    if (SDL_WasInit(SDL_INIT_AUDIO)) {
        SDL_QuitSubSystem(SDL_INIT_AUDIO);
    }
}

void audioEnginePause()
{
    if (c_audio_engine_is_initialized()) {
        SDL_PauseAudioDevice(c_get_audio_engine_device_id(), 1);
    }
}

void audioEngineResume()
{
    if (c_audio_engine_is_initialized()) {
        SDL_PauseAudioDevice(c_get_audio_engine_device_id(), 0);
    }
}

int audioEngineCreateSoundBuffer(unsigned int size, int bitsPerSample, int channels, int rate)
{
    if (!c_audio_engine_is_initialized()) {
        return -1;
    }

    for (int index = 0; index < c_get_audio_engine_sound_buffers_count(); index++) {
        AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(index);
        OnExit onExit([index]() {
            c_release_audio_engine_sound_buffers(index);
        });

        if (!soundBuffer->active) {
            soundBuffer->active = true;
            soundBuffer->size = size;
            soundBuffer->bitsPerSample = bitsPerSample;
            soundBuffer->channels = channels;
            soundBuffer->rate = rate;
            soundBuffer->volume = SDL_MIX_MAXVOLUME;
            soundBuffer->playing = false;
            soundBuffer->looping = false;
            soundBuffer->pos = 0;
            soundBuffer->data = malloc(size);
            soundBuffer->stream = SDL_NewAudioStream(bitsPerSample == 16 ? AUDIO_S16 : AUDIO_S8, channels, rate, c_get_audio_engine_spec()->format, c_get_audio_engine_spec()->channels, c_get_audio_engine_spec()->freq);
            return index;
        }
    }

    return -1;
}

bool audioEngineSoundBufferRelease(int soundBufferIndex)
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

    soundBuffer->active = false;

    free(soundBuffer->data);
    soundBuffer->data = nullptr;

    SDL_FreeAudioStream(soundBuffer->stream);
    soundBuffer->stream = nullptr;

    return true;
}

bool audioEngineSoundBufferSetVolume(int soundBufferIndex, int volume)
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

    soundBuffer->volume = volume;

    return true;
}

bool audioEngineSoundBufferGetVolume(int soundBufferIndex, int* volumePtr)
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

    *volumePtr = soundBuffer->volume;

    return true;
}

bool audioEngineSoundBufferSetPan(int soundBufferIndex, int pan)
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

    // NOTE: Audio engine does not support sound panning. I'm not sure it's
    // even needed. For now this value is silently ignored.

    return true;
}

bool audioEngineSoundBufferPlay(int soundBufferIndex, unsigned int flags)
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

    soundBuffer->playing = true;

    if ((flags & AUDIO_ENGINE_SOUND_BUFFER_PLAY_LOOPING) != 0) {
        soundBuffer->looping = true;
    }

    return true;
}

bool audioEngineSoundBufferStop(int soundBufferIndex)
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

    soundBuffer->playing = false;

    return true;
}

bool audioEngineSoundBufferGetCurrentPosition(int soundBufferIndex, unsigned int* readPosPtr, unsigned int* writePosPtr)
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

    if (readPosPtr != nullptr) {
        *readPosPtr = soundBuffer->pos;
    }

    if (writePosPtr != nullptr) {
        *writePosPtr = soundBuffer->pos;

        if (soundBuffer->playing) {
            // 15 ms lead
            // See: https://docs.microsoft.com/en-us/previous-versions/windows/desktop/mt708925(v=vs.85)#remarks
            *writePosPtr += soundBuffer->rate / 150;
            *writePosPtr %= soundBuffer->size;
        }
    }

    return true;
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

    if (audioBytes1 == NULL) {
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
