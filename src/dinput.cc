#include "dinput.h"

extern "C"
{
    bool rust_c_direct_input_init();
    void rust_c_direct_input_free();
    bool rust_c_mouse_device_acquire();
    bool rust_c_mouse_device_unacquire();

    bool rust_c_mouse_device_get_data(fallout::MouseData* mouseState);
    bool rust_c_keyboard_device_reset();
    void rust_c_handle_mouse_event(SDL_Event* event);
}

namespace fallout {

// 0x4E0400
bool directInputInit()
{
    return rust_c_direct_input_init();
}

// 0x4E0478
void directInputFree()
{
    rust_c_direct_input_free();
}

// 0x4E04E8
bool mouseDeviceAcquire()
{
    return rust_c_mouse_device_acquire();
}

// 0x4E0514
bool mouseDeviceUnacquire()
{
    return rust_c_mouse_device_unacquire();
}

// 0x4E053C
bool mouseDeviceGetData(MouseData* mouseState)
{
    return rust_c_mouse_device_get_data(mouseState);
}

// 0x4E05FC
bool keyboardDeviceReset()
{
    return rust_c_keyboard_device_reset();
}

void handleMouseEvent(SDL_Event* event)
{
    rust_c_handle_mouse_event(event);
}

} // namespace fallout
