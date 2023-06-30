#include "platform_compat.h"

#include <cstring>

#ifdef _WIN32
#include <windows.h>
#endif

#ifdef _WIN32
#include <direct.h>
#include <io.h>
#include <stdio.h>
#include <stdlib.h>
#else
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#endif

#ifdef _WIN32
#include <timeapi.h>
#else
#include <chrono>
#endif

#include <SDL.h>

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
    char nativePath[COMPAT_MAX_PATH];
    strcpy(nativePath, path);
    rust_compat_windows_path_to_native(nativePath);
    compat_resolve_path(nativePath);

#ifdef _WIN32
    return mkdir(nativePath);
#else
    return mkdir(nativePath, 0755);
#endif
}

unsigned int compat_timeGetTime()
{
#ifdef _WIN32
    return timeGetTime();
#else
    static auto start = std::chrono::steady_clock::now();
    auto now = std::chrono::steady_clock::now();
    return static_cast<unsigned int>(std::chrono::duration_cast<std::chrono::milliseconds>(now - start).count());
#endif
}

FILE* compat_fopen(const char* path, const char* mode)
{
    char nativePath[COMPAT_MAX_PATH];
    strcpy(nativePath, path);
    rust_compat_windows_path_to_native(nativePath);
    compat_resolve_path(nativePath);
    return fopen(nativePath, mode);
}

gzFile compat_gzopen(const char* path, const char* mode)
{
    char nativePath[COMPAT_MAX_PATH];
    strcpy(nativePath, path);
    rust_compat_windows_path_to_native(nativePath);
    compat_resolve_path(nativePath);
    return gzopen(nativePath, mode);
}

char* compat_fgets(char* buffer, int maxCount, FILE* stream)
{
    buffer = fgets(buffer, maxCount, stream);

    if (buffer != nullptr) {
        size_t len = strlen(buffer);
        if (len >= 2 && buffer[len - 1] == '\n' && buffer[len - 2] == '\r') {
            buffer[len - 2] = '\n';
            buffer[len - 1] = '\0';
        }
    }

    return buffer;
}

char* compat_gzgets(gzFile stream, char* buffer, int maxCount)
{
    buffer = gzgets(stream, buffer, maxCount);

    if (buffer != nullptr) {
        size_t len = strlen(buffer);
        if (len >= 2 && buffer[len - 1] == '\n' && buffer[len - 2] == '\r') {
            buffer[len - 2] = '\n';
            buffer[len - 1] = '\0';
        }
    }

    return buffer;
}

int compat_remove(const char* path)
{
    char nativePath[COMPAT_MAX_PATH];
    strcpy(nativePath, path);
    rust_compat_windows_path_to_native(nativePath);
    compat_resolve_path(nativePath);
    return remove(nativePath);
}

int compat_rename(const char* oldFileName, const char* newFileName)
{
    char nativeOldFileName[COMPAT_MAX_PATH];
    strcpy(nativeOldFileName, oldFileName);
    rust_compat_windows_path_to_native(nativeOldFileName);
    compat_resolve_path(nativeOldFileName);

    char nativeNewFileName[COMPAT_MAX_PATH];
    strcpy(nativeNewFileName, newFileName);
    rust_compat_windows_path_to_native(nativeNewFileName);
    compat_resolve_path(nativeNewFileName);

    return rename(nativeOldFileName, nativeNewFileName);
}

void compat_windows_path_to_native(char* path)
{
    rust_compat_windows_path_to_native(path);
}

void compat_resolve_path(char* path)
{
#ifndef _WIN32
    char* pch = path;

    DIR* dir;
    if (pch[0] == '/') {
        dir = opendir("/");
        pch++;
    } else {
        dir = opendir(".");
    }

    while (dir != nullptr) {
        char* sep = strchr(pch, '/');
        size_t length;
        if (sep != nullptr) {
            length = sep - pch;
        } else {
            length = strlen(pch);
        }

        bool found = false;

        struct dirent* entry = readdir(dir);
        while (entry != nullptr) {
            if (strlen(entry->d_name) == length && compat_strnicmp(pch, entry->d_name, length) == 0) {
                strncpy(pch, entry->d_name, length);
                found = true;
                break;
            }
            entry = readdir(dir);
        }

        closedir(dir);
        dir = nullptr;

        if (!found) {
            break;
        }

        if (sep == nullptr) {
            break;
        }

        *sep = '\0';
        dir = opendir(path);
        *sep = '/';

        pch = sep + 1;
    }
#endif
}

int compat_access(const char* path, int mode)
{
    char nativePath[COMPAT_MAX_PATH];
    strcpy(nativePath, path);
    rust_compat_windows_path_to_native(nativePath);
    compat_resolve_path(nativePath);
    return access(nativePath, mode);
}

char* compat_strdup(const char* string)
{
    return SDL_strdup(string);
}

// It's a replacement for compat_filelength(fileno(stream)) on platforms without
// fileno defined.
long getFileSize(FILE* stream)
{
    long originalOffset = ftell(stream);
    fseek(stream, 0, SEEK_END);
    long filesize = ftell(stream);
    fseek(stream, originalOffset, SEEK_SET);
    return filesize;
}

} // namespace fallout
