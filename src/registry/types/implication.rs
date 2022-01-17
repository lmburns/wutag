//! If tag1 implies tag2, when tag1 is placed on a [`File`], then anywhere tag1
//! can be searched or matched, tag2 will do the same

use super::{
    from_vec,
    tag::{Tag, TagId, TagValueCombo},
    value::{Value, ValueId},
};

use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};

// =================== Implication ====================

/// Representation of one [`Tag`] implementing another [`Tag`]
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Implication {
    /// `Tag` that is doing the implying
    implying_tag: Tag,
    /// `Value` that is doing the implying
    implying_val: Value,
    /// `Tag` that is being implied
    implied_tag:  Tag,
    /// `Value` that is being implied
    implied_val:  Value,
}

impl Implication {
    // /// Create a new [`Implication`]
    // pub(crate) fn new()

    /// Return the implying [`Tag`]
    pub(crate) const fn implying_tag(&self) -> &Tag {
        &self.implying_tag
    }

    /// Return the implying [`Value`]
    pub(crate) const fn implying_val(&self) -> &Value {
        &self.implying_val
    }

    /// Return the implied [`Tag`]
    pub(crate) const fn implied_tag(&self) -> &Tag {
        &self.implied_tag
    }

    /// Return the implied [`Value`]
    pub(crate) const fn implied_val(&self) -> &Value {
        &self.implied_val
    }

    /// Implying pair of ([`Tag`], [`Value`])
    pub(crate) const fn implying_tag_value(&self) -> TagValueCombo {
        TagValueCombo::new(self.implying_tag.id(), self.implying_val.id())
    }

    /// Implied pair of ([`Tag`], [`Value`])
    pub(crate) const fn implied_tag_value(&self) -> TagValueCombo {
        TagValueCombo::new(self.implied_tag.id(), self.implied_val.id())
    }
}

impl TryFrom<&Row<'_>> for Implication {
    type Error = rsq::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            implying_tag: Tag::new(
                row.get("tag.id")?,
                row.get::<_, String>("tag.name")?,
                row.get::<_, String>("tag.color")?,
            ),
            implying_val: Value::new(row.get("value.id")?, row.get("value.name")?),
            implied_tag:  Tag::new(
                row.get(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ),
            implied_val:  Value::new(row.get(8)?, row.get(9)?),
        })
    }
}

// =================== Implications ===================

/// Vector of [`Implication`]s
pub(crate) struct Implications {
    /// Inner `Vec` of `Implication`s
    inner: Vec<Implication>,
}

from_vec!(Implication, Implications);

impl Implications {
    /// Create a new set of [`Implications`]
    pub(crate) fn new(i: Vec<Implication>) -> Self {
        Self { inner: i }
    }

    /// Add an [`Implication`] to the set of [`Implications`]
    pub(crate) fn push(&mut self, implication: Implication) {
        self.inner.push(implication);
    }

    /// Return the vector of [`Implication`]s
    pub(crate) fn inner(&self) -> &[Implication] {
        &self.inner
    }

    /// Determine whether the given [`TagValueCombo`] is an implied one
    pub(crate) fn implies(&self, other: &TagValueCombo) -> bool {
        self.inner
            .iter()
            .any(|i| i.implied_tag.id() == other.tag_id() && i.implied_val.id() == other.value_id())
    }

    /// Determine whether [`Implications`] contains a given [`Implication`]
    pub(crate) fn contained(&self, other: &Implication) -> bool {
        self.inner.iter().any(|i| i == other)
    }
}
