#include "audio_engine.h"

extern "C" {
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
    bool rust_audio_engine_sound_buffer_set_current_position(int soundBufferIndex, unsigned int pos);
    bool rust_audio_engine_sound_buffer_lock(int soundBufferIndex, unsigned int writePos, unsigned int writeBytes, void** audioPtr1, unsigned int* audioBytes1, void** audioPtr2, unsigned int* audioBytes2, unsigned int flags);
    bool rust_audio_engine_sound_buffer_unlock(int soundBufferIndex);
    bool rust_audio_engine_sound_buffer_get_status(int soundBufferIndex, unsigned int* statusPtr);
}

namespace fallout {

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
    return rust_audio_engine_sound_buffer_set_current_position(soundBufferIndex, pos);
}

bool audioEngineSoundBufferLock(int soundBufferIndex, unsigned int writePos, unsigned int writeBytes, void** audioPtr1, unsigned int* audioBytes1, void** audioPtr2, unsigned int* audioBytes2, unsigned int flags)
{
    return rust_audio_engine_sound_buffer_lock(soundBufferIndex, writePos, writeBytes, audioPtr1, audioBytes1, audioPtr2, audioBytes2, flags);
}

bool audioEngineSoundBufferUnlock(int soundBufferIndex)
{
    return rust_audio_engine_sound_buffer_unlock(soundBufferIndex);
}

bool audioEngineSoundBufferGetStatus(int soundBufferIndex, unsigned int* statusPtr)
{
    return rust_audio_engine_sound_buffer_get_status(soundBufferIndex, statusPtr);
}

} // namespace fallout
