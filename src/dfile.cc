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
    // rust_dfile_unget_compressed
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

static int dfileReadCharInternal(DFile* stream);

// Reads .DAT file contents.
//
// 0x4E4F58
DBase* dbaseOpen(const char* filePath)
{
    assert(filePath); // "filename", "dfile.c", 74

    FILE* stream = compat_fopen(filePath, "rb");
    if (stream == nullptr) {
        return nullptr;
    }

    DBase* dbase = (DBase*)malloc(sizeof(*dbase));
    if (dbase == nullptr) {
        fclose(stream);
        return nullptr;
    }

    memset(dbase, 0, sizeof(*dbase));

    // Get file size, and reposition stream to read footer, which contains two
    // 32-bits ints.
    int fileSize = getFileSize(stream);
    if (fseek(stream, fileSize - sizeof(int) * 2, SEEK_SET) != 0) {
        goto err;
    }

    // Read the size of entries table.
    int entriesDataSize;
    if (fread(&entriesDataSize, sizeof(entriesDataSize), 1, stream) != 1) {
        goto err;
    }

    // Read the size of entire dbase content.
    //
    // NOTE: It appears that this approach allows existence of arbitrary data in
    // the beginning of the .DAT file.
    int dbaseDataSize;
    if (fread(&dbaseDataSize, sizeof(dbaseDataSize), 1, stream) != 1) {
        goto err;
    }

    // Reposition stream to the beginning of the entries table.
    if (fseek(stream, fileSize - entriesDataSize - sizeof(int) * 2, SEEK_SET) != 0) {
        goto err;
    }

    if (fread(&(dbase->entriesLength), sizeof(dbase->entriesLength), 1, stream) != 1) {
        goto err;
    }

    dbase->entries = (DBaseEntry*)malloc(sizeof(*dbase->entries) * dbase->entriesLength);
    if (dbase->entries == nullptr) {
        goto err;
    }

    memset(dbase->entries, 0, sizeof(*dbase->entries) * dbase->entriesLength);

    // Read entries one by one, stopping on any error.
    int entryIndex;
    for (entryIndex = 0; entryIndex < dbase->entriesLength; entryIndex++) {
        DBaseEntry* entry = &(dbase->entries[entryIndex]);

        int pathLength;
        if (fread(&pathLength, sizeof(pathLength), 1, stream) != 1) {
            break;
        }

        entry->path = (char*)malloc(pathLength + 1);
        if (entry->path == nullptr) {
            break;
        }

        if (fread(entry->path, pathLength, 1, stream) != 1) {
            break;
        }

        entry->path[pathLength] = '\0';

        if (fread(&(entry->compressed), sizeof(entry->compressed), 1, stream) != 1) {
            break;
        }

        if (fread(&(entry->uncompressedSize), sizeof(entry->uncompressedSize), 1, stream) != 1) {
            break;
        }

        if (fread(&(entry->dataSize), sizeof(entry->dataSize), 1, stream) != 1) {
            break;
        }

        if (fread(&(entry->dataOffset), sizeof(entry->dataOffset), 1, stream) != 1) {
            break;
        }
    }

    if (entryIndex < dbase->entriesLength) {
        // We haven't reached the end, which means there was an error while
        // reading entries.
        goto err;
    }

    dbase->path = compat_strdup(filePath);
    dbase->dataOffset = fileSize - dbaseDataSize;

    fclose(stream);

    return dbase;

err:

    dbaseClose(dbase);

    fclose(stream);

    return nullptr;
}

// Closes [dbase], all open file handles, frees all associated resources,
// including the [dbase] itself.
//
// 0x4E5270
bool dbaseClose(DBase* dbase)
{
    assert(dbase); // "dbase", "dfile.c", 173

    DFile* curr = dbase->dfileHead;
    while (curr != nullptr) {
        DFile* next = curr->next;
        rust_dfile_close(curr);
        curr = next;
    }

    if (dbase->entries != nullptr) {
        for (int index = 0; index < dbase->entriesLength; index++) {
            DBaseEntry* entry = &(dbase->entries[index]);
            char* entryName = entry->path;
            if (entryName != nullptr) {
                free(entryName);
            }
        }
        free(dbase->entries);
    }

    if (dbase->path != nullptr) {
        free(dbase->path);
    }

    memset(dbase, 0, sizeof(*dbase));

    free(dbase);

    return true;
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

    int ch = dfileReadCharInternal(stream);
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
        int ch = dfileReadCharInternal(stream);
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
                if (dfileReadCharInternal(stream) == -1) {
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

// 0x4E5F9C
static int dfileReadCharInternal(DFile* stream)
{
    return rust_dfile_read_char_internal(stream);
}

} // namespace fallout
