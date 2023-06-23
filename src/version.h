#ifndef VERSION_H
#define VERSION_H

#include <stddef.h>

namespace fallout {

// The size of buffer for version string.
#define VERSION_MAX (32)

#define VERSION_RELEASE ('R')
#define VERSION_BUILD_TIME ("Dec 11 1998 16:54:30")

void versionGetVersion(char* dest, size_t size);

} // namespace fallout

#endif /* VERSION_H */
