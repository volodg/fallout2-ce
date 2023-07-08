#include "xfile.h"

#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#ifdef _WIN32
#include <direct.h>
#else
#include <unistd.h>
#endif

// TODO Migrate

// Migrated
#include "file_find.h"

extern "C" {
    int rust_xfile_close(fallout::XFile* stream);
    fallout::XBase* rust_get_g_xbase_head();
    void rust_set_g_xbase_head(fallout::XBase* base);
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
    void rust_xbase_close_all();
    int rust_xbase_make_directory(const char* filePath);
    // rust_xbase_make_directory()
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

typedef bool(XListEnumerationHandler)(XListEnumerationContext* context);

static bool xlistEnumerate(const char* pattern, XListEnumerationHandler* handler, XList* xlist);
static void xbaseExitHandler(void);
static bool xlistEnumerateHandler(XListEnumerationContext* context);

// 0x6B24D4
static bool gXbaseExitHandlerRegistered;

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
    // NOTE: Uninline.
    rust_xbase_close_all();

    if (paths != nullptr) {
        char* tok = strtok(paths, ";");
        while (tok != nullptr) {
            if (!xbaseOpen(tok)) {
                return false;
            }
            tok = strtok(nullptr, ";");
        }
    }

    return true;
}

// 0x4DF938
bool xbaseOpen(const char* path)
{
    assert(path); // "path", "xfile.c", 747

    // Register atexit handler so that underlying dbase (if any) can be
    // gracefully closed.
    if (!gXbaseExitHandlerRegistered) {
        atexit(xbaseExitHandler);
        gXbaseExitHandlerRegistered = true;
    }

    XBase* curr = rust_get_g_xbase_head();
    XBase* prev = nullptr;
    while (curr != nullptr) {
        if (compat_stricmp(path, curr->path) == 0) {
            break;
        }

        prev = curr;
        curr = curr->next;
    }

    if (curr != nullptr) {
        if (prev != nullptr) {
            // Move found xbase to the top.
            prev->next = curr->next;
            curr->next = rust_get_g_xbase_head();
            rust_set_g_xbase_head(curr);
        }
        return true;
    }

    XBase* xbase = (XBase*)malloc(sizeof(*xbase));
    if (xbase == nullptr) {
        return false;
    }

    memset(xbase, 0, sizeof(*xbase));

    xbase->path = compat_strdup(path);
    if (xbase->path == nullptr) {
        free(xbase);
        return false;
    }

    DBase* dbase = dbaseOpen(path);
    if (dbase != nullptr) {
        xbase->isDbase = true;
        xbase->dbase = dbase;
        xbase->next = rust_get_g_xbase_head();
        rust_set_g_xbase_head(xbase);
        return true;
    }

    char workingDirectory[COMPAT_MAX_PATH];
    if (getcwd(workingDirectory, COMPAT_MAX_PATH) == nullptr) {
        // FIXME: Leaking xbase and path.
        return false;
    }

    if (chdir(path) == 0) {
        chdir(workingDirectory);
        xbase->next = rust_get_g_xbase_head();
        rust_set_g_xbase_head(xbase);
        return true;
    }

    if (rust_xbase_make_directory(path) != 0) {
        // FIXME: Leaking xbase and path.
        return false;
    }

    chdir(workingDirectory);

    xbase->next = rust_get_g_xbase_head();
    rust_set_g_xbase_head(xbase);

    return true;
}

// 0x4DFB3C
static bool xlistEnumerate(const char* pattern, XListEnumerationHandler* handler, XList* xlist)
{
    assert(pattern); // "filespec", "xfile.c", 845
    assert(handler); // "enumfunc", "xfile.c", 846

    DirectoryFileFindData directoryFileFindData;
    XListEnumerationContext context;

    context.xlist = xlist;

    char nativePattern[COMPAT_MAX_PATH];
    strcpy(nativePattern, pattern);
    compat_windows_path_to_native(nativePattern);

    char drive[COMPAT_MAX_DRIVE];
    char dir[COMPAT_MAX_DIR];
    char fileName[COMPAT_MAX_FNAME];
    char extension[COMPAT_MAX_EXT];
    compat_splitpath(nativePattern, drive, dir, fileName, extension);
    if (drive[0] != '\0' || dir[0] == '\\' || dir[0] == '/' || dir[0] == '.') {
        if (fileFindFirst(nativePattern, &directoryFileFindData)) {
            do {
                bool isDirectory = fileFindIsDirectory(&directoryFileFindData);
                char* entryName = fileFindGetName(&directoryFileFindData);

                if (isDirectory) {
                    if (strcmp(entryName, "..") == 0 || strcmp(entryName, ".") == 0) {
                        continue;
                    }

                    context.type = XFILE_ENUMERATION_ENTRY_TYPE_DIRECTORY;
                } else {
                    context.type = XFILE_ENUMERATION_ENTRY_TYPE_FILE;
                }

                compat_makepath(context.name, drive, dir, entryName, NULL);

                if (!handler(&context)) {
                    break;
                }
            } while (fileFindNext(&directoryFileFindData));
        }
        return findFindClose(&directoryFileFindData);
    }

    XBase* xbase = rust_get_g_xbase_head();
    while (xbase != nullptr) {
        if (xbase->isDbase) {
            DFileFindData dbaseFindData;
            if (dbaseFindFirstEntry(xbase->dbase, &dbaseFindData, pattern)) {
                context.type = XFILE_ENUMERATION_ENTRY_TYPE_DFILE;

                do {
                    strcpy(context.name, dbaseFindData.fileName);
                    if (!handler(&context)) {
                        return dbaseFindClose(xbase->dbase, &dbaseFindData);
                    }
                } while (dbaseFindNextEntry(xbase->dbase, &dbaseFindData));

                dbaseFindClose(xbase->dbase, &dbaseFindData);
            }
        } else {
            char path[COMPAT_MAX_PATH];
            snprintf(path, sizeof(path), "%s\\%s", xbase->path, pattern);
            compat_windows_path_to_native(path);

            if (fileFindFirst(path, &directoryFileFindData)) {
                do {
                    bool isDirectory = fileFindIsDirectory(&directoryFileFindData);
                    char* entryName = fileFindGetName(&directoryFileFindData);

                    if (isDirectory) {
                        if (strcmp(entryName, "..") == 0 || strcmp(entryName, ".") == 0) {
                            continue;
                        }

                        context.type = XFILE_ENUMERATION_ENTRY_TYPE_DIRECTORY;
                    } else {
                        context.type = XFILE_ENUMERATION_ENTRY_TYPE_FILE;
                    }

                    compat_makepath(context.name, drive, dir, entryName, nullptr);

                    if (!handler(&context)) {
                        break;
                    }
                } while (fileFindNext(&directoryFileFindData));
            }
            findFindClose(&directoryFileFindData);
        }
        xbase = xbase->next;
    }

    compat_splitpath(nativePattern, drive, dir, fileName, extension);
    if (fileFindFirst(nativePattern, &directoryFileFindData)) {
        do {
            bool isDirectory = fileFindIsDirectory(&directoryFileFindData);
            char* entryName = fileFindGetName(&directoryFileFindData);

            if (isDirectory) {
                if (strcmp(entryName, "..") == 0 || strcmp(entryName, ".") == 0) {
                    continue;
                }

                context.type = XFILE_ENUMERATION_ENTRY_TYPE_DIRECTORY;
            } else {
                context.type = XFILE_ENUMERATION_ENTRY_TYPE_FILE;
            }

            compat_makepath(context.name, drive, dir, entryName, nullptr);

            if (!handler(&context)) {
                break;
            }
        } while (fileFindNext(&directoryFileFindData));
    }
    return findFindClose(&directoryFileFindData);
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

// xbase atexit
static void xbaseExitHandler()
{
    // NOTE: Uninline.
    rust_xbase_close_all();
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
