use std::ffi::{c_void, CString};
use std::mem;
use std::ptr::null_mut;
use libc::{c_char, c_int, c_uchar, fclose, fgetc, fputc, fread, fwrite, rewind, size_t};
use libz_sys::{gzclose, gzgetc, gzputc};
use crate::platform_compat::{rust_compat_fopen, rust_compat_gzopen};

unsafe fn file_copy(existing_file_path: *const c_char, new_file_path: *const c_char) {
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

#[no_mangle]
pub unsafe extern "C" fn rust_file_copy_decompressed(existing_file_path: *const c_char, new_file_path: *const c_char) -> c_int {
    let rb = CString::new("rb").expect("valid string");

    let stream = rust_compat_fopen(existing_file_path, rb.as_ptr());
    if stream == null_mut() {
        return -1;
    }

    let magic = [fgetc(stream), fgetc(stream)];
    fclose(stream);

    if magic[0] == 0x1F && magic[1] == 0x8B {
        let in_stream = rust_compat_gzopen(existing_file_path, rb.as_ptr());
        let wb = CString::new("wb").expect("valid string");
        let out_stream = rust_compat_fopen(new_file_path, wb.as_ptr());

        if in_stream != null_mut() && out_stream != null_mut() {
            loop {
                let ch = gzgetc(in_stream);
                if ch == -1 {
                    break;
                }

                fputc(ch, out_stream);
            }

            gzclose(in_stream);
            fclose(out_stream);
        } else {
            if in_stream != null_mut() {
                gzclose(in_stream);
            }

            if out_stream != null_mut() {
                fclose(out_stream);
            }

            return -1;
        }
    } else {
        file_copy(existing_file_path, new_file_path);
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_file_copy_compressed(existing_file_path: *const c_char, new_file_path: *const c_char) -> c_int {
    let rb = CString::new("rb").expect("valid string");

    let in_stream = rust_compat_fopen(existing_file_path, rb.as_ptr());
    if in_stream == null_mut() {
        return -1;
    }

    let magic = [fgetc(in_stream), fgetc(in_stream)];
    rewind(in_stream);

    if magic[0] == 0x1F && magic[1] == 0x8B {
        // Source file is already gzipped, there is no need to do anything
        // besides copying.
        fclose(in_stream);
        file_copy(existing_file_path, new_file_path);
    } else {
        let wb = CString::new("wb").expect("valid string");
        let out_stream = rust_compat_gzopen(new_file_path, wb.as_ptr());
        if out_stream == null_mut() {
            fclose(in_stream);
            return -1;
        }

        // Copy byte-by-byte.
        loop {
            let ch = fgetc(in_stream);
            if ch == -1 {
                break;
            }

            gzputc(out_stream, ch);
        }

        fclose(in_stream);
        gzclose(out_stream);
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn rust_gzdecompress_file(existing_file_path: *const c_char, new_file_path: *const c_char) -> c_int {
    let rb = CString::new("rb").expect("valid string");
    let mut stream = rust_compat_fopen(existing_file_path, rb.as_ptr());
    if stream == null_mut() {
        return -1;
    }

    let magic = [fgetc(stream), fgetc(stream)];
    fclose(stream);

    // TODO: Is it broken?
    if magic[0] != 0x1F || magic[1] != 0x8B {
        let gzstream = rust_compat_gzopen(existing_file_path, rb.as_ptr());
        if gzstream == null_mut() {
            return -1;
        }

        let wb = CString::new("wb").expect("valid string");
        stream = rust_compat_fopen(new_file_path, wb.as_ptr());
        if stream == null_mut() {
            gzclose(gzstream);
            return -1;
        }

        loop {
            let ch = gzgetc(gzstream);
            if ch == -1 {
                break;
            }

            fputc(ch, stream);
        }

        gzclose(gzstream);
        fclose(stream);
    } else {
        file_copy(existing_file_path, new_file_path);
    }

    0
}
