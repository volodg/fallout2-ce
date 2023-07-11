#include <SDL.h>

#ifndef _WIN32
#include <unistd.h>
#else
#include "win32.h"
#include "svga.h"
#include "window_manager.h"
#endif

#include "main.h"

#if __APPLE__ && TARGET_OS_IOS
#include "platform/ios/paths.h"
#endif

extern "C"
{
    void rust_c_set_program_is_active(bool value);
}

namespace fallout {

#ifdef _WIN32
// 0x51E444

// GNW95MUTEX
HANDLE _GNW95_mutex = NULL;

// 0x4DE700
int main(int argc, char* argv[])
{
    _GNW95_mutex = CreateMutexA(0, TRUE, "GNW95MUTEX");
    if (GetLastError() == ERROR_SUCCESS) {
        SDL_ShowCursor(SDL_DISABLE);

        c_set_program_is_active(true);
        falloutMain(argc, argv);

        CloseHandle(_GNW95_mutex);
    }
    return 0;
}
#else

int main(int argc, char* argv[])
{
#if __APPLE__ && TARGET_OS_IOS
    SDL_SetHint(SDL_HINT_MOUSE_TOUCH_EVENTS, "0");
    SDL_SetHint(SDL_HINT_TOUCH_MOUSE_EVENTS, "0");
    chdir(iOSGetDocumentsPath());
#endif

#if __APPLE__ && TARGET_OS_OSX
    char* basePath = SDL_GetBasePath();
    chdir(basePath);
    SDL_free(basePath);
#endif

#if __ANDROID__
    SDL_SetHint(SDL_HINT_MOUSE_TOUCH_EVENTS, "0");
    SDL_SetHint(SDL_HINT_TOUCH_MOUSE_EVENTS, "0");
    chdir(SDL_AndroidGetExternalStoragePath());
#endif

    SDL_ShowCursor(SDL_DISABLE);
    rust_c_set_program_is_active(true);
    return falloutMain(argc, argv);
}
#endif

} // namespace fallout

int main(int argc, char* argv[])
{
    return fallout::main(argc, argv);
}
