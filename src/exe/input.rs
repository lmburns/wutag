use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

/// Remove the `./` prefix from a path.
pub(crate) fn strip_current_dir(path: &Path) -> &Path {
    path.strip_prefix(".").unwrap_or(path)
}

/// Removes the parent component of the path
pub(crate) fn basename(path: &Path) -> &OsStr {
    path.file_name().unwrap_or(path.as_os_str())
}

/// Execute the command in specified directory
pub(crate) fn wutag_dir(path: &Path) -> OsString {
    let mut wutag = OsString::new();
    wutag.push("wutag -d ");
    wutag.push(dirname(path));
    wutag
}

/// Execute the command in specified directory (colored)
pub(crate) fn wutag_colored_dir(path: &Path) -> OsString {
    let mut wutag = OsString::new();
    wutag.push("wutag --color=always -d ");
    wutag.push(dirname(path));
    wutag
}

/// Execute `set` command in specified directory
pub(crate) fn wutag_set_tag(path: &Path) -> OsString {
    let mut wutag = OsString::new();
    wutag.push("wutag --color=always -d ");
    wutag.push(dirname(path));
    wutag.push(" set ");
    wutag.push(basename(path));
    wutag
}

/// Execute `remove` command in specified directory
pub(crate) fn wutag_remove_tag(path: &Path) -> OsString {
    let mut wutag = OsString::new();
    wutag.push("wutag --color=always -d ");
    wutag.push(dirname(path));
    wutag.push(" remove ");
    wutag.push(basename(path));
    wutag
    // wutag.push(format!("wutag --color=always -d {}", dir));
}

/// Execute `clear` command in specified directory
pub(crate) fn wutag_clear_tag(path: &Path) -> OsString {
    let mut wutag = OsString::new();
    wutag.push("wutag --color=always -d ");
    wutag.push(dirname(path));
    wutag.push(" clear ");
    wutag.push(basename(path));
    wutag
    // wutag.push(format!("wutag --color=always -d {}", dir));
}

/// Execute `cp` command in specified directory
pub(crate) fn wutag_cp_tag(path: &Path) -> OsString {
    let mut wutag = OsString::new();
    wutag.push("wutag --color=always -d ");
    wutag.push(dirname(path));
    wutag.push(" cp ");
    wutag.push(path);
    wutag
    // wutag.push(format!("wutag --color=always -d {}", dir));
}

/// Removes the extension from the path
pub(crate) fn remove_extension(path: &Path) -> OsString {
    let dirname = dirname(path);
    let stem = path.file_stem().unwrap_or(path.as_os_str());

    let path = PathBuf::from(dirname).join(stem);

    strip_current_dir(&path).to_owned().into_os_string()
}

/// Removes the basename from the path.
pub(crate) fn dirname(path: &Path) -> OsString {
    path.parent().map_or_else(
        || path.as_os_str().to_owned(),
        |p| {
            if p == OsStr::new("") {
                OsString::from(".")
            } else {
                p.as_os_str().to_owned()
            }
        },
    )
}
