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

    bool c_audio_engine_ss_initialized();

    fallout::AudioEngineSoundBuffer* c_get_locked_audio_engine_sound_buffers(unsigned int index);
    void c_release_audio_engine_sound_buffers(fallout::AudioEngineSoundBuffer* ptr);
}

namespace fallout {

class OnExit {
public:
    OnExit(std::function<void(void)>&& onExit): onExit_(onExit) {
    }

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

#define AUDIO_ENGINE_SOUND_BUFFERS 8

static bool soundBufferIsValid(int soundBufferIndex);
static void audioEngineMixin(void* userData, Uint8* stream, int length);

static SDL_AudioSpec gAudioEngineSpec;
static AudioEngineSoundBuffer gAudioEngineSoundBuffers[AUDIO_ENGINE_SOUND_BUFFERS];

static bool audioEngineIsInitialized()
{
    return c_audio_engine_ss_initialized();
}

static bool soundBufferIsValid(int soundBufferIndex)
{
    return soundBufferIndex >= 0 && soundBufferIndex < AUDIO_ENGINE_SOUND_BUFFERS;
}

static void audioEngineMixin(void* userData, Uint8* stream, int length)
{
    memset(stream, gAudioEngineSpec.silence, length);

    if (!c_get_program_is_active()) {
        return;
    }

    for (int index = 0; index < AUDIO_ENGINE_SOUND_BUFFERS; index++) {
        AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(index);
        OnExit onExit([soundBuffer]() {
            c_release_audio_engine_sound_buffers(soundBuffer);
        });

        if (soundBuffer->active && soundBuffer->playing) {
            int srcFrameSize = soundBuffer->bitsPerSample / 8 * soundBuffer->channels;

            unsigned char buffer[1024];
            int pos = 0;
            while (pos < length) {
                int remaining = length - pos;
                if (remaining > sizeof(buffer)) {
                    remaining = sizeof(buffer);
                }

                // TODO: Make something better than frame-by-frame convertion.
                SDL_AudioStreamPut(soundBuffer->stream, (unsigned char*)soundBuffer->data + soundBuffer->pos, srcFrameSize);
                soundBuffer->pos += srcFrameSize;

                int bytesRead = SDL_AudioStreamGet(soundBuffer->stream, buffer, remaining);
                if (bytesRead == -1) {
                    break;
                }

                SDL_MixAudioFormat(stream + pos, buffer, gAudioEngineSpec.format, bytesRead, soundBuffer->volume);

                if (soundBuffer->pos >= soundBuffer->size) {
                    if (soundBuffer->looping) {
                        soundBuffer->pos %= soundBuffer->size;
                    } else {
                        soundBuffer->playing = false;
                        break;
                    }
                }

                pos += bytesRead;
            }
        }
    }
}

bool audioEngineInit()
{
    if (SDL_InitSubSystem(SDL_INIT_AUDIO) == -1) {
        return false;
    }

    SDL_AudioSpec desiredSpec;
    desiredSpec.freq = 22050;
    desiredSpec.format = AUDIO_S16;
    desiredSpec.channels = 2;
    desiredSpec.samples = 1024;
    desiredSpec.callback = audioEngineMixin;

    c_set_audio_engine_device_id(SDL_OpenAudioDevice(nullptr, 0, &desiredSpec, &gAudioEngineSpec, SDL_AUDIO_ALLOW_ANY_CHANGE));
    if (!audioEngineIsInitialized()) {
        return false;
    }

    SDL_PauseAudioDevice(c_get_audio_engine_device_id(), 0);

    return true;
}

void audioEngineExit()
{
    if (audioEngineIsInitialized()) {
        SDL_CloseAudioDevice(c_get_audio_engine_device_id());
        c_set_audio_engine_device_id(-1);
    }

    if (SDL_WasInit(SDL_INIT_AUDIO)) {
        SDL_QuitSubSystem(SDL_INIT_AUDIO);
    }
}

void audioEnginePause()
{
    if (audioEngineIsInitialized()) {
        SDL_PauseAudioDevice(c_get_audio_engine_device_id(), 1);
    }
}

void audioEngineResume()
{
    if (audioEngineIsInitialized()) {
        SDL_PauseAudioDevice(c_get_audio_engine_device_id(), 0);
    }
}

int audioEngineCreateSoundBuffer(unsigned int size, int bitsPerSample, int channels, int rate)
{
    if (!audioEngineIsInitialized()) {
        return -1;
    }

    for (int index = 0; index < AUDIO_ENGINE_SOUND_BUFFERS; index++) {
        AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(index);
        OnExit onExit([soundBuffer]() {
            c_release_audio_engine_sound_buffers(soundBuffer);
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
            soundBuffer->stream = SDL_NewAudioStream(bitsPerSample == 16 ? AUDIO_S16 : AUDIO_S8, channels, rate, gAudioEngineSpec.format, gAudioEngineSpec.channels, gAudioEngineSpec.freq);
            return index;
        }
    }

    return -1;
}

bool audioEngineSoundBufferRelease(int soundBufferIndex)
{
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
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
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
    });

    if (!soundBuffer->active) {
        return false;
    }

    soundBuffer->volume = volume;

    return true;
}

bool audioEngineSoundBufferGetVolume(int soundBufferIndex, int* volumePtr)
{
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
    });

    if (!soundBuffer->active) {
        return false;
    }

    *volumePtr = soundBuffer->volume;

    return true;
}

bool audioEngineSoundBufferSetPan(int soundBufferIndex, int pan)
{
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
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
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
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
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
    });

    if (!soundBuffer->active) {
        return false;
    }

    soundBuffer->playing = false;

    return true;
}

bool audioEngineSoundBufferGetCurrentPosition(int soundBufferIndex, unsigned int* readPosPtr, unsigned int* writePosPtr)
{
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
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
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
    });

    if (!soundBuffer->active) {
        return false;
    }

    soundBuffer->pos = pos % soundBuffer->size;

    return true;
}

bool audioEngineSoundBufferLock(int soundBufferIndex, unsigned int writePos, unsigned int writeBytes, void** audioPtr1, unsigned int* audioBytes1, void** audioPtr2, unsigned int* audioBytes2, unsigned int flags)
{
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
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
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
    });

    if (!soundBuffer->active) {
        return false;
    }

    // TODO: Mark range as unlocked.

    return true;
}

bool audioEngineSoundBufferGetStatus(int soundBufferIndex, unsigned int* statusPtr)
{
    if (!audioEngineIsInitialized()) {
        return false;
    }

    if (!soundBufferIsValid(soundBufferIndex)) {
        return false;
    }

    AudioEngineSoundBuffer* soundBuffer = c_get_locked_audio_engine_sound_buffers(soundBufferIndex);
    OnExit onExit([soundBuffer]() {
        c_release_audio_engine_sound_buffers(soundBuffer);
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
