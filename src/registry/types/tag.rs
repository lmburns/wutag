//! A representation of the tag found within the database
//!
//! Extended attributes are also implemented for [`Tag`] and can be found in
//! [`crate::xattr::tag`]

use super::{filetag::FileTag, from_vec, impl_vec, validate_name, value::ValueId, ID};
use crate::{
    consts::{DEFAULT_COLOR, WUTAG_NAMESPACE},
    utils::color::{self, parse_color},
    wutag_error,
    xattr::{
        core::{list_xattrs, remove_xattr, set_xattr, Xattr},
        tag_old::Tag as WTag,
        Error,
    },
};
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

// ╭──────────────────────────────────────────────────────────╮
// │                          TagId                           │
// ╰──────────────────────────────────────────────────────────╯

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

// ╭──────────────────────────────────────────────────────────╮
// │                           Tag                            │
// ╰──────────────────────────────────────────────────────────╯

/// A representation of a `Tag` found within the database
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
    /// check the [`Tag`]'s name to report an error
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

    /// Change or set the [`Tag`] color with an `str`
    pub(crate) fn set_color<S: AsRef<str>>(&mut self, color: S) {
        let color = parse_color(&color).unwrap_or(Color::BrightWhite);
        self.color = color;
    }

    /// Change or set the [`Tag`] color with an alread parsed [`Color`]
    pub(crate) fn set_color_parsed(&mut self, color: Color) {
        self.color = color;
    }

    /// Change or set the [`Tag`] name
    pub(crate) fn set_name<T: AsRef<str>>(&mut self, name: T) {
        self.name = name.as_ref().to_string();
    }

    /// Create a new [`Tag`] from an existing one
    pub(crate) fn copy_into(&mut self, tag: &Self) {
        self.set_color_parsed(tag.color());
        self.set_name(tag.name());
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

impl Hash for Tag {
    // TODO: Maybe add option to serialize value
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

impl fmt::Display for Tag {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
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

// TODO: Delete these once subcommands are finished
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

// ╭──────────────────────────────────────────────────────────╮
// │                      TagValueCombos                      │
// ╰──────────────────────────────────────────────────────────╯

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
