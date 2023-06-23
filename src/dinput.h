#ifndef DINPUT_H
#define DINPUT_H

#include <SDL.h>

namespace fallout {

typedef struct MouseData {
    int x;
    int y;
    unsigned char buttons[2];
    int wheelX;
    int wheelY;
} MouseData;

typedef struct KeyboardData {
    int key;
    char down;
} KeyboardData;

bool directInputInit();
void directInputFree();
bool mouseDeviceAcquire();
bool mouseDeviceUnacquire();
bool mouseDeviceGetData(MouseData* mouseData);
bool keyboardDeviceReset();

void handleMouseEvent(SDL_Event* event);

} // namespace fallout

#endif /* DINPUT_H */
