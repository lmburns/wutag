#![allow(unused)]

use super::{
    core::{list_xattrs, remove_xattr, set_xattr, Xattr},
    tag::DirEntryExt,
    Error, Result as XResult,
};
use crate::{
    consts::{WUTAG_NAMESPACE, WUTAG_TAG_NAMESPACE, WUTAG_VALUE_NAMESPACE},
    registry::types::{Tag, Value},
};
use colored::Colorize;
use std::{
    collections::BTreeSet,
    convert::TryFrom,
    path::{Display, Path, PathBuf},
};

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
    fn hash(&self, tag: &Tag) -> XResult<String> {
        let encoded_tag = serde_cbor::to_vec(tag).map(base64::encode).map_err(Error::from)?;
        serde_cbor::to_vec(&self)
            .map(|value| {
                format!(
                    "{}.{}.{}",
                    WUTAG_VALUE_NAMESPACE,
                    encoded_tag,
                    base64::encode(value)
                )
            })
            .map_err(Error::from)
    }

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
                return Err(Error::ValueExists(value.name().green().bold()));
            }
        }
        set_xattr(path, self.hash(tag)?.as_str(), "")
    }

    /// Removes this [`Value`] from the file at the given `path`.
    ///
    /// # Errors
    /// If the tag doesn't exist the error [`TagNotFound`] is returned
    ///
    /// [`TagNotFound`]: crate::xattr::Error::TagNotFound
    pub(crate) fn remove_from<P>(&self, path: P, tag: &Tag) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        let hash = self.hash(tag)?;

        for xattr in list_xattrs(path.as_ref())? {
            let key = xattr.key();
            // Make sure to only remove attributes corresponding to this namespace
            if key == hash {
                return remove_xattr(path, key);
            }
        }

        Err(Error::TagNotFound(tag.name().clone()))
    }

    /// Removes all [`Value`]s from the file at the given `path`.
    ///
    /// # Errors
    /// If no values exist the error [`ValueNotFound`] is returned
    ///
    /// [`TagNotFound`]: crate::xattr::Error::TagNotFound
    pub(crate) fn remove_all_from<P>(&self, path: P) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        for xattr in list_xattrs(path.as_ref())? {
            let key = xattr.key();
            // Make sure to only remove attributes corresponding to this namespace
            if key.starts_with(WUTAG_VALUE_NAMESPACE) {
                return remove_xattr(path, key);
            }
        }

        Err(Error::ValueNotFound(self.name().clone()))
    }

    /// Parse an extended attribute in a [`Value`].
    /// This function also matches against a [`Tag`]
    pub(crate) fn parse_xattr<T>(xattr: &Xattr, tag: T) -> XResult<Self>
    where
        T: AsRef<str>,
    {
        let key = xattr.key();
        let mut elems = key.split("wutag.value.");

        let ns = next_or_else!(elems, "missing namespace `user`")?;
        if ns != "user." {
            return Err(Error::InvalidTagKey(format!(
                "invalid namespace `{}`, valid namespace is `user`",
                ns
            )));
        }

        let rest = next_or_else!(elems, "missing value")?;
        let mut new_elems = rest.split('.');
        let tag_bytes = next_or_else!(new_elems, "missing tag")?;
        let value_bytes = next_or_else!(new_elems, "missing tag")?;
        let value = serde_cbor::from_slice(&base64::decode(value_bytes.as_bytes())?)?;

        if tag_bytes == tag.as_ref() {
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
        let key = xattr.key();
        let mut elems = key.split("wutag.value.");

        let ns = next_or_else!(elems, "missing namespace `user`")?;
        if ns != "user." {
            return Err(Error::InvalidTagKey(format!(
                "invalid namespace `{}`, valid namespace is `user`",
                ns
            )));
        }

        let rest = next_or_else!(elems, "missing value")?;
        let mut new_elems = rest.split('.');
        let _tag_bytes = next_or_else!(new_elems, "missing tag")?;
        let value_bytes = next_or_else!(new_elems, "missing tag")?;

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
pub(crate) fn get_value<P, T>(path: P, tag: T, value: T) -> XResult<Value>
where
    P: AsRef<Path>,
    T: AsRef<str>,
{
    let path = path.as_ref();
    let tag = tag.as_ref();
    let value = value.as_ref();
    for tag_ in list_xattrs(path)?.into_iter().flat_map(Tag::try_from) {
        if tag_.name() == tag {
            for value_ in &list_values(path, &tag_)? {
                if value_.name() == value {
                    return Ok(value_.clone());
                }
            }
            // TODO: Is this a good idea here?
            return Err(Error::ValueNotFound(value.to_owned()));
        }
    }

    Err(Error::TagNotFound(tag.to_owned()))
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
            .filter(|xattr| xattr.key().starts_with(WUTAG_VALUE_NAMESPACE))
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
            .filter(|xattr| xattr.key().starts_with(WUTAG_VALUE_NAMESPACE))
            .map(|value| Value::parse_xattr(&value, tag));

        for value in it.flatten() {
            values.push(value);
        }
        values
    })
}

/// Clears all [`Values`] of the file at the given `path`.
///
/// # Errors
/// If the action of clearing the tags failed
#[inline]
pub(crate) fn clear_values<P>(path: P) -> XResult<()>
where
    P: AsRef<Path>,
{
    for xattr in list_xattrs(path.as_ref())?
        .iter()
        .filter(|xattr| xattr.key().starts_with(WUTAG_VALUE_NAMESPACE))
    {
        remove_xattr(path.as_ref(), xattr.key())?;
    }

    Ok(())
}

/// Checks whether the given `path` has [`Values`] for a [`Tag`]
///
/// # Errors
/// If the file entry does not have any tags
#[inline]
pub(crate) fn has_values<P>(path: P, tag: &Tag) -> XResult<bool>
where
    P: AsRef<Path>,
{
    list_values(path, tag).map(|values| !values.is_empty())
}

/// Checks whether the given `path` has any tags.
///
/// # Errors
/// If the file entry does not have any tags
#[inline]
pub(crate) fn has_any_values<P>(path: P) -> XResult<bool>
where
    P: AsRef<Path>,
{
    list_all_values(path).map(|values| !values.is_empty())
}
