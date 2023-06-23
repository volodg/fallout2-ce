#include "dinput.h"

extern "C"
{
    bool c_direct_input_init();
    void c_direct_input_free();
    bool c_mouse_device_acquire();
    bool c_mouse_device_unacquire();

    void c_set_g_mouse_wheel_delta_x(int value);
    int c_get_g_mouse_wheel_delta_x();
    void c_set_g_mouse_wheel_delta_y(int value);
    int c_get_g_mouse_wheel_delta_y();

    bool c_mouse_device_get_data(fallout::MouseData* mouseState);
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
    SDL_FlushEvents(SDL_KEYDOWN, SDL_TEXTINPUT);
    return true;
}

void handleMouseEvent(SDL_Event* event)
{
    // Mouse movement and buttons are accumulated in SDL itself and will be
    // processed later in `mouseDeviceGetData` via `SDL_GetRelativeMouseState`.

    if (event->type == SDL_MOUSEWHEEL) {
        c_set_g_mouse_wheel_delta_x(c_get_g_mouse_wheel_delta_x() + event->wheel.x);
        c_set_g_mouse_wheel_delta_y(c_get_g_mouse_wheel_delta_y() + event->wheel.y);
    }
}

} // namespace fallout
