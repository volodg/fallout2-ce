#include "platform_compat.h"

#ifdef _WIN32
#include <windows.h>
#endif

#ifdef _WIN32
#include <direct.h>
#include <io.h>
#include <stdio.h>
#include <stdlib.h>
#endif

#ifdef _WIN32
#include <timeapi.h>
#endif

extern "C" {
    int rust_compat_stricmp(const char* string1, const char* string2);
    void rust_compat_splitpath(const char* path, char* drive, char* dir, char* fname, char* ext);
    int rust_compat_strnicmp(const char* string1, const char* string2, unsigned long size);
    char* rust_compat_strupr(char* string);
    char* rust_compat_strlwr(char* string);
    char* rust_compat_itoa(int value, char* buffer, int radix);
    void rust_compat_makepath(char* path, const char* drive, const char* dir, const char* fname, const char* ext);
    long rust_compat_tell(int fd);
    void rust_compat_windows_path_to_native(char* path);
    void rust_compat_resolve_path(char* path);
    int rust_compat_mkdir(const char* path);
    unsigned int rust_compat_time_get_time();
    FILE* rust_compat_fopen(const char* path, const char* mode);
    gzFile rust_compat_gzopen(const char* path, const char* mode);
    char* rust_compat_fgets(char* buffer, int maxCount, FILE* stream);
    char* rust_compat_gzgets(gzFile stream, char* buffer, int maxCount);
    int rust_compat_remove(const char* path);
    int rust_compat_rename(const char* oldFileName, const char* newFileName);
    int rust_compat_access(const char* path, int mode);
    char* rust_compat_strdup(const char* string);
    long rust_get_file_size(FILE* stream);
}

namespace fallout {

int compat_stricmp(const char* string1, const char* string2)
{
    return rust_compat_stricmp(string1, string2);
}

int compat_strnicmp(const char* string1, const char* string2, unsigned long size)
{
    return rust_compat_strnicmp(string1, string2, size);
}

char* compat_strupr(char* string)
{
    return rust_compat_strupr(string);
}

char* compat_strlwr(char* string)
{
    return rust_compat_strlwr(string);
}

char* compat_itoa(int value, char* buffer, int radix)
{
    return rust_compat_itoa(value, buffer, radix);
}

void compat_splitpath(const char* path, char* drive, char* dir, char* fname, char* ext)
{
    rust_compat_splitpath(path, drive, dir, fname, ext);
}

void compat_makepath(char* path, const char* drive, const char* dir, const char* fname, const char* ext)
{
    rust_compat_makepath(path, drive, dir, fname, ext);
}

long compat_tell(int fd)
{
    return rust_compat_tell(fd);
}

int compat_mkdir(const char* path)
{
    return rust_compat_mkdir(path);
}

unsigned int compat_timeGetTime()
{
    return rust_compat_time_get_time();
}

FILE* compat_fopen(const char* path, const char* mode)
{
    return rust_compat_fopen(path, mode);
}

gzFile compat_gzopen(const char* path, const char* mode)
{
    return rust_compat_gzopen(path, mode);
}

char* compat_fgets(char* buffer, int maxCount, FILE* stream)
{
    return rust_compat_fgets(buffer, maxCount, stream);
}

char* compat_gzgets(gzFile stream, char* buffer, int maxCount)
{
    return rust_compat_gzgets(stream, buffer, maxCount);
}

int compat_remove(const char* path)
{
    return rust_compat_remove(path);
}

int compat_rename(const char* oldFileName, const char* newFileName)
{
    return rust_compat_rename(oldFileName, newFileName);
}

void compat_windows_path_to_native(char* path)
{
    rust_compat_windows_path_to_native(path);
}

void compat_resolve_path(char* path)
{
    rust_compat_resolve_path(path);
}

int compat_access(const char* path, int mode)
{
    return rust_compat_access(path, mode);
}

char* compat_strdup(const char* string)
{
    return rust_compat_strdup(string);
}

long getFileSize(FILE* stream)
{
    return rust_get_file_size(stream);
}

} // namespace fallout
