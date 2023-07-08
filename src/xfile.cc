#include "xfile.h"

#include <cassert>
#include <cstdlib>
#include <cstring>

#ifdef _WIN32
#include <direct.h>
#endif

// TODO Migrate

namespace fallout {
    struct XListEnumerationContext;
    typedef bool(XListEnumerationHandler)(XListEnumerationContext* context);
}

extern "C" {
    int rust_xfile_close(fallout::XFile* stream);
    fallout::XFile* rust_xfile_open(const char* filePath, const char* mode);
    int rust_xfile_print_formatted_args(fallout::XFile* stream, const char* format, va_list args);
    int rust_xfile_read_char(fallout::XFile* stream);
    char* rust_xfile_read_string(char* string, int size, fallout::XFile* stream);
    int rust_xfile_write_char(int ch, fallout::XFile* stream);
    int rust_xfile_write_string(const char* string, fallout::XFile* stream);
    size_t rust_xfile_read(void* ptr, size_t size, size_t count, fallout::XFile* stream);
    size_t rust_xfile_write(const void* ptr, size_t size, size_t count, fallout::XFile* stream);
    int rust_xfile_seek(fallout::XFile* stream, long offset, int origin);
    long rust_xfile_tell(fallout::XFile* stream);
    void rust_xfile_rewind(fallout::XFile* stream);
    int rust_xfile_eof(fallout::XFile* stream);
    long rust_xfile_get_size(fallout::XFile* stream);
    bool rust_xbase_open(const char* path);
    bool rust_xbase_reopen_all(char* paths);
    bool rust_xlist_enumerate(const char* pattern, fallout::XListEnumerationHandler* handler, fallout::XList* xlist);
    // rust_xlist_enumerate()
}

namespace fallout {

typedef enum XFileEnumerationEntryType {
    XFILE_ENUMERATION_ENTRY_TYPE_FILE,
    XFILE_ENUMERATION_ENTRY_TYPE_DIRECTORY,
    XFILE_ENUMERATION_ENTRY_TYPE_DFILE,
} XFileEnumerationEntryType;

typedef struct XListEnumerationContext {
    char name[COMPAT_MAX_PATH];
    unsigned char type;
    XList* xlist;
} XListEnumerationContext;

static bool xlistEnumerate(const char* pattern, XListEnumerationHandler* handler, XList* xlist);
static bool xlistEnumerateHandler(XListEnumerationContext* context);

// 0x4DED6C
int xfileClose(XFile* stream)
{
    return rust_xfile_close(stream);
}

// 0x4DEE2C
XFile* xfileOpen(const char* filePath, const char* mode)
{
    return rust_xfile_open(filePath, mode);
}

// [vfprintf].
//
// 0x4DF1AC
int xfilePrintFormattedArgs(XFile* stream, const char* format, va_list args)
{
    return rust_xfile_print_formatted_args(stream, format, args);
}

// 0x4DF22C
int xfileReadChar(XFile* stream)
{
    return rust_xfile_read_char(stream);
}

// 0x4DF280
char* xfileReadString(char* string, int size, XFile* stream)
{
    return rust_xfile_read_string(string, size, stream);
}

// 0x4DF320
int xfileWriteChar(int ch, XFile* stream)
{
    return rust_xfile_write_char(ch, stream);
}

// 0x4DF380
int xfileWriteString(const char* string, XFile* stream)
{
    return rust_xfile_write_string(string, stream);
}

// 0x4DF44C
size_t xfileRead(void* ptr, size_t size, size_t count, XFile* stream)
{
    return rust_xfile_read(ptr, size, count, stream);
}

// 0x4DF4E8
size_t xfileWrite(const void* ptr, size_t size, size_t count, XFile* stream)
{
    return rust_xfile_write(ptr, size, count, stream);
}

// 0x4DF5D8
int xfileSeek(XFile* stream, long offset, int origin)
{
    return rust_xfile_seek(stream, offset, origin);
}

// 0x4DF690
long xfileTell(XFile* stream)
{
    return rust_xfile_tell(stream);
}

// 0x4DF6E4
void xfileRewind(XFile* stream)
{
    rust_xfile_rewind(stream);
}

// 0x4DF780
int xfileEof(XFile* stream)
{
    return rust_xfile_eof(stream);
}

// 0x4DF828
long xfileGetSize(XFile* stream)
{
    return rust_xfile_get_size(stream);
}

// Closes all open xbases and opens a set of xbases specified by [paths].
//
// [paths] is a set of paths separated by semicolon. Can be NULL, in this case
// all open xbases are simply closed.
//
// 0x4DF878
bool xbaseReopenAll(char* paths)
{
    return rust_xbase_reopen_all(paths);
}

// 0x4DF938
bool xbaseOpen(const char* path)
{
    return rust_xbase_open(path);
}

// 0x4DFB3C
static bool xlistEnumerate(const char* pattern, XListEnumerationHandler* handler, XList* xlist)
{
    return rust_xlist_enumerate(pattern, handler, xlist);
}

// 0x4DFF28
bool xlistInit(const char* pattern, XList* xlist)
{
    xlistEnumerate(pattern, xlistEnumerateHandler, xlist);
    return xlist->fileNamesLength != -1;
}

// 0x4DFF48
void xlistFree(XList* xlist)
{
    assert(xlist); // "list", "xfile.c", 949

    for (int index = 0; index < xlist->fileNamesLength; index++) {
        if (xlist->fileNames[index] != nullptr) {
            free(xlist->fileNames[index]);
        }
    }

    free(xlist->fileNames);

    memset(xlist, 0, sizeof(*xlist));
}

// 0x4E0278
static bool xlistEnumerateHandler(XListEnumerationContext* context)
{
    if (context->type == XFILE_ENUMERATION_ENTRY_TYPE_DIRECTORY) {
        return true;
    }

    XList* xlist = context->xlist;

    char** fileNames = (char**)realloc(xlist->fileNames, sizeof(*fileNames) * (xlist->fileNamesLength + 1));
    if (fileNames == nullptr) {
        xlistFree(xlist);
        xlist->fileNamesLength = -1;
        return false;
    }

    xlist->fileNames = fileNames;

    fileNames[xlist->fileNamesLength] = compat_strdup(context->name);
    if (fileNames[xlist->fileNamesLength] == nullptr) {
        xlistFree(xlist);
        xlist->fileNamesLength = -1;
        return false;
    }

    xlist->fileNamesLength++;

    return true;
}

} // namespace fallout
