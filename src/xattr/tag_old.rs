//! Functions for manipulating tags on files.
use colored::{Color, Colorize};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    convert::TryFrom,
    fmt,
    hash::{Hash, Hasher},
    path::Path,
};

use super::{
    core::{list_xattrs, remove_xattr, set_xattr, Xattr},
    Error, Result,
};
use crate::consts::WUTAG_NAMESPACE;

/// Default [`Color`] to use
pub(crate) const DEFAULT_COLOR: Color = Color::BrightWhite;

/// A representation of a `Tag`
/// Used to set `xattr` on files
#[derive(Clone, Debug, Deserialize, Eq, Serialize)]
pub(crate) struct Tag {
    /// The name of the tag
    pub(crate) name:  String,
    /// The color of the tag
    pub(crate) color: Color,
}

impl Hash for Tag {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.color.to_fg_str().hash(state);
    }
}

/// Extend a directory entry's ability to interact with `xattrs`
pub(crate) trait DirEntryExt {
    /// Add a [`Tag`] to a directory entry
    ///
    /// # Errors
    /// If the the `xattr` cannot be added
    fn tag(&self, tag: &Tag) -> Result<()>;
    /// Remove a [`Tag`] to a directory entry
    ///
    /// # Errors
    /// If the tag doesn't exist or the `xattr` cannot be removed
    fn untag(&self, tag: &Tag) -> Result<()>;
    /// Retrieve a [`Tag`] to a directory entry
    ///
    /// # Errors
    /// If there are no tags on the directory entry
    fn get_tag<T: AsRef<str>>(&self, tag: T) -> Result<Tag>;
    /// List the [`Tag`](s) on a directory entry as a [`Vec`]
    ///
    /// # Errors
    /// If there are no tags or if the collection into a [`Vec`] fails
    fn list_tags(&self) -> Result<Vec<Tag>>;
    /// List the [`Tag`](s) on a directory entry as a [`BTreeSet`]
    ///
    /// # Errors
    /// If there are no tags or if the collection into a [`BTreeSet`] fails
    fn list_tags_btree(&self) -> Result<BTreeSet<Tag>>;
    /// Remove all [`Tag`](s) on a directory entry
    ///
    /// # Errors
    /// If the action of clearing the tags failed
    fn clear_tags(&self) -> Result<()>;
    /// Check wheter a directory entry has any [`Tag`](s)
    ///
    /// # Errors
    /// If the directory entry does not have any tags
    fn has_tags(&self) -> Result<bool>;
}

impl DirEntryExt for &std::path::PathBuf {
    #[inline]
    fn tag(&self, tag: &Tag) -> Result<()> {
        tag.save_to(self)
    }

    #[inline]
    fn untag(&self, tag: &Tag) -> Result<()> {
        tag.remove_from(self)
    }

    #[inline]
    fn get_tag<T: AsRef<str>>(&self, tag: T) -> Result<Tag> {
        get_tag(self, tag)
    }

    #[inline]
    fn list_tags(&self) -> Result<Vec<Tag>> {
        list_tags(self)
    }

    #[inline]
    fn list_tags_btree(&self) -> Result<BTreeSet<Tag>> {
        list_tags_btree(self)
    }

    #[inline]
    fn clear_tags(&self) -> Result<()> {
        clear_tags(self)
    }

    #[inline]
    fn has_tags(&self) -> Result<bool> {
        has_tags(self)
    }
}

impl DirEntryExt for ignore::DirEntry {
    #[inline]
    fn tag(&self, tag: &Tag) -> Result<()> {
        tag.save_to(self.path())
    }

    #[inline]
    fn untag(&self, tag: &Tag) -> Result<()> {
        tag.remove_from(self.path())
    }

    #[inline]
    fn get_tag<T: AsRef<str>>(&self, tag: T) -> Result<Tag> {
        get_tag(self.path(), tag)
    }

    #[inline]
    fn list_tags(&self) -> Result<Vec<Tag>> {
        list_tags(self.path())
    }

    #[inline]
    fn list_tags_btree(&self) -> Result<BTreeSet<Tag>> {
        list_tags_btree(self.path())
    }

    #[inline]
    fn clear_tags(&self) -> Result<()> {
        clear_tags(self.path())
    }

    #[inline]
    fn has_tags(&self) -> Result<bool> {
        has_tags(self.path())
    }
}

impl Tag {
    /// Generate a new tag with a specified color
    #[inline]
    pub(crate) fn new<S>(name: S, color: Color) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            color,
        }
    }

    /// Generate a new tag with a random color
    #[inline]
    pub(crate) fn random<S>(name: S, colors: &[Color]) -> Self
    where
        S: Into<String>,
    {
        let mut rng = thread_rng();
        Self::new(name, colors.choose(&mut rng).copied().unwrap_or(DEFAULT_COLOR))
    }

    /// Get the tag's name
    #[inline]
    #[must_use]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Get the tag's color
    #[inline]
    #[must_use]
    pub(crate) const fn color(&self) -> &Color {
        &self.color
    }

    /// Change or set the tag's color
    #[inline]
    pub(crate) fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Change or set the tag's name
    #[inline]
    pub(crate) fn set_name<T: AsRef<str>>(&mut self, name: T) {
        self.name = name.as_ref().to_owned();
    }

    /// Custom implementation of `Hash`
    #[allow(clippy::same_name_method)]
    fn hash(&self) -> Result<String> {
        serde_cbor::to_vec(&self)
            .map(|tag| format!("{}.{}", WUTAG_NAMESPACE, base64::encode(tag)))
            .map_err(Error::from)
    }

    /// Tags the file at the given `path`.
    ///
    /// # Errors
    /// If the tag exists it returns an [`Error`]
    #[inline]
    pub(crate) fn save_to<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        for tag in list_tags(path.as_ref())? {
            if &tag == self {
                return Err(Error::TagExists(tag.name.green().bold()));
            }
        }
        set_xattr(path, self.hash()?.as_str(), "")
    }

    /// Removes this tag from the file at the given `path`.
    ///
    /// # Errors
    /// If the tag doesn't exist it returns [`TagNotFound`]
    ///
    /// [`TagNotFound`]: crate::Error::TagNotFound
    #[inline]
    pub(crate) fn remove_from<P>(&self, path: P) -> Result<()>
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

        Err(Error::TagNotFound(self.name.clone()))
    }
}

impl fmt::Display for Tag {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Ord for Tag {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialEq for Tag {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PartialOrd for Tag {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
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
    fn try_from(xattr: Xattr) -> Result<Self> {
        let key = xattr.key();

        let mut elems = key.split("wutag.");

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
pub(crate) fn get_tag<P, T>(path: P, tag: T) -> Result<Tag>
where
    P: AsRef<Path>,
    T: AsRef<str>,
{
    let path = path.as_ref();
    let tag = tag.as_ref();
    for tag_ in list_xattrs(path)?.into_iter().flat_map(Tag::try_from) {
        if tag_.name == tag {
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
pub(crate) fn list_tags<P>(path: P) -> Result<Vec<Tag>>
where
    P: AsRef<Path>,
{
    list_xattrs(path).map(|attrs| {
        let mut tags = Vec::new();
        let it = attrs
            .into_iter()
            .filter(|xattr| xattr.key().starts_with(WUTAG_NAMESPACE))
            .map(Tag::try_from);

        for tag in it.flatten() {
            tags.push(tag);
        }
        tags
    })
}

/// Lists tags of the file at the given `path` as a [`BTreeSet`].
///
/// # Errors
/// If there are no tags or if the collection into a [`BTreeSet`] fails
#[inline]
pub(crate) fn list_tags_btree<P>(path: P) -> Result<BTreeSet<Tag>>
where
    P: AsRef<Path>,
{
    list_xattrs(path).map(|attrs| {
        let mut tags = BTreeSet::new();
        let it = attrs
            .into_iter()
            .filter(|xattr| xattr.key().starts_with(WUTAG_NAMESPACE))
            .map(Tag::try_from);

        for tag in it.flatten() {
            tags.insert(tag);
        }
        tags
    })
}

/// Clears all tags of the file at the given `path`.
///
/// # Errors
/// If the action of clearing the tags failed
#[inline]
pub(crate) fn clear_tags<P>(path: P) -> Result<()>
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

/// Checks whether the given `path` has any tags.
///
/// # Errors
/// If the directory entry does not have any tags
#[inline]
pub(crate) fn has_tags<P>(path: P) -> Result<bool>
where
    P: AsRef<Path>,
{
    list_tags(path).map(|tags| !tags.is_empty())
}
