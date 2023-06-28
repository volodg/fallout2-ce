use libc::c_char;

#[cfg(target_family = "windows")]
// #[link(name = "snappy")]
extern "C" {
    fn rust_compat_splitpath(path: *const c_char, drive: *const c_char, dir: *const c_char, fname: *const c_char, ext: *const c_char);
}

#[no_mangle]
#[cfg(target_family = "windows")]
pub extern "C" fn rust_compat_splitpath(path: *const c_char, drive: *const c_char, dir: *const c_char, fname: *const c_char, ext: *const c_char) {
    _splitpath(path, drive, dir, fname, ext)
}

#[no_mangle]
#[cfg(not(target_family = "windows"))]
pub extern "C" fn rust_compat_splitpath(path: *const c_char, drive: *const c_char, dir: *const c_char, fname: *const c_char, ext: *const c_char) {

}

/*
void compat_splitpath(const char* path, char* drive, char* dir, char* fname, char* ext)
{
#ifdef _WIN32
    _splitpath(path, drive, dir, fname, ext);
#else
    const char* driveStart = path;
    if (path[0] == '/' && path[1] == '/') {
        path += 2;
        while (*path != '\0' && *path != '/' && *path != '.') {
            path++;
        }
    }

    if (drive != nullptr) {
        size_t driveSize = path - driveStart;
        if (driveSize > COMPAT_MAX_DRIVE - 1) {
            driveSize = COMPAT_MAX_DRIVE - 1;
        }
        strncpy(drive, path, driveSize);
        drive[driveSize] = '\0';
    }

    const char* dirStart = path;
    const char* fnameStart = path;
    const char* extStart = nullptr;

    const char* end = path;
    while (*end != '\0') {
        if (*end == '/') {
            fnameStart = end + 1;
        } else if (*end == '.') {
            extStart = end;
        }
        end++;
    }

    if (extStart == nullptr) {
        extStart = end;
    }

    if (dir != nullptr) {
        size_t dirSize = fnameStart - dirStart;
        if (dirSize > COMPAT_MAX_DIR - 1) {
            dirSize = COMPAT_MAX_DIR - 1;
        }
        strncpy(dir, path, dirSize);
        dir[dirSize] = '\0';
    }

    if (fname != nullptr) {
        size_t fileNameSize = extStart - fnameStart;
        if (fileNameSize > COMPAT_MAX_FNAME - 1) {
            fileNameSize = COMPAT_MAX_FNAME - 1;
        }
        strncpy(fname, fnameStart, fileNameSize);
        fname[fileNameSize] = '\0';
    }

    if (ext != nullptr) {
        size_t extSize = end - extStart;
        if (extSize > COMPAT_MAX_EXT - 1) {
            extSize = COMPAT_MAX_EXT - 1;
        }
        strncpy(ext, extStart, extSize);
        ext[extSize] = '\0';
    }
#endif
}
 */
