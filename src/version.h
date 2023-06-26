#ifndef VERSION_H
#define VERSION_H

#include <stddef.h>

// The size of buffer for version string.
// Duplicated rust constant
#define VERSION_MAX (32)

namespace fallout {

void versionGetVersion(char* dest, size_t size);

} // namespace fallout

#endif /* VERSION_H */
