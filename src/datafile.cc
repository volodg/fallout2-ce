#include "datafile.h"

#include <cstring>

#include "color.h"
#include "db.h"
#include "memory_manager.h"
#include "pcx.h"

// Migrated
#include "platform_compat.h"

namespace fallout {

// 0x5184AC
DatafileLoader* gDatafileLoader = nullptr;

// 0x5184B0
DatafileNameMangler* gDatafileNameMangler = datafileDefaultNameManglerImpl;

// 0x56D7E0
unsigned char gDatafilePalette[768];

// 0x42EE70
char* datafileDefaultNameManglerImpl(char* path)
{
    return path;
}

// 0x42EE84
void sub_42EE84(unsigned char* data, unsigned char* palette, int width, int height)
{
    unsigned char indexedPalette[256];

    indexedPalette[0] = 0;
    for (int index = 1; index < 256; index++) {
        // TODO: Check.
        int r = palette[index * 3 + 2] >> 3;
        int g = palette[index * 3 + 1] >> 3;
        int b = palette[index * 3] >> 3;
        int colorTableIndex = (r << 10) | (g << 5) | b;
        indexedPalette[index] = _colorTable[colorTableIndex];
    }

    int size = width * height;
    for (int index = 0; index < size; index++) {
        data[index] = indexedPalette[data[index]];
    }
}

// 0x42EF60
unsigned char* datafileReadRaw(char* path, int* widthPtr, int* heightPtr)
{
    char* mangledPath = gDatafileNameMangler(path);
    char* dot = strrchr(mangledPath, '.');
    if (dot != NULL) {
        if (compat_stricmp(dot + 1, "pcx") == 0) {
            return pcxRead(mangledPath, widthPtr, heightPtr, gDatafilePalette);
        }
    }

    if (gDatafileLoader != NULL) {
        return gDatafileLoader(mangledPath, gDatafilePalette, widthPtr, heightPtr);
    }

    return NULL;
}

// 0x42EFCC
unsigned char* datafileRead(char* path, int* widthPtr, int* heightPtr)
{
    unsigned char* v1 = datafileReadRaw(path, widthPtr, heightPtr);
    if (v1 != NULL) {
        sub_42EE84(v1, gDatafilePalette, *widthPtr, *heightPtr);
    }
    return v1;
}

// 0x42F0E4
unsigned char* datafileGetPalette()
{
    return gDatafilePalette;
}

} // namespace fallout
