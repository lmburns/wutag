//! A wrapper around the [`Tag`] found in [`wutag_core`]

// TODO: Look into merging wutag_core Tag and this Tag
// When writing metadata to file, possibly write tag values as well

use super::{
    filetag::FileTag, from_vec, impl_vec, implication::Implication, validate_name, value::ValueId,
    ID,
};
use crate::wutag_error;
use anyhow::{anyhow, Context, Result};
use colored::{Color, Colorize};
use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::BTreeSet,
    fmt,
    hash::{Hash, Hasher},
    path::{Display, Path, PathBuf},
};
use wutag_core::{
    color::{self, parse_color},
    tag::{Tag as WTag, DEFAULT_COLOR},
    xattr::{list_xattrs, remove_xattr, set_xattr, Xattr},
    Error, WUTAG_NAMESPACE,
};

// ======================= TagId ======================

/// Alias to [`ID`](super::ID)
pub(crate) type TagId = ID;

/// A vector of [`TagId`]s
#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct TagIds {
    inner: Vec<TagId>,
}

impl From<TagIds> for Vec<TagId> {
    fn from(t: TagIds) -> Self {
        t.inner
    }
}
from_vec!(TagId, TagIds);
impl_vec!(TagIds, TagId);

/// Extend a directory entry's ability to interact with `xattrs`
pub(crate) trait DirEntryExt {
    /// Add a [`Tag`] to a directory entry
    ///
    /// # Errors
    /// If the the `xattr` cannot be added
    fn tag(&self, tag: &Tag) -> Result<(), Error>;
    /// Remove a [`Tag`] to a directory entry
    ///
    /// # Errors
    /// If the tag doesn't exist or the `xattr` cannot be removed
    fn untag(&self, tag: &Tag) -> Result<(), Error>;
    /// Retrieve a [`Tag`] to a directory entry
    ///
    /// # Errors
    /// If there are no tags on the directory entry
    fn get_tag<T: AsRef<str>>(&self, tag: T) -> Result<Tag, Error>;
    /// List the [`Tag`](s) on a directory entry as a [`Vec`]
    ///
    /// # Errors
    /// If there are no tags or if the collection into a [`Vec`] fails
    fn list_tags(&self) -> Result<Vec<Tag>, Error>;
    /// List the [`Tag`](s) on a directory entry as a [`BTreeSet`]
    ///
    /// # Errors
    /// If there are no tags or if the collection into a [`BTreeSet`] fails
    fn list_tags_btree(&self) -> Result<BTreeSet<Tag>, Error>;
    /// Remove all [`Tag`](s) on a directory entry
    ///
    /// # Errors
    /// If the action of clearing the tags failed
    fn clear_tags(&self) -> Result<(), Error>;
    /// Check wheter a directory entry has any [`Tag`](s)
    ///
    /// # Errors
    /// If the directory entry does not have any tags
    fn has_tags(&self) -> Result<bool, Error>;

    /// Nothing more than a helper function for this trait to assist in writing
    /// generics
    fn path(&self) -> &Path;
    /// Nothing more than a helper function for this trait to assist in writing
    /// generics
    fn display(&self) -> Display;
}

impl DirEntryExt for &PathBuf {
    #[inline]
    fn tag(&self, tag: &Tag) -> Result<(), Error> {
        tag.save_to(self)
    }

    #[inline]
    fn untag(&self, tag: &Tag) -> Result<(), Error> {
        tag.remove_from(self)
    }

    #[inline]
    fn get_tag<T: AsRef<str>>(&self, tag: T) -> Result<Tag, Error> {
        get_tag(self, tag)
    }

    #[inline]
    fn list_tags(&self) -> Result<Vec<Tag>, Error> {
        list_tags(self)
    }

    #[inline]
    fn list_tags_btree(&self) -> Result<BTreeSet<Tag>, Error> {
        list_tags_btree(self)
    }

    #[inline]
    fn clear_tags(&self) -> Result<(), Error> {
        clear_tags(self)
    }

    #[inline]
    fn has_tags(&self) -> Result<bool, Error> {
        has_tags(self)
    }

    #[inline]
    fn path(&self) -> &Path {
        self.as_path()
    }

    #[inline]
    fn display(&self) -> Display {
        // How to use fully qualified syntax here to prevent recursion?
        // <&PathBuf>::display(self)
        self.path().display()
    }
}

impl DirEntryExt for ignore::DirEntry {
    #[inline]
    fn tag(&self, tag: &Tag) -> Result<(), Error> {
        tag.save_to(self.path())
    }

    #[inline]
    fn untag(&self, tag: &Tag) -> Result<(), Error> {
        tag.remove_from(self.path())
    }

    #[inline]
    fn get_tag<T: AsRef<str>>(&self, tag: T) -> Result<Tag, Error> {
        get_tag(self.path(), tag)
    }

    #[inline]
    fn list_tags(&self) -> Result<Vec<Tag>, Error> {
        list_tags(self.path())
    }

    #[inline]
    fn list_tags_btree(&self) -> Result<BTreeSet<Tag>, Error> {
        list_tags_btree(self.path())
    }

    #[inline]
    fn clear_tags(&self) -> Result<(), Error> {
        clear_tags(self.path())
    }

    #[inline]
    fn has_tags(&self) -> Result<bool, Error> {
        has_tags(self.path())
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

// ╭──────────────────────────────────────────────────────────╮
// │                           Tag                            │
// ╰──────────────────────────────────────────────────────────╯

/// A representation of a `Tag` found within the database
///
/// The [`Tag`](wutag_core::tag::Tag) in [`wutag_core`] is what is written to a
/// file's extended attributes
#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub(crate) struct Tag {
    /// The unique identifier
    id:    TagId,
    /// The string representation
    name:  String,
    /// The color (if any) the [`Tag`] is supposed to display
    color: Color,
}

impl Tag {
    validate_name!("tag name", "tag names");

    /// Return the [`TagId`]
    pub(crate) const fn id(&self) -> TagId {
        self.id
    }

    /// Return the [`Tag`] name
    pub(crate) const fn name(&self) -> &String {
        &self.name
    }

    /// Return the [`Color`]
    pub(crate) const fn color(&self) -> Color {
        self.color
    }

    /// Create a new [`Tag`]
    pub(crate) fn new<S: AsRef<str>>(id: TagId, name: S, color: Color) -> Self {
        Self {
            id,
            name: name.as_ref().to_owned(),
            color,
        }
    }

    /// Choose a random [`Color`]
    pub(crate) fn random_color(colors: &[Color]) -> Color {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        colors.choose(&mut rng).copied().unwrap_or(DEFAULT_COLOR)
    }

    /// Create a new [`Tag`] with a random [`Color`]
    pub(crate) fn random<S: AsRef<str>>(id: TagId, name: S, colors: &[Color]) -> Self {
        Self::new(id, name, Self::random_color(colors))
    }

    /// Create a new [`Tag`] with no `id`
    pub(crate) fn new_noid<S: AsRef<str>>(name: S, color: S) -> Self {
        Self {
            id:    TagId::null(),
            name:  name.as_ref().to_owned(),
            color: parse_color(color).unwrap_or_else(|e| {
                wutag_error!("{}", e);
                DEFAULT_COLOR
            }),
        }
    }

    /// Create a new [`Tag`] with a random [`Color`] and no `id`
    pub(crate) fn random_noid<S: AsRef<str>>(name: S, colors: &[Color]) -> Self {
        Self {
            id:    TagId::null(),
            name:  name.as_ref().to_owned(),
            color: Self::random_color(colors),
        }
    }

    /// Create an empty [`Tag`] to be used in a closure usually
    ///
    /// This prevents a vector of [`Result`]s and allows the developer to later
    /// check the [`Tag`]'s name
    pub(crate) fn null<M: AsRef<str>>(name: M) -> Self {
        Self {
            id:    TagId::null(),
            name:  name.as_ref().to_owned(),
            color: parse_color("white").unwrap_or(DEFAULT_COLOR),
        }
    }

    /// Check whether the [`Tag`] ID is null
    pub(crate) fn is_null_id(&self) -> bool {
        self.id == TagId::null()
    }

    /// Check whether the [`Tag`] name is null
    pub(crate) fn is_null_name(&self) -> bool {
        self.name.is_empty()
    }

    /// Check whether the [`Tag`] is null
    pub(crate) fn is_null(&self) -> bool {
        self.is_null_id() && self.is_null_name()
    }

    /// Set the [`Tag`] id
    pub(crate) fn set_id(&mut self, id: TagId) {
        self.id = id;
    }

    /// Change or set the [`Tag`] color
    pub(crate) fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Change or set the [`Tag`] name
    pub(crate) fn set_name<T: AsRef<str>>(&mut self, name: T) {
        self.name = name.as_ref().to_string();
    }

    // ╭──────────────────────────────────────────────────────────╮
    // │                   Extended Attributes                    │
    // ╰──────────────────────────────────────────────────────────╯

    /// Custom implementation of `Hash`
    #[allow(clippy::same_name_method)]
    fn hash(&self) -> Result<String, Error> {
        serde_cbor::to_vec(&self)
            .map(|tag| format!("{}.{}", WUTAG_NAMESPACE, base64::encode(tag)))
            .map_err(Error::from)
    }

    /// Tags the file at the given `path`.
    ///
    /// # Errors
    /// If the tag exists it returns an [`Error`]
    pub(crate) fn save_to<P>(&self, path: P) -> Result<(), Error>
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
    pub(crate) fn remove_from<P>(&self, path: P) -> Result<(), Error>
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

impl Hash for Tag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.color.to_fg_str().hash(state);
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

impl TryFrom<&Row<'_>> for Tag {
    type Error = rsq::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id:    row.get("id")?,
            name:  row.get("name")?,
            color: color::color_from_fg_str(&row.get::<_, String>("color")?)
                .context("color already in table is unable to be parsed; should never happen")
                .map_err(|e| rsq::Error::InvalidParameterName(e.to_string()))?,
        })
    }
}

impl From<WTag> for Tag {
    fn from(w: WTag) -> Self {
        Self {
            id:    ID::null(),
            name:  w.name().to_owned(),
            color: *w.color(),
        }
    }
}

impl From<&WTag> for Tag {
    fn from(w: &WTag) -> Self {
        Self {
            id:    ID::null(),
            name:  w.name().to_owned(),
            color: *w.color(),
        }
    }
}

impl From<Tag> for WTag {
    fn from(w: Tag) -> Self {
        Self {
            name:  w.name().clone(),
            color: w.color(),
        }
    }
}

impl From<&Tag> for WTag {
    fn from(w: &Tag) -> Self {
        Self {
            name:  w.name().clone(),
            color: w.color(),
        }
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
    fn try_from(xattr: Xattr) -> Result<Self, Error> {
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
pub(crate) fn get_tag<P, T>(path: P, tag: T) -> Result<Tag, Error>
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
pub(crate) fn list_tags<P>(path: P) -> Result<Vec<Tag>, Error>
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
pub(crate) fn list_tags_btree<P>(path: P) -> Result<BTreeSet<Tag>, Error>
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
pub(crate) fn clear_tags<P>(path: P) -> Result<(), Error>
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
pub(crate) fn has_tags<P>(path: P) -> Result<bool, Error>
where
    P: AsRef<Path>,
{
    list_tags(path).map(|tags| !tags.is_empty())
}

// ╭──────────────────────────────────────────────────────────╮
// │                           Tags                           │
// ╰──────────────────────────────────────────────────────────╯

/// A vector of [`Tag`]s
#[derive(Debug, Clone)]
pub(crate) struct Tags {
    /// The inner vector of [`Tag`]s
    inner: Vec<Tag>,
}

from_vec!(Tag, Tags);

impl Tags {
    impl_vec!(Tag);

    /// Does the inner vector contain a specific [`Tag`] by [`ID`]?
    pub(crate) fn contains(&self, other: &Tag) -> bool {
        self.any(|v| v.id() == other.id())
    }

    /// Does the inner vector contain a specific [`Value`] by name?
    pub(crate) fn contains_name<S: AsRef<str>>(&self, name: S, ignore_case: bool) -> bool {
        let name = name.as_ref();
        self.any(|v| {
            *v.name()
                == ignore_case
                    .then(|| name.to_lowercase())
                    .unwrap_or_else(|| name.to_string())
        })
    }
}

// ╭──────────────────────────────────────────────────────────╮
// │                        TagFileCnt                        │
// ╰──────────────────────────────────────────────────────────╯

// TODO: Find a way to not use this

/// Struct holding information about a [`Tag`]'s associated number of [`File`]s
#[derive(Debug, Clone)]
pub(crate) struct TagFileCnt {
    /// The [`TagId`] of a [`Tag`]
    id:    TagId,
    /// The name of the [`Tag`]
    name:  String,
    /// The number of [`File`]s the [`Tag`] is associated with
    count: usize,
}

impl TagFileCnt {
    /// Return the [`TagId`]
    pub(crate) const fn id(&self) -> TagId {
        self.id
    }

    /// Return the [`Tag`] name
    pub(crate) const fn name(&self) -> &String {
        &self.name
    }

    /// Return the [`Tag`] count
    pub(crate) const fn count(&self) -> usize {
        self.count
    }
}

impl TryFrom<&Row<'_>> for TagFileCnt {
    type Error = rsq::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id:    row.get("id")?,
            name:  row.get("name")?,
            count: row.get("cnt")?,
        })
    }
}

// ╭──────────────────────────────────────────────────────────╮
// │                      TagValueCombo                       │
// ╰──────────────────────────────────────────────────────────╯

/// A pair made of a [`TagId`] and [`ValueId`]
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct TagValueCombo {
    /// ID of the [`Tag`]
    tag_id:   TagId,
    /// ID of the [`Value`](super::value::Value)
    value_id: ValueId,
}

impl TagValueCombo {
    /// Create a new [`TagValueCombo`]
    pub(crate) const fn new(tid: TagId, vid: ValueId) -> Self {
        Self {
            tag_id:   tid,
            value_id: vid,
        }
    }

    /// Return the [`TagId`]
    pub(crate) const fn tag_id(&self) -> TagId {
        self.tag_id
    }

    /// Return the [`ValueId`]
    pub(crate) const fn value_id(&self) -> ValueId {
        self.value_id
    }
}

impl fmt::Display for TagValueCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use colored::Colorize;
        write!(
            f,
            "#{}=#{}",
            self.tag_id.to_string().green(),
            self.value_id.to_string().red()
        )
    }
}

impl From<FileTag> for TagValueCombo {
    fn from(f: FileTag) -> Self {
        Self::new(f.tag_id(), f.file_id())
    }
}

// ===================== TagValueCombos =====================

/// A vector of [`TagValueCombo`]s
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct TagValueCombos {
    /// The inner vector of [`TagValueCombo`]s
    inner: Vec<TagValueCombo>,
}

from_vec!(TagValueCombo, TagValueCombos);

impl TagValueCombos {
    impl_vec!(TagValueCombo);
}

// ╒══════════════════════════════════════════════════════════╕
//                            Tests
// ╘══════════════════════════════════════════════════════════╛

mod test {
    use super::{TagId, TagIds};

    #[test]
    fn unique_tagids() {
        let v = vec![1, 2, 5, 5, 3, 1, 7]
            .iter()
            .map(|i| TagId::new(*i))
            .collect::<Vec<_>>();
        let mut ids = TagIds::new(v);

        assert!(ids.len() == 7);

        ids.unique();
        assert!(ids.len() == 5);

        assert_eq!(ids, TagIds {
            inner: vec![1, 2, 3, 5, 7]
                .iter()
                .map(|i| TagId::new(*i))
                .collect::<Vec<_>>(),
        });
    }
}
