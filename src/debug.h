#ifndef DEBUG_H
#define DEBUG_H

namespace fallout {

typedef int(DebugPrintProc)(char* string);

void _GNW_debug_init();
int debugPrint(const char* format, ...);
void _debug_exit(void);

} // namespace fallout

#endif /* DEBUG_H */
