#include "dinput.h"

extern "C"
{
    bool c_direct_input_init();
    void c_direct_input_free();
    bool c_mouse_device_acquire();
    bool c_mouse_device_unacquire();

    bool c_mouse_device_get_data(fallout::MouseData* mouseState);
    bool c_keyboard_device_reset();
    void c_handle_mouse_event(SDL_Event* event);
}

namespace fallout {

// 0x4E0400
bool directInputInit()
{
    return c_direct_input_init();
}

// 0x4E0478
void directInputFree()
{
    c_direct_input_free();
}

// 0x4E04E8
bool mouseDeviceAcquire()
{
    return c_mouse_device_acquire();
}

// 0x4E0514
bool mouseDeviceUnacquire()
{
    return c_mouse_device_unacquire();
}

// 0x4E053C
bool mouseDeviceGetData(MouseData* mouseState)
{
    return c_mouse_device_get_data(mouseState);
}

// 0x4E05FC
bool keyboardDeviceReset()
{
    return c_keyboard_device_reset();
}

void handleMouseEvent(SDL_Event* event)
{
    c_handle_mouse_event(event);
}

} // namespace fallout
