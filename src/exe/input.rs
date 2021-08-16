use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

// pub fn default_path_separator() -> Option<String> {
//     if cfg!(windows) {
//         let msystem = std::env::var("MSYSTEM").ok()?;
//         match msystem.as_str() {
//             "MINGW64" | "MINGW32" | "MSYS" => Some("/".to_owned()),
//             _ => None,
//         }
//     } else {
//         None
//     }
// }

/// Remove the `./` prefix from a path.
pub fn strip_current_dir(path: &Path) -> &Path {
    path.strip_prefix(".").unwrap_or(path)
}

/// Removes the parent component of the path
pub fn basename(path: &Path) -> &OsStr {
    path.file_name().unwrap_or_else(|| path.as_os_str())
}

/// Removes the extension from the path
pub fn remove_extension(path: &Path) -> OsString {
    let dirname = dirname(path);
    let stem = path.file_stem().unwrap_or_else(|| path.as_os_str());

    let path = PathBuf::from(dirname).join(stem);

    strip_current_dir(&path).to_owned().into_os_string()
}

/// Removes the basename from the path.
pub fn dirname(path: &Path) -> OsString {
    path.parent()
        .map(|p| {
            if p == OsStr::new("") {
                OsString::from(".")
            } else {
                p.as_os_str().to_owned()
            }
        })
        .unwrap_or_else(|| path.as_os_str().to_owned())
}
