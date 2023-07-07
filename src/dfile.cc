#include "dfile.h"

#include <cassert>

extern "C" {
    bool rust_dbase_close(fallout::DBase* dbase);
    fallout::DBase* rust_dbase_open_part(const char* filePath);
    bool rust_dbase_find_first_entry(fallout::DBase* dbase, fallout::DFileFindData* findFileData, const char* pattern);
    bool rust_dbase_find_next_entry(fallout::DBase* dbase, fallout::DFileFindData* findFileData);
    int rust_dfile_seek(fallout::DFile* stream, long offset, int origin);
    void rust_dfile_rewind(fallout::DFile* stream);
    size_t rust_dfile_write(const void* ptr, size_t size, size_t count, fallout::DFile* stream);
    // rust_dfile_write
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

// [fwrite].
//
// 0x4E59F8
size_t dfileWrite(const void* ptr, size_t size, size_t count, DFile* stream)
{
    return rust_dfile_write(ptr, size, count, stream);
}

// [fseek].
//
// 0x4E5A74
int dfileSeek(DFile* stream, long offset, int origin)
{
    return rust_dfile_seek(stream, offset, origin);
}

// [ftell].
//
// 0x4E5C88
long dfileTell(DFile* stream)
{
    assert(stream); // "stream", "dfile.c", 654

    return stream->position;
}

// [rewind].
//
// 0x4E5CB0
void dfileRewind(DFile* stream)
{
    rust_dfile_rewind(stream);
}

// [feof].
//
// 0x4E5D10
int dfileEof(DFile* stream)
{
    assert(stream); // "stream", "dfile.c", 685

    return stream->flags & DFILE_EOF;
}

} // namespace fallout
