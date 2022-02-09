//! Struct that represents the relationship between [`File`]s, [`Tag`]s, and
//! [`Value`]s

use super::{
    file::{File, FileId, FileIds},
    from_vec, impl_vec,
    tag::{Tag, TagId, TagValueCombo},
    value::{Value, ValueId},
};
use anyhow::{Context, Result};

use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};

// ====================== FileTag =====================

/// Relation between [`File`], [`Tag`], and [`Value`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileTag {
    /// ID of the [`File`]
    file_id:  FileId,
    /// ID of the [`Tag`]
    tag_id:   TagId,
    /// ID of the [`Value`]
    value_id: ValueId,

    /// Is it explicitly tagged?
    explicit: bool,
    /// Is it implicitly tagged?
    implicit: bool,
}

impl FileTag {
    /// Create a new [`FileTag`]
    pub(crate) const fn new(fid: FileId, tid: TagId, vid: ValueId) -> Self {
        Self {
            file_id:  fid,
            tag_id:   tid,
            value_id: vid,
            explicit: true,
            implicit: false,
        }
    }

    /// Create a new [`FileTag`] with `explicit` and `implicit` specified
    pub(crate) const fn new_full(
        fid: FileId,
        tid: TagId,
        vid: ValueId,
        exp: bool,
        imp: bool,
    ) -> Self {
        Self {
            file_id:  fid,
            tag_id:   tid,
            value_id: vid,
            explicit: exp,
            implicit: imp,
        }
    }

    /// Return the [`FileId`]
    pub(crate) const fn file_id(&self) -> FileId {
        self.file_id
    }

    /// Return the [`TagId`]
    pub(crate) const fn tag_id(&self) -> TagId {
        self.tag_id
    }

    /// Return the [`ValueId`]
    pub(crate) const fn value_id(&self) -> ValueId {
        self.value_id
    }

    /// Convert a [`FileTag`] to a [`TagValueCombo`]
    pub(crate) const fn to_tag_value_combo(self) -> TagValueCombo {
        TagValueCombo::new(self.tag_id, self.file_id)
    }
}

impl TryFrom<&Row<'_>> for FileTag {
    type Error = rsq::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            file_id:  row.get("file_id")?,
            tag_id:   row.get("tag_id")?,
            value_id: row.get("value_id")?,
            explicit: true,
            implicit: false,
        })
    }
}

// ===================== FileTags =====================

/// A vector of [`FileTag`]s
#[derive(Debug, Clone)]
pub(crate) struct FileTags {
    /// The inner vector of [`FileTag`]s
    inner: Vec<FileTag>,
}

from_vec!(FileTag, FileTags);

impl FileTags {
    impl_vec!(FileTag);

    /// Return the [`FileId`] of each [`FileTag`]
    pub(crate) fn file_ids(&self) -> FileIds {
        self.iter()
            .map(|ft| ft.file_id())
            .collect::<Vec<_>>()
            .into()
    }

    /// Return the first [`FileTag`]
    pub(crate) fn first(&self) -> Result<FileTag> {
        self.get(0)
            .copied()
            .context("failed to get first item in `FileTags`")
    }
}
