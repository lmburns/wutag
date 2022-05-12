//! Safe and os-agnostic(TODO) wrappers for manipulating extra attributes
#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(unix)]
use unix::{
    get_xattr as _get_xattr, list_xattrs as _list_xattrs, remove_xattr as _remove_xattr,
    set_xattr as _set_xattr,
};

#[cfg(windows)]
pub use windows::{
    get_xattr as _get_xattr, list_xattrs as _list_xattrs, remove_xattr as _remove_xattr,
    set_xattr as _set_xattr,
};

use crate::Result;
use std::path::Path;

/// Extended attribute representation
#[derive(Debug)]
pub struct Xattr {
    /// Key of the [`Xattr`] (i.e., `key`.value)
    key: String,
    /// Value of the [`Xattr`] (i.e., key.`value`)
    val: String,
}

impl Xattr {
    /// Create a new [`Xattr`]
    pub fn new<K, V>(key: K, val: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self {
            key: key.into(),
            val: val.into(),
        }
    }

    /// Return the `key` of the [`Xattr`]
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Return the `val` of the `Xattr`
    #[must_use]
    pub fn val(&self) -> &str {
        &self.val
    }
}

impl From<(String, String)> for Xattr {
    fn from(xattr: (String, String)) -> Self {
        Self::new(xattr.0, xattr.1)
    }
}

/// Set an extended attribute on a file
/// [`set_xattr`] -> [`_set_xattr`] -> [`__set_xattr`]
///
/// # Errors
/// Will raise an error if an `xattr` is unable to be set on a file/directory
///
/// [`set_xattr`]: crate::xattr::unix::set_xattr
/// [`_set_xattr`]: crate::xattr::unix::_set_xattr
/// [`__set_xattr`]: crate::xattr::unix::__set_xattr
pub fn set_xattr<P, S>(path: P, name: S, value: S) -> Result<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    _set_xattr(path, name, value)
}

/// Get an extended attribute
/// [`get_xattr`] -> [`_get_xattr`] -> [`__get_xattr`]
///
/// # Errors
/// Will raise an error if an `xattr` is unable to be gotten on a file/directory
///
/// [`get_xattr`]: crate::xattr::unix::get_xattr
/// [`_get_xattr`]: crate::xattr::unix::_get_xattr
/// [`__get_xattr`]: crate::xattr::unix::__get_xattr
pub fn get_xattr<P, S>(path: P, name: S) -> Result<String>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    _get_xattr(path, name)
}

/// List extended attribute(s)
/// [`list_xattr`] -> [`_list_xattr`] -> [`__list_xattr`]
///
/// # Errors
/// Will raise an error if an `xattr` is unable to be listed on a file/directory
///
/// [`list_xattr`]: crate::xattr::unix::list_xattr
/// [`_list_xattr`]: crate::xattr::unix::_list_xattr
/// [`__list_xattr`]: crate::xattr::unix::__list_xattr
pub fn list_xattrs<P>(path: P) -> Result<Vec<Xattr>>
where
    P: AsRef<Path>,
{
    _list_xattrs(path).map(|attrs| attrs.into_iter().map(From::from).collect())
}

/// Remove an extended attribute
/// [`remove_xattr`] -> [`_remove_xattr`] -> [`__remove_xattr`]
///
/// # Errors
/// Will raise an error if an `xattr` is unable to be removed on a
/// file/directory
///
/// [`remove_xattr`]: crate::xattr::unix::remove_xattr
/// [`_remove_xattr`]: crate::xattr::unix::_remove_xattr
/// [`__remove_xattr`]: crate::xattr::unix::__remove_xattr
pub fn remove_xattr<P, S>(path: P, name: S) -> Result<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    _remove_xattr(path, name)
}
