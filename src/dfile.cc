#include "dfile.h"

extern "C" {
    fallout::DBase* rust_dbase_open(const char* filePath);
    bool rust_dbase_find_first_entry(fallout::DBase* dbase, fallout::DFileFindData* findFileData, const char* pattern);
    bool rust_dbase_find_next_entry(fallout::DBase* dbase, fallout::DFileFindData* findFileData);
}

namespace fallout {

// Specifies that [DFile] has reached end of stream.
#define DFILE_EOF (0x02)

// Reads .DAT file contents.
//
// 0x4E4F58
DBase* dbaseOpen(const char* filePath)
{
    return rust_dbase_open(filePath);
}

// 0x4E5308
bool dbaseFindFirstEntry(DBase* dbase, DFileFindData* findFileData, const char* pattern)
{
    return rust_dbase_find_first_entry(dbase, findFileData, pattern);
}

// 0x4E53A0
bool dbaseFindNextEntry(DBase* dbase, DFileFindData* findFileData)
{
    return rust_dbase_find_next_entry(dbase, findFileData);
}

// 0x4E541C
bool dbaseFindClose(DBase* dbase, DFileFindData* findFileData)
{
    return true;
}

} // namespace fallout
