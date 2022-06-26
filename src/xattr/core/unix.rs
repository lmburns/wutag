#![allow(clippy::cast_sign_loss)]
#![allow(clippy::ptr_as_ptr)]
#![cfg(unix)]

use super::{Error, Result};
use colored::Colorize;

#[cfg(target_os = "macos")]
use libc::XATTR_NOFOLLOW;

#[cfg(target_os = "linux")]
use libc::{c_char, c_void, lgetxattr, llistxattr, lremovexattr, lsetxattr, size_t, ssize_t};

use libc::{getxattr, listxattr, removexattr, setxattr, XATTR_CREATE};

use std::{
    ffi::{CStr, CString, OsStr},
    fs, io, mem,
    os::unix::ffi::OsStrExt,
    path::Path,
    ptr,
};

/// Check whether the given `Path` is a symlink
fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).map_or(false, |f| f.file_type().is_symlink())
}

/// Sets the value of the extended attribute identified by `name` and associated
/// with the given `path` in the filesystem.
pub(super) fn set_xattr<P, S>(path: P, name: S, value: S) -> Result<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let size = value.as_ref().as_bytes().len();
    let path = path.as_ref();

    _set_xattr(path, name.as_ref(), value.as_ref(), size, is_symlink(path))
}

/// Retrieves the value of the extended attribute identified by `name` and
/// associated with the given `path` in the filesystem.
pub(super) fn get_xattr<P, S>(path: P, name: S) -> Result<String>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let path = path.as_ref();
    _get_xattr(path, name.as_ref(), is_symlink(path))
}

/// Retrieves a list of all extended attributes with their values associated
/// with the given `path` in the filesystem.
pub(super) fn list_xattrs<P>(path: P) -> Result<Vec<(String, String)>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    _list_xattrs(path, is_symlink(path))
}

/// Removes the extended attribute identified by `name` and associated with the
/// given `path` in the filesystem.
pub(super) fn remove_xattr<P, S>(path: P, name: S) -> Result<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let path = path.as_ref();
    _remove_xattr(path, name.as_ref(), is_symlink(path))
}

//################################################################################
// Wrappers - Lowest level functions
//################################################################################

/// Call to the `C` function to get the extended attribute
///
///  - [`lgetxattr`] if the file is a symlink
///  - [`getxattr`] if the file is not a symlink
#[cfg(target_os = "linux")]
unsafe fn __getxattr(
    path: *const i8,
    name: *const i8,
    value: *mut c_void,
    size: usize,
    symlink: bool,
) -> isize {
    let func = if symlink { lgetxattr } else { getxattr };

    func(path, name, value, size)
}

/// Call to the `C` function to get the extended attribute
///
///  - [`XATTR_NOFOLLOW`] if the file is a symlink
///  - `0` if the file is not a symlink
#[cfg(target_os = "macos")]
unsafe fn __getxattr(
    path: *const i8,
    name: *const i8,
    value: *mut c_void,
    size: usize,
    symlink: bool,
) -> isize {
    let opts = if symlink { XATTR_NOFOLLOW } else { 0 };

    getxattr(path, name, value, size, 0, opts)
}

/// Call to the `C` function to set the extended attribute
///
///  - [`lsetxattr`] if the file is a symlink
///  - [`setxattr`] if the file is not a symlink
#[cfg(target_os = "linux")]
unsafe fn __setxattr(
    path: *const c_char,
    name: *const c_char,
    value: *const c_void,
    size: size_t,
    symlink: bool,
) -> ssize_t {
    let func = if symlink { lsetxattr } else { setxattr };

    func(path, name, value, size, XATTR_CREATE) as ssize_t
}

/// Call to the `C` function to set the extended attribute
///
///  - [`XATTR_NOFOLLOW`] if the file is a symlink
///  - `0` if the file is not a symlink
#[cfg(target_os = "macos")]
unsafe fn __setxattr(
    path: *const i8,
    name: *const i8,
    value: *const c_void,
    size: usize,
    symlink: bool,
) -> isize {
    let opts = if symlink { XATTR_NOFOLLOW } else { 0 };

    setxattr(path, name, value, size, 0, opts | XATTR_CREATE) as isize
}

/// Call to the `C` function to remove the extended attribute
///
///  - [`lremovexattr`] if the file is a symlink
///  - [`removexattr`] if the file is not a symlink
#[cfg(target_os = "linux")]
#[allow(clippy::as_conversions)]
unsafe fn __removexattr(path: *const i8, name: *const i8, symlink: bool) -> isize {
    let func = if symlink { lremovexattr } else { removexattr };

    func(path, name) as isize
}

/// Call to the `C` function to remove the extended attribute
///
///  - [`XATTR_NOFOLLOW`] if the file is a symlink
///  - `0` if the file is not a symlink
#[cfg(target_os = "macos")]
#[allow(clippy::as_conversions)]
unsafe fn __removexattr(path: *const i8, name: *const i8, symlink: bool) -> isize {
    let opts = if symlink { XATTR_NOFOLLOW } else { 0 };

    removexattr(path, name, opts) as isize
}

/// Call to the `C` function to list extended attribute(s)
///
///  - [`llistxattr`] if the file is a symlink
///  - [`listxattr`] if the file is not a symlink
#[cfg(target_os = "linux")]
unsafe fn __listxattr(path: *const i8, list: *mut i8, size: usize, symlink: bool) -> isize {
    let func = if symlink { llistxattr } else { listxattr };

    func(path, list, size)
}

/// Call to the `C` function to list extended attribute(s)
///
///  - [`XATTR_NOFOLLOW`] if the file is a symlink
///  - `0` if the file is not a symlink
#[cfg(target_os = "macos")]
unsafe fn __listxattr(path: *const i8, list: *mut i8, size: usize, symlink: bool) -> isize {
    let opts = if symlink { XATTR_NOFOLLOW } else { 0 };

    listxattr(path, list, size, opts | XATTR_CREATE) as isize
}

//################################################################################
// Impl
//################################################################################

/// See [`__removexattr`]
fn _remove_xattr(path: &Path, name: &str, symlink: bool) -> Result<()> {
    let path = CString::new(path.to_string_lossy().as_bytes())?;
    let name = CString::new(name.as_bytes())?;

    // Unsafe needs to be used to call [`libc`]
    unsafe {
        let ret = __removexattr(path.as_ptr(), name.as_ptr(), symlink);
        if ret != 0 {
            return Err(Error::from(io::Error::last_os_error()));
        }
    }

    Ok(())
}

/// See [`__setxattr`]
///
/// If provided path is a symlink, set the attribute on the symlink not the
/// file/directory it points to
///
/// NOTE: When setting an extended attribute on a symlink:
///       - Using 'user.' prefix => Operation not supported (Errno 1)
///       - Using any other prefix => Operation not permitted (Errno 95)
///       - For this to work, a privileged user must use 'trusted.' prefix
///
/// See: https://unix.stackexchange.com/questions/16537
/// See: https://stackoverflow.com/questions/65985725
///
/// In user.* namespace, only regular files and directories can have extended
/// attributes. For sticky directories, only the owner and privileged user
/// can write attributes.
///
/// The  file  permission  bits of regular files and directories are interpreted
/// differently from the file permission bits of special files and
/// symbolic links.  For regular files and directories the file permission bits
/// define access to the file's contents, while for device  special files  they
/// define  access  to  the  device  described by the special file.  The file
/// permissions of symbolic links are not used in access checks.  These
/// differences would allow users to consume filesystem resources in a way not
/// controllable by disk quotas for  group  or  world writable special files and
/// directories.
///
/// For this reason, user extended attributes are allowed only for regular files
/// and directories, and access to user extended attributes is restricted to
/// the owner and to users with appropriate capabilities for directories with
/// the sticky bit set (see the chmod(1) manual page  for an explanation of the
/// sticky bit).
fn _set_xattr(path: &Path, name: &str, value: &str, size: usize, symlink: bool) -> Result<()> {
    let path = CString::new(path.as_os_str().as_bytes())?;
    let name = {
        if symlink && cfg!(target_os = "linux") {
            CString::new(format!("trusted.{}", name).as_bytes())?
        } else {
            CString::new(name.as_bytes())?
        }
    };
    let value = CString::new(value.as_bytes())?;

    // value.as_ptr().cast::<libc::c_void>(),
    let ret = unsafe {
        __setxattr(
            path.as_ptr(),
            name.as_ptr(),
            value.as_ptr() as *const c_void,
            size,
            symlink,
        )
    };

    if ret != 0 {
        let last_os = io::Error::last_os_error();
        if last_os.raw_os_error() == Some(95) {
            return Err(Error::SymlinkUnavailable95(last_os.to_string()));
        }

        if last_os.raw_os_error() == Some(1) && symlink {
            return Err(Error::SymlinkUnavailable1(
                last_os.to_string(),
                "privileged".green().bold().to_string(),
            ));
        }

        return Err(Error::from(io::Error::last_os_error()));
    }

    Ok(())
}

/// See [`__getxattr`]
fn _get_xattr(path: &Path, name: &str, symlink: bool) -> Result<String> {
    let path = CString::new(path.to_string_lossy().as_bytes())?;
    let name = CString::new(name.as_bytes())?;
    let size = get_xattr_size(path.as_c_str(), name.as_c_str(), symlink)?;
    let mut buf = Vec::<u8>::with_capacity(size);
    let buf_ptr = buf.as_mut_ptr();

    mem::forget(buf);

    let ret = unsafe {
        __getxattr(
            path.as_ptr(),
            name.as_ptr(),
            // buf_ptr.cast::<libc::c_void>(),
            buf_ptr as *mut c_void,
            size,
            symlink,
        )
    };

    if ret == -1 {
        return Err(Error::from(io::Error::last_os_error()));
    }

    #[allow(clippy::as_conversions)]
    let ret = ret as usize;

    if ret != size {
        return Err(Error::AttrsChanged);
    }

    let buf = unsafe { Vec::from_raw_parts(buf_ptr, ret, size) };

    Ok(unsafe { CString::from_vec_unchecked(buf) }
        .to_string_lossy()
        .to_string())
}

/// See [`__listxattr`]
fn _list_xattrs(path: &Path, symlink: bool) -> Result<Vec<(String, String)>> {
    let cpath = CString::new(path.to_string_lossy().as_bytes())?;
    let raw = list_xattrs_raw(cpath.as_c_str(), symlink)?;
    let keys = parse_xattrs(&raw);

    let mut attrs = Vec::new();

    for key in keys {
        attrs.push((key.clone(), _get_xattr(path, key.as_str(), symlink)?));
    }

    Ok(attrs)
}

//################################################################################
// Other - Helper functions
//################################################################################

fn get_xattr_size(path: &CStr, name: &CStr, symlink: bool) -> Result<usize> {
    let ret = unsafe { __getxattr(path.as_ptr(), name.as_ptr(), ptr::null_mut(), 0, symlink) };

    if ret == -1 {
        return Err(Error::from(io::Error::last_os_error()));
    }

    #[allow(clippy::as_conversions)]
    Ok(ret as usize)
}

fn get_xattrs_list_size(path: &CStr, symlink: bool) -> Result<usize> {
    let ret = unsafe { __listxattr(path.as_ptr(), ptr::null_mut(), 0, symlink) };

    if ret == -1 {
        return Err(Error::from(io::Error::last_os_error()));
    }

    #[allow(clippy::as_conversions)]
    Ok(ret as usize)
}

fn list_xattrs_raw(path: &CStr, symlink: bool) -> Result<Vec<u8>> {
    let size = get_xattrs_list_size(path, symlink)?;
    let mut buf = Vec::<u8>::with_capacity(size);
    let buf_ptr = buf.as_mut_ptr();

    mem::forget(buf);

    let ret = unsafe { __listxattr(path.as_ptr(), buf_ptr as *mut c_char, size, symlink) };

    if ret == -1 {
        return Err(Error::from(io::Error::last_os_error()));
    }

    #[allow(clippy::as_conversions)]
    let ret = ret as usize;

    if ret != size {
        return Err(Error::AttrsChanged);
    }

    // its safe to construct a Vec here because original pointer to buf is forgotten
    // and the size of return buffer is verified against original size
    unsafe { Ok(Vec::from_raw_parts(buf_ptr, ret, size)) }
}

fn parse_xattrs(input: &[u8]) -> Vec<String> {
    let mut keys = Vec::new();
    let mut start = 0;

    for (i, ch) in input.iter().enumerate() {
        if *ch == b'\0' {
            keys.push(OsStr::from_bytes(&input[start..i]).to_string_lossy().to_string());
            start += i - start + 1;
        }
    }

    keys
}

#[test]
fn parses_xattrs_from_raw() {
    let raw = &[
        117, 115, 101, 114, 46, 107, 101, 121, 49, 0, 117, 115, 101, 114, 46, 107, 101, 121, 50, 0, 117,
        115, 101, 114, 46, 107, 101, 121, 51, 0, 115, 101, 99, 117, 114, 105, 116, 121, 46, 116, 101, 115,
        116, 105, 110, 103, 0, 119, 117, 116, 97, 103, 46, 118, 97, 108, 117, 101, 0,
    ];

    let attrs = parse_xattrs(raw);
    let mut it = attrs.iter();

    assert_eq!(it.next(), Some(&"user.key1".to_owned()));
    assert_eq!(it.next(), Some(&"user.key2".to_owned()));
    assert_eq!(it.next(), Some(&"user.key3".to_owned()));
    assert_eq!(it.next(), Some(&"security.testing".to_owned()));
    assert_eq!(it.next(), Some(&"wutag.value".to_owned()));
}
