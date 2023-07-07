#include "dfile.h"

#include <cassert>

extern "C" {
    bool rust_dbase_close(fallout::DBase* dbase);
    fallout::DBase* rust_dbase_open_part(const char* filePath);
    bool rust_dbase_find_first_entry(fallout::DBase* dbase, fallout::DFileFindData* findFileData, const char* pattern);
    bool rust_dbase_find_next_entry(fallout::DBase* dbase, fallout::DFileFindData* findFileData);
    // rust_dfile_eof
}

namespace fallout {

// Specifies that [DFile] has reached end of stream.
#define DFILE_EOF (0x02)

// Reads .DAT file contents.
//
// 0x4E4F58
DBase* dbaseOpen(const char* filePath)
{
    return rust_dbase_open_part(filePath);
}

// Closes [dbase], all open file handles, frees all associated resources,
// including the [dbase] itself.
//
// 0x4E5270
bool dbaseClose(DBase* dbase)
{
    return rust_dbase_close(dbase);
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

// [filelength].
//
// 0x4E5424
long dfileGetSize(DFile* stream)
{
    return stream->entry->uncompressedSize;
}

} // namespace fallout
