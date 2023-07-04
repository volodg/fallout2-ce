use std::ffi::{c_void, CString};
use std::mem;
use std::ptr::null_mut;
use libc::{c_char, c_uchar, fclose, fread, fwrite, size_t};
use crate::platform_compat::rust_compat_fopen;

#[no_mangle]
pub unsafe extern "C" fn rust_file_copy(existing_file_path: *const c_char, new_file_path: *const c_char) {
    let rb = CString::new("rb").expect("valid string");
    let wb = CString::new("wb").expect("valid string");
    let in_ = rust_compat_fopen(existing_file_path, rb.as_ptr());
    let out = rust_compat_fopen(new_file_path, wb.as_ptr());
    if in_ != null_mut() && out != null_mut() {
        let mut buffer = [0 as c_uchar; 0xFFFF];

        let mut bytes_read = fread(buffer.as_mut_ptr() as *mut c_void, mem::size_of_val(&buffer[0]), buffer.len(), in_) as isize;
        while bytes_read > 0 {
            let mut offset = 0;
            let mut bytes_written = fwrite(buffer.as_mut_ptr().offset(offset) as *mut c_void, mem::size_of_val(&buffer[0]), bytes_read as size_t, out) as isize;
            while bytes_written > 0 {
                bytes_read -= bytes_written;
                offset += bytes_written;
                bytes_written = fwrite(buffer.as_mut_ptr().offset(offset) as *mut c_void, mem::size_of_val(&buffer[0]), bytes_read as size_t, out) as isize;
            }

            if bytes_written < 0 {
                break;
            }

            bytes_read = fread(buffer.as_mut_ptr() as *mut c_void, mem::size_of_val(&buffer), buffer.len(), in_) as isize;
        }
    }

    if in_ != null_mut() {
        fclose(in_);
    }

    if out != null_mut() {
        fclose(out);
    }
}

/*
static void fileCopy(const char* existingFilePath, const char* newFilePath)
{
}
 */

/*
#[no_mangle]
pub unsafe extern "C" fn rust_file_copy_decompressed(existing_file_path: *const c_char, new_file_path: *const c_char) -> c_int {
    FILE* stream = compat_fopen(existingFilePath, "rb");
    if stream == null() {
        return -1;
    }

    let magic = [c_int; 2];
    magic[0] = fgetc(stream);
    magic[1] = fgetc(stream);
    fclose(stream);

    if magic[0] == 0x1F && magic[1] == 0x8B {
        let inStream = rust_compat_gzopen(existingFilePath, "rb");
        FILE* outStream = rust_compat_fopen(newFilePath, "wb");

        if inStream != null() && outStream != null() {
            loop {
                let ch = gzgetc(inStream);
                if ch == -1 {
                    break;
                }

                fputc(ch, outStream);
            }

            gzclose(inStream);
            fclose(outStream);
        } else {
            if inStream != null() {
                gzclose(inStream);
            }

            if outStream != null() {
                fclose(outStream);
            }

            return -1;
        }
    } else {
        fileCopy(existingFilePath, newFilePath);
    }

    0
}*/

/*
int fileCopyDecompressed(const char* existingFilePath, const char* newFilePath)
{
    FILE* stream = compat_fopen(existingFilePath, "rb");
    if (stream == nullptr) {
        return -1;
    }

    int magic[2];
    magic[0] = fgetc(stream);
    magic[1] = fgetc(stream);
    fclose(stream);

    if (magic[0] == 0x1F && magic[1] == 0x8B) {
        gzFile inStream = compat_gzopen(existingFilePath, "rb");
        FILE* outStream = compat_fopen(newFilePath, "wb");

        if (inStream != nullptr && outStream != nullptr) {
            for (;;) {
                int ch = gzgetc(inStream);
                if (ch == -1) {
                    break;
                }

                fputc(ch, outStream);
            }

            gzclose(inStream);
            fclose(outStream);
        } else {
            if (inStream != nullptr) {
                gzclose(inStream);
            }

            if (outStream != nullptr) {
                fclose(outStream);
            }

            return -1;
        }
    } else {
        fileCopy(existingFilePath, newFilePath);
    }

    return 0;
}
 */
