#include "file_find.h"

#include <cstring>

extern "C" {
    bool rust_fpattern_match(const char *pat, const char *fname);
    bool rust_file_find_first(const char* path, fallout::DirectoryFileFindData* findData);
}

// TODO Migrate

// TODO migrate
namespace fallout {

// 0x4E6380
bool fileFindFirst(const char* path, DirectoryFileFindData* findData)
{
    return rust_file_find_first(path, findData);
}

// 0x4E63A8
bool fileFindNext(DirectoryFileFindData* findData)
{
#if defined(_WIN32)
    if (!FindNextFileA(findData->hFind, &(findData->ffd))) {
        return false;
    }
#else
    char drive[COMPAT_MAX_DRIVE];
    char dir[COMPAT_MAX_DIR];
    compat_splitpath(findData->path, drive, dir, nullptr, nullptr);

    findData->entry = readdir(findData->dir);
    while (findData->entry != nullptr) {
        char entryPath[COMPAT_MAX_PATH];
        compat_makepath(entryPath, drive, dir, fileFindGetName(findData), nullptr);
        if (rust_fpattern_match(findData->path, entryPath)) {
            break;
        }
        findData->entry = readdir(findData->dir);
    }

    if (findData->entry == nullptr) {
        closedir(findData->dir);
        findData->dir = nullptr;
        return false;
    }
#endif

    return true;
}

// 0x4E63CC
bool findFindClose(DirectoryFileFindData* findData)
{
#if defined(_MSC_VER)
    FindClose(findData->hFind);
#else
    if (findData->dir != nullptr) {
        if (closedir(findData->dir) != 0) {
            return false;
        }
    }
#endif

    return true;
}

} // namespace fallout
