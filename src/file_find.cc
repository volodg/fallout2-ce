#include "file_find.h"

extern "C" {
    bool rust_file_find_first(const char* path, fallout::DirectoryFileFindData* findData);
    bool rust_file_find_next(fallout::DirectoryFileFindData* findData);
    bool rust_file_find_close(fallout::DirectoryFileFindData* findData);
    // rust_file_find_close
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
    return rust_file_find_next(findData);
}

// 0x4E63CC
bool findFindClose(DirectoryFileFindData* findData)
{
    return rust_file_find_close(findData);
}

} // namespace fallout
