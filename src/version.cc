#include "version.h"

#include <stdio.h>

extern "C"
{
    void c_get_version(char* dest, size_t size);
}

namespace fallout {

// 0x4B4580
void versionGetVersion(char* dest, size_t size)
{
    c_get_version(dest, size);
}

} // namespace fallout
