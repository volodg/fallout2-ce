#ifndef XFILE_H
#define XFILE_H

#include <cstdio>
#include <zlib.h>

namespace fallout {

struct XFile;

typedef struct XList {
    int _fileNamesLength;
    char** _fileNames;
} XList;

int xfileClose(XFile* stream);
XFile* xfileOpen(const char* filename, const char* mode);
int xfilePrintFormattedArgs(XFile* stream, const char* format, va_list args);
int xfileWriteString(const char* s, XFile* stream);
size_t xfileWrite(const void* buf, size_t size, size_t count, XFile* stream);
int xfileSeek(XFile* stream, long offset, int origin);
long xfileTell(XFile* stream);
void xfileRewind(XFile* stream);
int xfileEof(XFile* stream);
long xfileGetSize(XFile* stream);
bool xbaseReopenAll(char* paths);

} // namespace fallout

#endif /* XFILE_H */
