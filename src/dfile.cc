#include "dfile.h"

#include <cassert>
#include <cstdio>

// TODO migrate

// Migrated
#include "platform_compat.h"

extern "C" {
    int rust_dfile_close(fallout::DFile* stream);
    fallout::DFile* rust_dfile_open_internal(fallout::DBase* dbase, const char* filePath, const char* mode, fallout::DFile* dfile);
    bool rust_dfile_read_compressed(fallout::DFile* stream, void* ptr, size_t size);
    int rust_dfile_read_char_internal(fallout::DFile* stream);
    bool rust_dbase_close(fallout::DBase* dbase);
    fallout::DBase* rust_dbase_open_part(const char* filePath);
    bool rust_fpattern_match(const char *pat, const char *fname);
    bool rust_dbase_find_first_entry(fallout::DBase* dbase, fallout::DFileFindData* findFileData, const char* pattern);
    bool rust_dbase_find_next_entry(fallout::DBase* dbase, fallout::DFileFindData* findFileData);
    int rust_dfile_read_char(fallout::DFile* stream);
    char* rust_dfile_read_string(char* string, int size, fallout::DFile* stream);
    size_t rust_dfile_read(void* ptr, size_t size, size_t count, fallout::DFile* stream);
    // rust_dfile_read
}

namespace fallout {

// Specifies that [DFile] has unget character.
//
// NOTE: There is an unused function at 0x4E5894 which ungets one character and
// stores it in [ungotten]. Since that function is not used, this flag will
// never be set.
#define DFILE_HAS_UNGETC (0x01)

// Specifies that [DFile] has reached end of stream.
#define DFILE_EOF (0x02)

// Specifies that [DFile] is in error state.
//
// [dfileRewind] can be used to clear this flag.
#define DFILE_ERROR (0x04)

// Specifies that [DFile] was opened in text mode.
#define DFILE_TEXT (0x08)

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

// [fclose].
//
// 0x4E542C
int dfileClose(DFile* stream)
{
    return rust_dfile_close(stream);
}

// [fopen].
//
// 0x4E5504
DFile* dfileOpen(DBase* dbase, const char* filePath, const char* mode)
{
    return rust_dfile_open_internal(dbase, filePath, mode, 0);
}

// [vfprintf].
//
// 0x4E56C0
int dfilePrintFormattedArgs(DFile* stream, const char* format, va_list args)
{
    assert(stream); // "stream", "dfile.c", 368
    assert(format); // "format", "dfile.c", 369

    return -1;
}

// [fgetc].
//
// This function reports \r\n sequence as one character \n, even though it
// consumes two characters from the underlying stream.
//
// 0x4E5700
int dfileReadChar(DFile* stream)
{
    return rust_dfile_read_char(stream);
}

// [fgets].
//
// Both Windows (\r\n) and Unix (\n) line endings are recognized. Windows
// line ending is reported as \n.
//
// 0x4E5764
char* dfileReadString(char* string, int size, DFile* stream)
{
    return rust_dfile_read_string(string, size, stream);
}

// [fputc].
//
// 0x4E5830
int dfileWriteChar(int ch, DFile* stream)
{
    assert(stream); // "stream", "dfile.c", 437

    return -1;
}

// [fputs].
//
// 0x4E5854
int dfileWriteString(const char* string, DFile* stream)
{
    assert(string); // "s", "dfile.c", 448
    assert(stream); // "stream", "dfile.c", 449

    return -1;
}

// [fread].
//
// 0x4E58FC
size_t dfileRead(void* ptr, size_t size, size_t count, DFile* stream)
{
    return rust_dfile_read(ptr, size, count, stream);
}

// [fwrite].
//
// 0x4E59F8
size_t dfileWrite(const void* ptr, size_t size, size_t count, DFile* stream)
{
    assert(ptr); // "ptr", "dfile.c", 538
    assert(stream); // "stream", "dfile.c", 539

    return count - 1;
}

// [fseek].
//
// 0x4E5A74
int dfileSeek(DFile* stream, long offset, int origin)
{
    assert(stream); // "stream", "dfile.c", 569

    if ((stream->flags & DFILE_ERROR) != 0) {
        return 1;
    }

    if ((stream->flags & DFILE_TEXT) != 0) {
        if (offset != 0 && origin != SEEK_SET) {
            // NOTE: For unknown reason this function does not allow arbitrary
            // seeks in text streams, whether compressed or not. It only
            // supports rewinding. Probably because of reading functions which
            // handle \r\n sequence as \n.
            return 1;
        }
    }

    long offsetFromBeginning;
    switch (origin) {
    case SEEK_SET:
        offsetFromBeginning = offset;
        break;
    case SEEK_CUR:
        offsetFromBeginning = stream->position + offset;
        break;
    case SEEK_END:
        offsetFromBeginning = stream->entry->uncompressedSize + offset;
        break;
    default:
        return 1;
    }

    if (offsetFromBeginning >= stream->entry->uncompressedSize) {
        return 1;
    }

    long pos = stream->position;
    if (offsetFromBeginning == pos) {
        stream->flags &= ~(DFILE_HAS_UNGETC | DFILE_EOF);
        return 0;
    }

    if (offsetFromBeginning != 0) {
        if (stream->entry->compressed == 1) {
            if (offsetFromBeginning < pos) {
                // We cannot go backwards in compressed stream, so the only way
                // is to start from the beginning.
                dfileRewind(stream);
            }

            // Consume characters one by one until we reach specified offset.
            while (offsetFromBeginning > stream->position) {
                if (rust_dfile_read_char_internal(stream) == -1) {
                    return 1;
                }
            }
        } else {
            if (fseek(stream->stream, offsetFromBeginning - pos, SEEK_CUR) != 0) {
                stream->flags |= DFILE_ERROR;
                return 1;
            }

            // FIXME: I'm not sure what this assignment means. This field is
            // only meaningful when reading compressed streams.
            stream->compressedBytesRead = offsetFromBeginning;
        }

        stream->flags &= ~(DFILE_HAS_UNGETC | DFILE_EOF);
        return 0;
    }

    if (fseek(stream->stream, stream->dbase->dataOffset + stream->entry->dataOffset, SEEK_SET) != 0) {
        stream->flags |= DFILE_ERROR;
        return 1;
    }

    if (inflateEnd(stream->decompressionStream) != Z_OK) {
        stream->flags |= DFILE_ERROR;
        return 1;
    }

    stream->decompressionStream->zalloc = Z_NULL;
    stream->decompressionStream->zfree = Z_NULL;
    stream->decompressionStream->opaque = Z_NULL;
    stream->decompressionStream->next_in = stream->decompressionBuffer;
    stream->decompressionStream->avail_in = 0;

    if (inflateInit(stream->decompressionStream) != Z_OK) {
        stream->flags |= DFILE_ERROR;
        return 1;
    }

    stream->position = 0;
    stream->compressedBytesRead = 0;
    stream->flags &= ~(DFILE_HAS_UNGETC | DFILE_EOF);

    return 0;
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
    assert(stream); // "stream", "dfile.c", 664

    dfileSeek(stream, 0, SEEK_SET);

    stream->flags &= ~DFILE_ERROR;
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
