#include "xfile.h"

#ifdef _WIN32
#include <direct.h>
#endif

extern "C" {
    int rust_xfile_close(fallout::XFile* stream);
    fallout::XFile* rust_xfile_open(const char* filePath, const char* mode);
    int rust_xfile_print_formatted_args(fallout::XFile* stream, const char* format, va_list args);
    int rust_xfile_write_string(const char* string, fallout::XFile* stream);
    size_t rust_xfile_write(const void* ptr, size_t size, size_t count, fallout::XFile* stream);
    int rust_xfile_seek(fallout::XFile* stream, long offset, int origin);
    long rust_xfile_tell(fallout::XFile* stream);
    void rust_xfile_rewind(fallout::XFile* stream);
    int rust_xfile_eof(fallout::XFile* stream);
    long rust_xfile_get_size(fallout::XFile* stream);
    bool rust_xbase_reopen_all(char* paths);
    void rust_xlist_free(fallout::XList* xlist);
}

namespace fallout {

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

// 0x4DF380
int xfileWriteString(const char* string, XFile* stream)
{
    return rust_xfile_write_string(string, stream);
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

// 0x4DFF48
void xlistFree(XList* xlist)
{
    rust_xlist_free(xlist);
}

} // namespace fallout
