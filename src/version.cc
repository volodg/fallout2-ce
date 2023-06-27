#include "version.h"

extern "C"
{
    void rust_get_version(char* dest, size_t size);
}

namespace fallout {

// 0x4B4580
void versionGetVersion(char* dest, size_t size)
{
    rust_get_version(dest, size);
}

} // namespace fallout
