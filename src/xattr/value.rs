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

// TODO: If tag has multiple values

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

    /// Removes a [`Value`] from a tag on file at the given `path`.
    ///
    /// # Errors
    /// If the tag doesn't exist the error [`TagValueNotFound`] is returned
    ///
    /// [`TagValueNotFound`]: crate::xattr::Error:TagValueNotFound:
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

            println!("iter xattr: {:#?}", xattr);

            // Make sure to only remove attributes corresponding to this tag
            if key == tag_hash && val == val_hash {
                println!("===== IS A MATCH =====");
                // First, remove the tag (which removes the value(s))
                remove_xattr(path, key)?;
                return path.tag(tag, None);
            }
        }

        Err(Error::TagValueNotFound(g!((self.name())), g!(tag_name)))
    }

    /// Removes all [`Value`]s from the file at the given `path`.
    ///
    /// # Errors
    /// If no values exist the error [`ValueNotFound`] is returned
    ///
    /// [`ValueNotFound`]: crate::xattr::Error::ValueNotFound
    pub(crate) fn remove_all_from<P>(&self, path: P) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        let val_hash = self.hash()?;
        let path = &path.as_ref().to_owned();

        for xattr in list_xattrs(path)? {
            let key = xattr.key();
            let val = xattr.val();

            if val == val_hash {
                remove_xattr(path, key)?;
                let tag = Tag::try_from(xattr)?;
                return path.tag(&tag, None);
            }
        }

        Err(Error::ValueNotFound(g!((self.name()))))
    }

    /// Replace a [`Value`] with another [`Value`] for a given [`Tag`]
    ///
    /// # Errors
    /// If the value doesn't exist the error [`TagValueNotFound`] is returned
    ///
    /// [`TagValueNotFound`]: crate::xattr::Error::TagValueNotFound
    pub(crate) fn update<P>(&self, path: P, tag: &Tag, replacer: &Self) -> XResult<()>
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

            // Make sure to only remove attributes corresponding to this tag
            if key == tag_hash && val == val_hash {
                // First, remove the tag (which removes the value(s))
                remove_xattr(path, key)?;
                return path.tag(tag, Some(replacer));
            }
        }

        Err(Error::TagValueNotFound(g!((self.name())), g!(tag_name)))
    }

    /// Add a [`Value`] to an already existing [`Tag`]
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn add<P>(&self, path: P, tag: &Tag) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        let val_hash = self.hash()?;
        let tag_hash = tag.hash()?;
        let path = &path.as_ref().to_owned();
        let tag_name = tag.name();

        Ok(())
    }

    /// Parse an extended attribute in a [`Value`].
    /// This function also matches against a [`Tag`]
    ///
    /// # Errors
    /// If the extended attribute fails to parse, a [`ValueNotFoundOnTag`] error
    /// is thrown. This can happen because:
    /// 1. [`Tag`] does not exist
    /// 2. [`Tag`] does not have the [`Value`]
    ///
    /// If the extended attribute's namespace is not `wutag`, then an
    /// [`InvalidValueVal`] or an [`InvalidTagKey`] will be thrown
    ///
    /// [`ValueNotFoundOnTag`]: crate::xattr::Error::ValueNotFoundOnTag
    /// [`InvalidValueVal`]: crate::xattr::Error::InvalidValueVal
    /// [`InvalidTagKey`]: crate::xattr::Error::InvalidTagKey
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
            .filter(|xattr| {
                println!("XATTR; {:#?}", xattr);

                xattr.key().starts_with(WUTAG_NAMESPACE)
            })
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
            .filter(|xattr| {
                println!("Xattr value: {:#?}", xattr);
                xattr.key().starts_with(WUTAG_NAMESPACE)
            })
            .map(|value| Value::parse_xattr(&value, tag));

        for value in it.flatten() {
            values.push(value);
        }
        values
    })
}

/// Clear a single value
pub(crate) fn clear_value<P>(path: P, xattr: Xattr) -> XResult<()>
where
    P: AsRef<Path>,
{
    let path = &path.as_ref().to_owned();
    remove_xattr(path, xattr.key())?;
    let tag = Tag::try_from(xattr)?;
    path.tag(&tag, None);

    Ok(())
}

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
        let path = &path.as_ref().to_owned();
        remove_xattr(path, xattr.key())?;
        let tag = Tag::try_from(xattr.clone())?;
        path.tag(&tag, None);
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
