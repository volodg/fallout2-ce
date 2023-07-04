// NOTE: For unknown reason functions in this module use __stdcall instead
// of regular __usercall.

#include "file_utils.h"

// TODO Migrate

extern "C" {
    int rust_file_copy_decompressed(const char* existingFilePath, const char* newFilePath);
    int rust_file_copy_compressed(const char* existingFilePath, const char* newFilePath);
    int rust_gzdecompress_file(const char* existingFilePath, const char* newFilePath);
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
    return rust_file_copy_compressed(existingFilePath, newFilePath);
}

// TODO: Check, implementation looks odd.
int _gzdecompress_file(const char* existingFilePath, const char* newFilePath)
{
    return rust_gzdecompress_file(existingFilePath, newFilePath);
}

} // namespace fallout
