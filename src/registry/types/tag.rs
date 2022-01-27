//! A wrapper around the [`Tag`] found in [`wutag_core`]

// TODO: Look into merging wutag_core Tag and this Tag
// When writing metadata to file, possibly write tag values as well

use super::{filetag::FileTag, from_vec, impl_vec, implication::Implication, value::ValueId, ID};
use anyhow::{Context, Result};
use colored::{Color, Colorize};
use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};
use serde::{Deserialize, Serialize};
use wutag_core::{color::parse_color, tag::Tag as WTag};

// ======================= TagId ======================

/// Alias to [`ID`](super::ID)
pub(crate) type TagId = ID;

/// A vector of [`TagId`]s
#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct TagIds {
    inner: Vec<TagId>,
}

from_vec!(TagId, TagIds);

impl TagIds {
    impl_vec!(TagId);

    /// Get unique [`TagIds`]
    pub(crate) fn unique(&mut self) {
        self.inner.sort_unstable();
        self.inner.dedup();
    }
}

// ======================== Tag =======================

/// A representation of a `Tag` found within the database
///
/// The [`Tag`](wutag_core::tag::Tag) in [`wutag_core`] is what is written to a
/// file's extended attributes
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Tag {
    /// The unique identifier
    id:    TagId,
    /// The string representation
    name:  String,
    /// The color (if any) the [`Tag`] is supposed to display
    color: Color,
}

impl Tag {
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
    pub(crate) fn new<S: AsRef<str>>(id: TagId, name: S, color: S) -> Self {
        Self {
            id,
            name: name.as_ref().to_owned(),
            color: parse_color(color).unwrap_or(Color::BrightWhite),
        }
    }

    /// Set the [`Tag`] id
    pub(crate) fn set_id(&mut self, id: TagId) {
        self.id = id;
    }
}

impl TryFrom<&Row<'_>> for Tag {
    type Error = rsq::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id:    row.get("id")?,
            name:  row.get("name")?,
            color: parse_color(row.get::<_, String>("color")?)
                .map_err(|e| rsq::Error::InvalidColumnName(e.to_string()))?,
        })
    }
}

// impl ToSql for Tag {
//     fn to_sql(&self) -> rsq::Result<ToSqlOutput> {
//         let string = serde_json::to_string(self)
//             .map_err(|e| rsq::Error::ToSqlConversionFailure(Box::new(e)))?;
//         Ok(ToSqlOutput::from(string))
//     }
// }
//
// // TODO: May error out on ID, or on Color
//
// #[allow(clippy::wildcard_enum_match_arm)]
// impl FromSql for Tag {
//     fn column_result(value: ValueRef) -> FromSqlResult<Self> {
//         match value {
//             ValueRef::Text(d) | ValueRef::Blob(d) =>
// serde_json::from_slice(d),             _s => {
//                 // let val = s.as_i64();
//                 return Err(FromSqlError::InvalidType);
//             },
//         }
//         .map_err(|err| FromSqlError::Other(Box::new(err)))
//     }
// }

impl From<WTag> for Tag {
    fn from(w: WTag) -> Self {
        Self {
            id:    ID::null(),
            name:  w.name().to_owned(),
            color: *w.color(),
        }
    }
}

// ======================= Tags =======================

/// A vector of [`Tag`]s
#[derive(Debug, Clone)]
pub(crate) struct Tags {
    /// The inner vector of [`Tag`]s
    inner: Vec<Tag>,
}

from_vec!(Tag, Tags);

impl Tags {
    impl_vec!(Tag);

    /// Shorthand to the Rust builtin `any`
    pub(crate) fn any<F>(&self, f: F) -> bool
    where
        F: Fn(&Tag) -> bool,
    {
        self.inner.iter().any(f)
    }

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

// ==================== TagFileCnt ====================

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

// TODO: Maybe getting column by index would be better

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

// ================== TagValueCombo ===================

/// A pair made of a [`TagId`] and [`ValueId`]
#[derive(Debug, Clone)]
pub(crate) struct TagValueCombo {
    /// ID of the [`Tag`]
    tag_id:   TagId,
    /// ID of the [`Value`]
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

impl From<FileTag> for TagValueCombo {
    fn from(f: FileTag) -> Self {
        Self {
            tag_id:   f.tag_id(),
            value_id: f.value_id(),
        }
    }
}

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
