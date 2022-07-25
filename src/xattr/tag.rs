//! Functions for manipulating tags on files.

use super::{
    core::{list_xattrs, remove_xattr, set_xattr, Xattr},
    value::{clear_values, get_value, has_any_values, has_values, list_all_values, list_values},
    Error, Result as XResult,
};
use crate::{
    consts::WUTAG_NAMESPACE,
    g,
    registry::types::{Tag, Value},
};
use colored::Colorize;
use itertools::Itertools;
use std::{
    convert::TryFrom,
    path::{Display, Path, PathBuf},
};

/// Extend a file-path's ability to interact with `xattrs`
pub(crate) trait DirEntryExt {
    // ╭──────╮
    // │ Tags │
    // ╰──────╯
    /// Add a [`Tag`] and an optional [`Value`] to a given path
    ///
    /// # Errors
    /// If the `xattr` cannot be added
    fn tag(&self, tag: &Tag, value: Option<&Value>) -> XResult<()>;
    /// Remove a [`Tag`] from a given path and add a new one
    ///
    /// # Errors
    /// If the tag doesn't exist
    /// If the `xattr` cannot be removed
    /// If the new `xattr` cannot be set
    fn untag(&self, tag: &Tag) -> XResult<()>;
    /// Update a [`Tag`] for a given path
    ///
    /// # Errors
    /// If the tag doesn't exist or the `xattr` cannot be removed
    fn update_tag(&self, tag: &Tag, value: Option<&Value>) -> XResult<()>;
    /// Replace a [`Tag`] with another [`Tag`]
    ///
    /// # Errors
    /// If the tag doesn't exist or the `xattr` cannot be removed
    fn replace_tag(&self, replaced: &Tag, replacer: &Tag, value: Option<&Value>) -> XResult<()>;
    /// Retrieve a [`Tag`] from a given path
    ///
    /// # Errors
    /// If there are no tags on the file entry
    fn get_tag<T>(&self, tag: T) -> XResult<Tag>
    where
        T: AsRef<str>;
    /// List the [`Tag`](s) on a given path as a [`Vec`]
    ///
    /// # Errors
    /// If there are no tags or if the collection into a [`Vec`] fails
    fn list_tags(&self) -> XResult<Vec<Tag>>;
    /// Remove all [`Tag`](s) on a given path
    ///
    /// # Errors
    /// If clearing the extended attributes failed
    fn clear_tags(&self) -> XResult<()>;
    /// Check whether a given path has any [`Tag`](s)
    ///
    /// # Errors
    /// If the file entry does not have any tags
    fn has_tags(&self) -> XResult<bool>;

    // ╭────────╮
    // │ Values │
    // ╰────────╯
    // /// Add a [`Value`] to an already existing [`Tag`]
    // ///
    // /// # Errors
    // /// If the [`Tag`] doesn't exist
    // fn add_value(&self, tag: &Tag, value: &Value) -> XResult<()>;
    /// Remove a [`Value`] from a given path
    ///
    /// # Errors
    /// If the `value` cannot be found on the file
    fn unvalue(&self, tag: &Tag, value: &Value) -> XResult<()>;
    /// Remove all instances of [`Value`] from a given path
    ///
    /// # Errors
    /// If the `value` cannot be found on the file
    fn unvalue_all(&self, value: &Value) -> XResult<()>;
    /// Replace a [`Value`] with another [`Value`] for a given [`Tag`]
    ///
    /// # Errors
    /// If the tag or value doesn't exist
    fn update_value(&self, tag: &Tag, replaced: &Value, replacer: &Value) -> XResult<()>;
    /// Retrieve a [`Value`] from a given path
    /// This function is to be used as confirmation.
    ///
    /// # Errors
    /// If there are no tags on the file entry
    fn get_value<T, V>(&self, tag: T, value: V) -> XResult<Value>
    where
        T: AsRef<str>,
        V: AsRef<str>;
    /// List the [`Values`](s) that match a [`Tag`] on a given path
    ///
    /// # Errors
    /// * If there are no tags or values
    /// * If the collection into a [`Vec`] fails
    fn list_values(&self, tag: &Tag) -> XResult<Vec<Value>>;
    /// List all the [`Values`](s) on a given path
    ///
    /// # Errors
    /// * If there are no tags or values
    /// * If the collection into a [`Vec`] fails
    fn list_all_values(&self) -> XResult<Vec<Value>>;
    /// Remove all [`Value`](s) from a given path
    ///
    /// # Errors
    /// If clearing the extended attributes failed
    fn clear_values(&self) -> XResult<()>;
    /// Check whether the [`Tag`] on path has any [`Value`](s)
    ///
    /// # Errors
    /// If the file entry does not have any tags
    fn has_values(&self, tag: &Tag) -> XResult<bool>;
    /// Check whether the path has any [`Value`](s)
    ///
    /// # Errors
    /// If the file entry does not have any tags
    fn has_any_values(&self) -> XResult<bool>;

    // ╭───────╮
    // │ Extra │
    // ╰───────╯
    /// Check whether a given path has any [`Tag`](s)
    ///
    /// # Errors
    /// If the file entry does not have any tags
    fn has_tags_or_values(&self) -> XResult<bool>;

    /// Nothing more than a helper function.
    /// Assist in writing generics
    fn path(&self) -> &Path;
    /// Nothing more than a helper function.
    // Assist in writing generics
    fn display(&self) -> Display;
}

impl DirEntryExt for &PathBuf {
    // ╭─────╮
    // │ Tag │
    // ╰─────╯
    #[inline]
    fn tag(&self, tag: &Tag, value: Option<&Value>) -> XResult<()> {
        tag.save_to(self, value)
    }

    #[inline]
    fn untag(&self, tag: &Tag) -> XResult<()> {
        tag.remove_from(self)
    }

    #[inline]
    fn update_tag(&self, tag: &Tag, value: Option<&Value>) -> XResult<()> {
        tag.remove_from(self)?;
        tag.save_to(self, value)
    }

    #[inline]
    fn replace_tag(&self, replaced: &Tag, replacer: &Tag, value: Option<&Value>) -> XResult<()> {
        replaced.remove_from(self)?;
        replacer.save_to(self, value)
    }

    #[inline]
    fn get_tag<T: AsRef<str>>(&self, tag: T) -> XResult<Tag> {
        get_tag(self, tag)
    }

    #[inline]
    fn list_tags(&self) -> XResult<Vec<Tag>> {
        list_tags(self)
    }

    #[inline]
    fn clear_tags(&self) -> XResult<()> {
        clear_tags(self)
    }

    #[inline]
    fn has_tags(&self) -> XResult<bool> {
        has_tags(self)
    }

    // ╭───────╮
    // │ Value │
    // ╰───────╯
    #[inline]
    fn unvalue(&self, tag: &Tag, value: &Value) -> XResult<()> {
        value.remove_from(self, tag)
    }

    #[inline]
    fn unvalue_all(&self, value: &Value) -> XResult<()> {
        value.remove_all_from(self)
    }

    #[inline]
    fn update_value(&self, tag: &Tag, replaced: &Value, replacer: &Value) -> XResult<()> {
        replaced.update(self, tag, replacer)
    }

    #[inline]
    fn get_value<T, V>(&self, tag: T, value: V) -> XResult<Value>
    where
        T: AsRef<str>,
        V: AsRef<str>,
    {
        get_value(self, tag, value)
    }

    #[inline]
    fn list_values(&self, tag: &Tag) -> XResult<Vec<Value>> {
        list_values(self, tag)
    }

    #[inline]
    fn list_all_values(&self) -> XResult<Vec<Value>> {
        list_all_values(self)
    }

    #[inline]
    fn clear_values(&self) -> XResult<()> {
        clear_values(self)
    }

    #[inline]
    fn has_values(&self, tag: &Tag) -> XResult<bool> {
        has_values(self, tag)
    }

    #[inline]
    fn has_any_values(&self) -> XResult<bool> {
        has_any_values(self)
    }

    // ╭───────╮
    // │ Other │
    // ╰───────╯
    #[inline]
    fn has_tags_or_values(&self) -> XResult<bool> {
        has_tags_or_values(self)
    }

    #[inline]
    fn path(&self) -> &Path {
        self.as_path()
    }

    #[inline]
    fn display(&self) -> Display {
        self.path().display()
    }
}

impl DirEntryExt for ignore::DirEntry {
    // ╭─────╮
    // │ Tag │
    // ╰─────╯
    #[inline]
    fn tag(&self, tag: &Tag, value: Option<&Value>) -> XResult<()> {
        tag.save_to(self.path(), value)
    }

    #[inline]
    fn untag(&self, tag: &Tag) -> XResult<()> {
        tag.remove_from(self.path())
    }

    #[inline]
    fn update_tag(&self, tag: &Tag, value: Option<&Value>) -> XResult<()> {
        tag.remove_from(self.path())?;
        tag.save_to(self.path(), value)
    }

    #[inline]
    fn replace_tag(&self, replaced: &Tag, replacer: &Tag, value: Option<&Value>) -> XResult<()> {
        replaced.remove_from(self.path())?;
        replacer.save_to(self.path(), value)
    }

    #[inline]
    fn get_tag<T: AsRef<str>>(&self, tag: T) -> XResult<Tag> {
        get_tag(self.path(), tag)
    }

    #[inline]
    fn list_tags(&self) -> XResult<Vec<Tag>> {
        list_tags(self.path())
    }

    #[inline]
    fn clear_tags(&self) -> XResult<()> {
        clear_tags(self.path())
    }

    #[inline]
    fn has_tags(&self) -> XResult<bool> {
        has_tags(self.path())
    }

    // ╭───────╮
    // │ Value │
    // ╰───────╯
    #[inline]
    fn unvalue(&self, tag: &Tag, value: &Value) -> XResult<()> {
        value.remove_from(self.path(), tag)
    }

    #[inline]
    fn unvalue_all(&self, value: &Value) -> XResult<()> {
        value.remove_all_from(self.path())
    }

    #[inline]
    fn update_value(&self, tag: &Tag, replaced: &Value, replacer: &Value) -> XResult<()> {
        replaced.update(self.path(), tag, replacer)
    }

    #[inline]
    fn get_value<T, V>(&self, tag: T, value: V) -> XResult<Value>
    where
        T: AsRef<str>,
        V: AsRef<str>,
    {
        get_value(self.path(), tag, value)
    }

    #[inline]
    fn list_values(&self, tag: &Tag) -> XResult<Vec<Value>> {
        list_values(self.path(), tag)
    }

    #[inline]
    fn list_all_values(&self) -> XResult<Vec<Value>> {
        list_all_values(self.path())
    }

    #[inline]
    fn clear_values(&self) -> XResult<()> {
        clear_values(self.path())
    }

    #[inline]
    fn has_values(&self, tag: &Tag) -> XResult<bool> {
        has_values(self.path(), tag)
    }

    #[inline]
    fn has_any_values(&self) -> XResult<bool> {
        has_any_values(self.path())
    }

    // ╭───────╮
    // │ Other │
    // ╰───────╯
    #[inline]
    fn has_tags_or_values(&self) -> XResult<bool> {
        has_tags_or_values(self.path())
    }

    #[inline]
    fn path(&self) -> &Path {
        self.path()
    }

    #[inline]
    fn display(&self) -> Display {
        self.path().display()
    }
}

impl Tag {
    /// Custom implementation of `Hash`
    #[allow(clippy::same_name_method)]
    pub(super) fn hash(&self) -> XResult<String> {
        serde_cbor::to_vec(&self)
            .map(|tag| format!("{}.{}", WUTAG_NAMESPACE, base64::encode(tag)))
            .map_err(Error::from)
    }

    /// Tags the file at the given `path`.
    ///
    /// # Errors
    /// If the tag exists it returns an [`Error`]
    pub(crate) fn save_to<P>(&self, path: P, value: Option<&Value>) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        log::debug!(
            "XATTR: {}: saving Tag({}), Value({})",
            path.display(),
            self.name(),
            value.map_or("None", |v| v.name())
        );

        println!(
            "\n++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
XATTR: {}:
Tag({})
Value({})
++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++",
            path.display(),
            self.name(),
            value.map_or("None", |v| v.name())
        );

        let mut exists = false;

        for tag in list_tags(&path)? {
            println!("iterTag: {:#?}", tag);
            // Return an error if it is the same tag and no value
            // If value, user probably wants to set more than one value on same tag
            if &tag == self {
                println!("{} == {}", tag.name(), self.name());
                if value.is_none() {
                    println!("ERROR: TagExists");
                    return Err(Error::TagExists(g!((tag.name()))));
                }

                exists = true;
            }
        }

        let mut values = vec![];

        // Check whether the value already exists on this Tag on this file
        if let Some(v) = value {
            println!("There is some value");
            for val in list_values(path, self)? {
                println!("iterValue: {:?}", val);
                if v == &val {
                    println!("ERROR: ValueExists: {} == {}", v.name(), val.name());
                    return Err(Error::ValueExists(g!((val.name())), g!((self.name()))));
                }

                values.push(val);
            }
        }

        // If the tag exists, rewrite it
        if exists {
            println!("EXISTS == TRUE");
            // self.remove_from(path)?;
        }

        println!("VALUES: {:#?}", values);

        let vals = values
            .iter()
            .map(|val| Value::hash(val).unwrap_or_else(|_| "".to_owned()))
            .join(".");

        println!("VALUES JOINED: {}", vals);

        // let val = value
        //     .map(Value::hash)
        //     .transpose()?
        //     .unwrap_or_else(|| "".to_owned());

        set_xattr(path, self.hash()?.as_str(), &vals)
    }

    /// Removes this tag from the file at the given `path`.
    ///
    /// # Errors
    /// If the tag doesn't exist the error [`TagNotFound`] is returned
    ///
    /// [`TagNotFound`]: crate::xattr::Error::TagNotFound
    pub(crate) fn remove_from<P>(&self, path: P) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        log::debug!("XATTR: {}: removing Tag({})", path.display(), self.name());
        let hash = self.hash()?;

        for xattr in list_xattrs(path)? {
            let key = xattr.key();
            // make sure to only remove attributes corresponding to this namespace
            if key == hash {
                return remove_xattr(path, key);
            }
        }

        Err(Error::TagNotFound(self.name().clone()))
    }

    #[allow(dead_code)]
    pub(crate) fn remove_value_from<P>(&self, path: P) -> XResult<()>
    where
        P: AsRef<Path>,
    {
        let hash = self.hash()?;

        for xattr in list_xattrs(path.as_ref())? {
            let key = xattr.key();
            // make sure to only remove attributes corresponding to this namespace
            if key == hash {
                return remove_xattr(path, key);
            }
        }

        Err(Error::TagNotFound(self.name().clone()))
    }
}

/// Get the `next` item or return an `Error`
macro_rules! next_or_else {
    ($it:ident, $msg:expr) => {
        $it.next()
            .ok_or_else(|| Error::InvalidTagKey($msg.to_owned()))
    };
}

impl TryFrom<Xattr> for Tag {
    type Error = Error;

    #[inline]
    fn try_from(xattr: Xattr) -> XResult<Self> {
        let key = xattr.key();

        let mut elems = key.split("wutag.");

        // TODO: This needs to be checked for symlink
        let ns = next_or_else!(elems, "missing namespace `user`")?;
        if ns != "user." {
            return Err(Error::InvalidTagKey(format!(
                "invalid namespace `{}`, valid namespace is `user`",
                ns
            )));
        }

        let tag_bytes = next_or_else!(elems, "missing tag")?;
        let tag = serde_cbor::from_slice(&base64::decode(tag_bytes.as_bytes())?)?;

        Ok(tag)
    }
}

/// Return a [`Tag`] given a `path` and a tag as a `str`
///
/// # Errors
/// If the tag doesn't exist it returns [`TagNotFound`]
///
/// [`TagNotFound`]: crate::Error::TagNotFound
#[inline]
pub(crate) fn get_tag<P, T>(path: P, tag: T) -> XResult<Tag>
where
    P: AsRef<Path>,
    T: AsRef<str>,
{
    let path = path.as_ref();
    let tag = tag.as_ref();
    log::debug!("XATTR: {}: getting Tag({})", path.display(), tag);

    for tag_ in list_xattrs(path)?.into_iter().flat_map(Tag::try_from) {
        if tag_.name() == tag {
            return Ok(tag_);
        }
    }

    Err(Error::TagNotFound(tag.to_owned()))
}

/// Lists tags of the file at the given `path`.
///
/// # Errors
/// If there are no tags or if the collection into a [`Vec`] fails
#[inline]
pub(crate) fn list_tags<P>(path: P) -> XResult<Vec<Tag>>
where
    P: AsRef<Path>,
{
    list_xattrs(path).map(|attrs| {
        let mut tags = Vec::new();
        let it = attrs
            .into_iter()
            .filter(|xattr| {
                log::trace!("XATTR: {:#?}", xattr);

                println!("xattrTag: {:#?}", xattr);
                xattr.key().starts_with(WUTAG_NAMESPACE)
            })
            .map(Tag::try_from);

        for tag in it.flatten() {
            tags.push(tag);
        }
        tags
    })
}

/// Clears all tags (and values) of the file at the given `path`.
///
/// # Errors
/// If clearing the extended attributes failed
#[inline]
pub(crate) fn clear_tags<P>(path: P) -> XResult<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    log::debug!("XATTR: {}: clearing Tags", path.display());

    for xattr in list_xattrs(path)?
        .iter()
        .filter(|xattr| xattr.key().starts_with(WUTAG_NAMESPACE))
    {
        remove_xattr(path, xattr.key())?;
    }

    clear_values(path)?;

    Ok(())
}

/// Checks whether the given `path` has any tags.
///
/// # Errors
/// If the file entry does not have any tags
#[inline]
pub(crate) fn has_tags<P>(path: P) -> XResult<bool>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    list_tags(path).map(|tags| {
        let has = !tags.is_empty();
        log::debug!(
            "XATTR: {}: {} have Tags",
            path.display(),
            tern::t!(has ? "does" : "doesn't")
        );
        has
    })
}

/// Checks whether the given `path` has any tags or values
///
/// # Errors
/// If there is a failure in listing extended attributes
#[inline]
pub(crate) fn has_tags_or_values<P>(path: P) -> XResult<bool>
where
    P: AsRef<Path>,
{
    let has = has_tags(&path)? || has_any_values(&path)?;
    log::debug!(
        "XATTR: {}: {} have Tags or Values",
        path.as_ref().display(),
        tern::t!(has ? "does" : "doesn't")
    );
    Ok(has)
}
