#![allow(unused)]
//! Functions for manipulating values on files.

use super::{
    core::{list_xattrs, remove_xattr, set_xattr, Xattr},
    tag::DirEntryExt,
    Error, Result as XResult,
};
use crate::{
    consts::WUTAG_NAMESPACE,
    g,
    registry::types::{Tag, Value},
};
use colored::Colorize;
use std::{convert::TryFrom, path::Path};

/// Get the `next` item or return an `Error`
macro_rules! next_or_else {
    ($it:ident, $msg:expr) => {
        $it.next()
            .ok_or_else(|| Error::InvalidTagKey($msg.to_owned()))
    };
}

impl Value {
    /// Custom implementation of `Hash`
    #[allow(clippy::same_name_method)]
    pub(super) fn hash(&self) -> XResult<String> {
        serde_cbor::to_vec(&self)
            .map(|val| format!("{}.{}", WUTAG_NAMESPACE, base64::encode(val)))
            .map_err(Error::from)
    }

    // XXX: Remove
    /// Tags the file at the given `path` with a [`Value`].
    ///
    /// # Errors
    /// If the tag that exists already has the same [`Value`]
    pub(crate) fn save_to<P>(&self, path: P, tag: &Tag) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        for value in list_values(path.as_ref(), tag)? {
            if &value == self {
                return Err(Error::ValueExists(g!((value.name())), g!((tag.name()))));
            }
        }
        set_xattr(path, self.hash()?.as_str(), "")
    }

    // XXX: Finish
    /// Removes a [`Value`] from a tag on file at the given `path`.
    ///
    /// # Errors
    /// If the tag doesn't exist the error [`TagNotFound`] is returned
    ///
    /// [`TagNotFound`]: crate::xattr::Error::TagNotFound
    pub(crate) fn remove_from<P>(&self, path: P, tag: &Tag) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        let val_hash = self.hash()?;
        let tag_hash = tag.hash()?;
        let path = &path.as_ref().to_owned();
        let tag_name = tag.name();

        for xattr in list_xattrs(path)? {
            let key = xattr.key();
            let val = xattr.val();

            // Make sure to only remove attributes corresponding to this namespace
            if key == tag_hash {
                // First, remove the tag (which removes the value(s))
                if path.untag(tag).is_err() {
                    return Err(Error::Untagging(
                        tag_name.to_string(),
                        path.to_string_lossy().to_string(),
                    ));
                }

                // return remove_xattr(path, key);
            }
        }

        Err(Error::TagValueNotFound(g!((self.name())), g!(tag_name)))
    }

    // XXX: Finish
    /// Removes all [`Value`]s from the file at the given `path`.
    ///
    /// # Errors
    /// If no values exist the error [`ValueNotFound`] is returned
    ///
    /// [`TagNotFound`]: crate::xattr::Error::TagNotFound
    #[allow(dead_code)]
    pub(crate) fn remove_all_from<P>(&self, path: P) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        for xattr in list_xattrs(path.as_ref())? {
            let key = xattr.key();
            // Make sure to only remove attributes corresponding to this namespace
            if key.starts_with(WUTAG_NAMESPACE) {
                return remove_xattr(path, key);
            }
        }

        Err(Error::ValueNotFound(g!((self.name()))))
    }

    /// Parse an extended attribute in a [`Value`].
    /// This function also matches against a [`Tag`]
    pub(crate) fn parse_xattr<T>(xattr: &Xattr, tag: T) -> XResult<Self>
    where
        T: AsRef<str>,
    {
        let key = xattr.key();
        let val = xattr.val();

        let mut key_elems = key.split("wutag.");
        let ns = next_or_else!(key_elems, "missing namespace `user`")?;
        if ns != "user." {
            return Err(Error::InvalidTagKey(format!(
                "invalid namespace `{}` for key",
                ns
            )));
        }

        let tag_bytes = next_or_else!(key_elems, "missing value")?;
        let tag_decode: Tag = serde_cbor::from_slice(&base64::decode(tag_bytes.as_bytes())?)?;

        let mut val_elems = val.split("wutag.");
        let ns = next_or_else!(val_elems, "missing namespace `user`")?;
        if ns != "user." {
            return Err(Error::InvalidValueVal(format!(
                "invalid namespace `{}` for value",
                ns
            )));
        }

        let value_bytes = next_or_else!(val_elems, "missing value")?;
        let value = serde_cbor::from_slice(&base64::decode(value_bytes.as_bytes())?)?;

        if tag_decode.name() == tag.as_ref() {
            return Ok(value);
        }

        Err(Error::ValueNotFoundOnTag(
            String::from(value_bytes),
            String::from(tag_bytes),
        ))
    }
}

impl TryFrom<Xattr> for Value {
    type Error = Error;

    #[inline]
    fn try_from(xattr: Xattr) -> XResult<Self> {
        let val = xattr.val();
        let mut elems = val.split("wutag.");

        let ns = next_or_else!(elems, "missing namespace `user`")?;
        if ns != "user." {
            return Err(Error::InvalidValueVal(format!(
                "invalid namespace `{}` for value",
                ns
            )));
        }

        let value_bytes = next_or_else!(elems, "missing value")?;
        let value = serde_cbor::from_slice(&base64::decode(value_bytes.as_bytes())?)?;

        Ok(value)
    }
}

/// Return a [`Value`] for a given [`Tag`] and `path`
///
/// This function isn't really necessary. It is used to confirm that a [`Value`]
/// exists for a [`Tag`] on a [`File`].
///
/// # Errors
/// If the tag doesn't exist it returns [`TagNotFound`]
///
/// [`TagNotFound`]: crate::xattr::Error::TagNotFound
#[inline]
pub(crate) fn get_value<P, T, V>(path: P, tag: T, value: V) -> XResult<Value>
where
    P: AsRef<Path>,
    T: AsRef<str>,
    V: AsRef<str>,
{
    let path = path.as_ref();
    let tag = tag.as_ref();
    let value = value.as_ref();

    for value_ in list_xattrs(path)?
        .into_iter()
        .flat_map(|xattr| Value::parse_xattr(&xattr, tag))
    {
        if value_.name() == value {
            return Ok(value_);
        }
    }

    Err(Error::TagValueNotFound(g!(value), g!(tag)))
}

/// Lists values on a file at the given `path`.
///
/// # Errors
/// If there are no tags or if the collection into a [`Vec`] fails
#[inline]
pub(crate) fn list_all_values<P>(path: P) -> XResult<Vec<Value>>
where
    P: AsRef<Path>,
{
    list_xattrs(path).map(|attrs| {
        let mut values = Vec::new();
        let it = attrs
            .into_iter()
            .filter(|xattr| xattr.key().starts_with(WUTAG_NAMESPACE))
            .map(Value::try_from);

        for value in it.flatten() {
            values.push(value);
        }
        values
    })
}

/// Lists values of a [`Tag`] on a file at the given `path`.
///
/// # Errors
/// If there are no tags or if the collection into a [`Vec`] fails
#[inline]
pub(crate) fn list_values<P>(path: P, tag: &Tag) -> XResult<Vec<Value>>
where
    P: AsRef<Path>,
{
    list_xattrs(path).map(|attrs| {
        let mut values = Vec::new();
        let it = attrs
            .into_iter()
            .filter(|xattr| xattr.key().starts_with(WUTAG_NAMESPACE))
            .map(|value| Value::parse_xattr(&value, tag));

        for value in it.flatten() {
            values.push(value);
        }
        values
    })
}

// XXX: Finish
/// Clears all [`Values`] of the file at the given `path`.
///
/// # Errors
/// If clearing the extended attributes failed
#[inline]
pub(crate) fn clear_values<P>(path: P) -> XResult<()>
where
    P: AsRef<Path>,
{
    for xattr in list_xattrs(path.as_ref())?
        .iter()
        .filter(|xattr| xattr.key().starts_with(WUTAG_NAMESPACE))
    {
        remove_xattr(path.as_ref(), xattr.key())?;
    }

    Ok(())
}

/// Checks whether the given `path` has [`Values`] for a [`Tag`]
///
/// # Errors
/// If there is a failure in listing extended attributes
#[inline]
pub(crate) fn has_values<P>(path: P, tag: &Tag) -> XResult<bool>
where
    P: AsRef<Path>,
{
    list_values(path, tag).map(|values| !values.is_empty())
}

/// Checks whether the given `path` has any values.
///
/// # Errors
/// If there is a failure in listing extended attributes
#[inline]
pub(crate) fn has_any_values<P>(path: P) -> XResult<bool>
where
    P: AsRef<Path>,
{
    list_all_values(path).map(|values| !values.is_empty())
}
