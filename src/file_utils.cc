// NOTE: For unknown reason functions in this module use __stdcall instead
// of regular __usercall.

#include "file_utils.h"

#include <cstdio>
#include <zlib.h>

// TODO Migrate

// Migrated
#include "platform_compat.h"

extern "C" {
    void rust_file_copy(const char* existingFilePath, const char* newFilePath);
    int rust_file_copy_decompressed(const char* existingFilePath, const char* newFilePath);
}

namespace fallout {

// 0x452740
int fileCopyDecompressed(const char* existingFilePath, const char* newFilePath)
{
    return rust_file_copy_decompressed(existingFilePath, newFilePath);
}

// 0x452804
int fileCopyCompressed(const char* existingFilePath, const char* newFilePath)
{
    FILE* inStream = compat_fopen(existingFilePath, "rb");
    if (inStream == nullptr) {
        return -1;
    }

    int magic[2];
    magic[0] = fgetc(inStream);
    magic[1] = fgetc(inStream);
    rewind(inStream);

    if (magic[0] == 0x1F && magic[1] == 0x8B) {
        // Source file is already gzipped, there is no need to do anything
        // besides copying.
        fclose(inStream);
        rust_file_copy(existingFilePath, newFilePath);
    } else {
        gzFile outStream = compat_gzopen(newFilePath, "wb");
        if (outStream == nullptr) {
            fclose(inStream);
            return -1;
        }

        // Copy byte-by-byte.
        for (;;) {
            int ch = fgetc(inStream);
            if (ch == -1) {
                break;
            }

            gzputc(outStream, ch);
        }

        fclose(inStream);
        gzclose(outStream);
    }

    return 0;
}

// TODO: Check, implementation looks odd.
int _gzdecompress_file(const char* existingFilePath, const char* newFilePath)
{
    FILE* stream = compat_fopen(existingFilePath, "rb");
    if (stream == nullptr) {
        return -1;
    }

    int magic[2];
    magic[0] = fgetc(stream);
    magic[1] = fgetc(stream);
    fclose(stream);

    // TODO: Is it broken?
    if (magic[0] != 0x1F || magic[1] != 0x8B) {
        gzFile gzstream = compat_gzopen(existingFilePath, "rb");
        if (gzstream == nullptr) {
            return -1;
        }

        stream = compat_fopen(newFilePath, "wb");
        if (stream == nullptr) {
            gzclose(gzstream);
            return -1;
        }

        while (true) {
            int ch = gzgetc(gzstream);
            if (ch == -1) {
                break;
            }

            fputc(ch, stream);
        }

        gzclose(gzstream);
        fclose(stream);
    } else {
        rust_file_copy(existingFilePath, newFilePath);
    }

    return 0;
}

} // namespace fallout
