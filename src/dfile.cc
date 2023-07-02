#include "dfile.h"

#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include <fpattern.h>

// Migrated
#include "platform_compat.h"

extern "C" {
    int rust_dfile_close(fallout::DFile* stream);
    fallout::DFile* rust_dfile_open_internal(fallout::DBase* dbase, const char* filePath, const char* mode, fallout::DFile* dfile);
    bool rust_dfile_read_compressed(fallout::DFile* stream, void* ptr, size_t size);
    int rust_dfile_read_char_internal(fallout::DFile* stream);
    bool rust_dbase_close(fallout::DBase* dbase);
    fallout::DBase* rust_dbase_open_part(const char* filePath, bool* success, FILE** outStream, int* fileSize, int* dbaseDataSize,
        bool (*callback)(FILE*, fallout::DBaseEntry*)
        );
    // rust_dbase_open
}

namespace fallout {

// The size of decompression buffer for reading compressed [DFile]s.
#define DFILE_DECOMPRESSION_BUFFER_SIZE (0x400)

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
bool callback(FILE* stream, DBaseEntry* entry) {
    // Migrated until HERE !!!

    if (fread(&(entry->dataOffset), sizeof(entry->dataOffset), 1, stream) != 1) {
        return false;
    }

    return true;
}

DBase* dbaseOpen(const char* filePath)
{
    bool success = true;
    FILE* stream2 = nullptr;
    int fileSize2 = 0;
    int dbaseDataSize2 = 0;
    DBase* dbase = rust_dbase_open_part(filePath, &success, &stream2, &fileSize2, &dbaseDataSize2, callback);

    if (!success) {
        return nullptr;
    }

    FILE* stream = stream2;
    int fileSize = fileSize2;
    int dbaseDataSize = dbaseDataSize2;

    // Migrated until HERE !!!
//
//    if (entryIndex < dbase->entriesLength) {
//        // We haven't reached the end, which means there was an error while
//        // reading entries.
//        dbaseClose(dbase);
//        fclose(stream);
//        return nullptr;
//    }

    dbase->path = compat_strdup(filePath);
    dbase->dataOffset = fileSize - dbaseDataSize;

    fclose(stream);

    return dbase;
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
    for (int index = 0; index < dbase->entriesLength; index++) {
        DBaseEntry* entry = &(dbase->entries[index]);
        if (fpattern_match(pattern, entry->path)) {
            strcpy(findFileData->fileName, entry->path);
            strcpy(findFileData->pattern, pattern);
            findFileData->index = index;
            return true;
        }
    }

    return false;
}

// 0x4E53A0
bool dbaseFindNextEntry(DBase* dbase, DFileFindData* findFileData)
{
    for (int index = findFileData->index + 1; index < dbase->entriesLength; index++) {
        DBaseEntry* entry = &(dbase->entries[index]);
        if (fpattern_match(findFileData->pattern, entry->path)) {
            strcpy(findFileData->fileName, entry->path);
            findFileData->index = index;
            return true;
        }
    }

    return false;
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
    assert(dbase); // dfile.c, 295
    assert(filePath); // dfile.c, 296
    assert(mode); // dfile.c, 297

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
    assert(stream); // "stream", "dfile.c", 384

    if ((stream->flags & DFILE_EOF) != 0 || (stream->flags & DFILE_ERROR) != 0) {
        return -1;
    }

    if ((stream->flags & DFILE_HAS_UNGETC) != 0) {
        stream->flags &= ~DFILE_HAS_UNGETC;
        return stream->ungotten;
    }

    int ch = rust_dfile_read_char_internal(stream);
    if (ch == -1) {
        stream->flags |= DFILE_EOF;
    }

    return ch;
}

// [fgets].
//
// Both Windows (\r\n) and Unix (\n) line endings are recognized. Windows
// line ending is reported as \n.
//
// 0x4E5764
char* dfileReadString(char* string, int size, DFile* stream)
{
    assert(string); // "s", "dfile.c", 407
    assert(size); // "n", "dfile.c", 408
    assert(stream); // "stream", "dfile.c", 409

    if ((stream->flags & DFILE_EOF) != 0 || (stream->flags & DFILE_ERROR) != 0) {
        return NULL;
    }

    char* pch = string;

    if ((stream->flags & DFILE_HAS_UNGETC) != 0) {
        *pch++ = stream->ungotten & 0xFF;
        size--;
        stream->flags &= ~DFILE_HAS_UNGETC;
    }

    // Read up to size - 1 characters one by one saving space for the null
    // terminator.
    for (int index = 0; index < size - 1; index++) {
        int ch = rust_dfile_read_char_internal(stream);
        if (ch == -1) {
            break;
        }

        *pch++ = ch & 0xFF;

        if (ch == '\n') {
            break;
        }
    }

    if (pch == string) {
        // No character was set into the buffer.
        return nullptr;
    }

    *pch = '\0';

    return string;
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
    assert(ptr); // "ptr", "dfile.c", 499
    assert(stream); // "stream", dfile.c, 500

    if ((stream->flags & DFILE_EOF) != 0 || (stream->flags & DFILE_ERROR) != 0) {
        return 0;
    }

    size_t remainingSize = stream->entry->uncompressedSize - stream->position;
    if ((stream->flags & DFILE_HAS_UNGETC) != 0) {
        remainingSize++;
    }

    size_t bytesToRead = size * count;
    if (remainingSize < bytesToRead) {
        bytesToRead = remainingSize;
        stream->flags |= DFILE_EOF;
    }

    size_t extraBytesRead = 0;
    if ((stream->flags & DFILE_HAS_UNGETC) != 0) {
        unsigned char* byteBuffer = (unsigned char*)ptr;
        *byteBuffer++ = stream->ungotten & 0xFF;
        ptr = byteBuffer;

        bytesToRead--;

        stream->flags &= ~DFILE_HAS_UNGETC;
        extraBytesRead = 1;
    }

    size_t bytesRead;
    if (stream->entry->compressed == 1) {
        if (!rust_dfile_read_compressed(stream, ptr, bytesToRead)) {
            stream->flags |= DFILE_ERROR;
            return false;
        }

        bytesRead = bytesToRead;
    } else {
        bytesRead = fread(ptr, 1, bytesToRead, stream->stream) + extraBytesRead;
        stream->position += bytesRead;
    }

    return bytesRead / size;
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
