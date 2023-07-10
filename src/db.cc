#include "db.h"

#include <cassert>
#include <cstdlib>
// #include <cstring>

// TODO Migrate

// Migrated
// #include "platform_compat.h"
#include "xfile.h"

namespace fallout {
    struct FileList;
}

extern "C" {
    int rust_db_open(const char* filePath1, int a2, const char* filePath2, int a4);
    int rust_db_get_file_size(const char* filePath, int* sizePtr);
    void rust_set_g_file_read_progress_handler(fallout::FileReadProgressHandler*);
    void rust_set_g_file_read_progress_chunk_size(int);
    fallout::FileList* rust_g_get_file_list_head();
    void rust_g_set_file_list_head(fallout::FileList*);
    int rust_db_get_file_contents(const char* filePath, void* ptr);
    int rust_file_read_char(fallout::File* stream);
    char* rust_file_read_string(char* string, size_t size, fallout::File* stream);
    size_t rust_file_read(void* ptr, size_t size, size_t count, fallout::File* stream);
    int rust_file_read_uint8(fallout::File* stream, unsigned char* valuePtr);
    int rust_file_read_int16(fallout::File* stream, short* valuePtr);
    int rust_file_read_int32(fallout::File* stream, int* valuePtr);
    int rust_file_read_bool(fallout::File* stream, bool* valuePtr);
    int rust_file_write_uint8(fallout::File* stream, unsigned char value);
    int rust_file_write_int16(fallout::File* stream, short value);
    int rust_db_fwrite_long(fallout::File* stream, int value);
    int rust_file_read_uint8_list(fallout::File* stream, unsigned char* arr, int count);
    int rust_file_read_int16_list(fallout::File* stream, short* arr, int count);
    int rust_file_read_int32_list(fallout::File* stream, int* arr, int count);
    int rust_file_write_uint8_list(fallout::File* stream, unsigned char* arr, int count);
    int rust_file_write_int16_list(fallout::File* stream, short* arr, int count);
    int rust_file_write_int32_list(fallout::File* stream, int* arr, int count);
    int rust_db_fwrite_long_count(fallout::File* stream, int* arr, int count);
    int rust_db_list_compare(const void* p1, const void* p2);
    int rust_file_name_list_init(const char* pattern, char*** fileNameListPtr, int a3, int a4);
    // rust_file_read_uint8
}

namespace fallout {

typedef struct FileList {
    XList xlist;
    struct FileList* next;
} FileList;

// Opens file database.
//
// Returns -1 if [filePath1] was specified, but could not be opened by the
// underlying xbase implementation. Result of opening [filePath2] is ignored.
// Returns 0 on success.
//
// NOTE: There are two unknown parameters passed via edx and ecx. The [a2] is
// always 0 at the calling sites, and [a4] is always 1. Both parameters are not
// used, so it's impossible to figure out their meaning.
//
// 0x4C5D30
int dbOpen(const char* filePath1, int a2, const char* filePath2, int a4)
{
    return rust_db_open(filePath1, a2, filePath2, a4);
}

// 0x4C5D58
int _db_total()
{
    return 0;
}

// 0x4C5D60
void dbExit()
{
    xbaseReopenAll(nullptr);
}

// TODO: sizePtr should be long*.
//
// 0x4C5D68
int dbGetFileSize(const char* filePath, int* sizePtr)
{
    return rust_db_get_file_size(filePath, sizePtr);
}

// 0x4C5DD4
int dbGetFileContents(const char* filePath, void* ptr)
{
    return rust_db_get_file_contents(filePath, ptr);
}

// 0x4C5EB4
int fileClose(File* stream)
{
    return xfileClose(stream);
}

// 0x4C5EC8
File* fileOpen(const char* filename, const char* mode)
{
    return xfileOpen(filename, mode);
}

// 0x4C5ED0
int filePrintFormatted(File* stream, const char* format, ...)
{
    assert(format); // "format", "db.c", 224

    va_list args;
    va_start(args, format);

    int rc = xfilePrintFormattedArgs(stream, format, args);

    va_end(args);

    return rc;
}

// 0x4C5F24
int fileReadChar(File* stream)
{
    return rust_file_read_char(stream);
}

// 0x4C5F70
char* fileReadString(char* string, size_t size, File* stream)
{
    return rust_file_read_string(string, size, stream);
}

// 0x4C5FEC
int fileWriteString(const char* string, File* stream)
{
    return xfileWriteString(string, stream);
}

// 0x4C5FFC
size_t fileRead(void* ptr, size_t size, size_t count, File* stream)
{
    return rust_file_read(ptr, size, count, stream);
}

// 0x4C60B8
size_t fileWrite(const void* buf, size_t size, size_t count, File* stream)
{
    return xfileWrite(buf, size, count, stream);
}

// 0x4C60C0
int fileSeek(File* stream, long offset, int origin)
{
    return xfileSeek(stream, offset, origin);
}

// 0x4C60C8
long fileTell(File* stream)
{
    return xfileTell(stream);
}

// 0x4C60D0
void fileRewind(File* stream)
{
    xfileRewind(stream);
}

// 0x4C60D8
int fileEof(File* stream)
{
    return xfileEof(stream);
}

// NOTE: Not sure about signness.
//
// 0x4C60E0
int fileReadUInt8(File* stream, unsigned char* valuePtr)
{
    return rust_file_read_uint8(stream, valuePtr);
}

// NOTE: Not sure about signness.
//
// 0x4C60F4
int fileReadInt16(File* stream, short* valuePtr)
{
    return rust_file_read_int16(stream, valuePtr);
}

// 0x4C614C
int fileReadInt32(File* stream, int* valuePtr)
{
    return rust_file_read_int32(stream, valuePtr);
}

// NOTE: Uncollapsed 0x4C614C. The opposite of [_db_fwriteLong]. It can be either
// signed vs. unsigned variant, as well as int vs. long. It's provided here to
// identify places where data was written with [_db_fwriteLong].
int _db_freadInt(File* stream, int* valuePtr)
{
    return fileReadInt32(stream, valuePtr);
}

// NOTE: Probably uncollapsed 0x4C614C.
int fileReadUInt32(File* stream, unsigned int* valuePtr)
{
    return _db_freadInt(stream, (int*)valuePtr);
}

// NOTE: Uncollapsed 0x4C614C. The opposite of [fileWriteFloat].
int fileReadFloat(File* stream, float* valuePtr)
{
    return fileReadInt32(stream, (int*)valuePtr);
}

// rust_file_read_bool
int fileReadBool(File* stream, bool* valuePtr)
{
    return rust_file_read_bool(stream, valuePtr);
}

// NOTE: Not sure about signness.
//
// 0x4C61AC
int fileWriteUInt8(File* stream, unsigned char value)
{
    return rust_file_write_uint8(stream, value);
};

// 0x4C61C8
int fileWriteInt16(File* stream, short value)
{
    return rust_file_write_int16(stream, value);
}

// NOTE: Not sure about signness and int vs. long.
//
// 0x4C6214
int fileWriteInt32(File* stream, int value)
{
    // NOTE: Uninline.
    return _db_fwriteLong(stream, value);
}

// NOTE: Can either be signed vs. unsigned variant of [fileWriteInt32],
// or int vs. long.
//
// 0x4C6244
int _db_fwriteLong(File* stream, int value)
{
    return rust_db_fwrite_long(stream, value);
}

// NOTE: Probably uncollapsed 0x4C6214 or 0x4C6244.
int fileWriteUInt32(File* stream, unsigned int value)
{
    return _db_fwriteLong(stream, (int)value);
}

// 0x4C62C4
int fileWriteFloat(File* stream, float value)
{
    // NOTE: Uninline.
    return _db_fwriteLong(stream, *(int*)&value);
}

int fileWriteBool(File* stream, bool value)
{
    return _db_fwriteLong(stream, value ? 1 : 0);
}

// 0x4C62FC
int fileReadUInt8List(File* stream, unsigned char* arr, int count)
{
    return rust_file_read_uint8_list(stream, arr, count);
}

// NOTE: Probably uncollapsed 0x4C62FC. There are couple of places where
// [fileReadUInt8List] is used to read strings of fixed length. I'm not
// pretty sure this function existed in the original code, but at least
// it increases visibility of these places.
int fileReadFixedLengthString(File* stream, char* string, int length)
{
    return fileReadUInt8List(stream, (unsigned char*)string, length);
}

// 0x4C6330
int fileReadInt16List(File* stream, short* arr, int count)
{
    return rust_file_read_int16_list(stream, arr, count);
}

// NOTE: Not sure about signed/unsigned int/long.
//
// 0x4C63BC
int fileReadInt32List(File* stream, int* arr, int count)
{
    return rust_file_read_int32_list(stream, arr, count);
}

// NOTE: Uncollapsed 0x4C63BC. The opposite of [_db_fwriteLongCount].
int _db_freadIntCount(File* stream, int* arr, int count)
{
    return fileReadInt32List(stream, arr, count);
}

// 0x4C6464
int fileWriteUInt8List(File* stream, unsigned char* arr, int count)
{
    return rust_file_write_uint8_list(stream, arr, count);
}

// NOTE: Probably uncollapsed 0x4C6464. See [fileReadFixedLengthString].
int fileWriteFixedLengthString(File* stream, char* string, int length)
{
    return fileWriteUInt8List(stream, (unsigned char*)string, length);
}

// 0x4C6490
int fileWriteInt16List(File* stream, short* arr, int count)
{
    return rust_file_write_int16_list(stream, arr, count);
}

// NOTE: Can be either signed/unsigned + int/long variant.
//
// 0x4C64F8
int fileWriteInt32List(File* stream, int* arr, int count)
{
    return rust_file_write_int32_list(stream, arr, count);
}

// NOTE: Not sure about signed/unsigned int/long.
//
// 0x4C6550
int _db_fwriteLongCount(File* stream, int* arr, int count)
{
    return rust_db_fwrite_long_count(stream, arr, count);
}

// 0x4C6628
int fileNameListInit(const char* pattern, char*** fileNameListPtr, int a3, int a4)
{
    return rust_file_name_list_init(pattern, fileNameListPtr, a3, a4);
}

// 0x4C6868
// ???
void fileNameListFree(char*** fileNameListPtr, int a2)
{
    if (rust_g_get_file_list_head() == nullptr) {
        return;
    }

    FileList* currentFileList = rust_g_get_file_list_head();
    FileList* previousFileList = rust_g_get_file_list_head();
    while (*fileNameListPtr != currentFileList->xlist.fileNames) {
        previousFileList = currentFileList;
        currentFileList = currentFileList->next;
        if (currentFileList == nullptr) {
            return;
        }
    }

    if (previousFileList == rust_g_get_file_list_head()) {
        rust_g_set_file_list_head(currentFileList->next);
    } else {
        previousFileList->next = currentFileList->next;
    }

    xlistFree(&(currentFileList->xlist));

    free(currentFileList);
}

// TODO: Return type should be long.
//
// 0x4C68BC
int fileGetSize(File* stream)
{
    return xfileGetSize(stream);
}

// 0x4C68C4
void fileSetReadProgressHandler(FileReadProgressHandler* handler, int size)
{
    if (handler != nullptr && size != 0) {
        rust_set_g_file_read_progress_handler(handler);
        rust_set_g_file_read_progress_chunk_size(size);
    } else {
        rust_set_g_file_read_progress_handler(nullptr);
        rust_set_g_file_read_progress_chunk_size(0);
    }
}

} // namespace fallout
