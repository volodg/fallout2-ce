#ifndef WIN32_H
#define WIN32_H

#ifdef _WIN32
#include <windows.h>
#endif

namespace fallout {

#ifdef _WIN32
extern HANDLE _GNW95_mutex;
#endif

} // namespace fallout

#endif /* WIN32_H */
