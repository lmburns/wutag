//! A value that a [`Tag`] can have. In the database, the table is named
//! `xattr`, since it is an extended attribute of the [`Tag`] itself

use super::{from_vec, wuid::Wuid, ID};
use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};
use serde::{Deserialize, Serialize};

/// Alias to [`Uuid`](uuid::Uuid)
// pub(crate) type ValueId = Wuid;
pub(crate) type ValueId = ID;

// ======================= Value ======================

/// The representation of a "`Tag`'s tag"
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Value {
    /// The value's unique identifier
    id:   ValueId,
    /// The string representation of the value
    name: String,
}

impl Value {
    /// Return the [`ValueId`]
    pub(crate) const fn id(&self) -> ValueId {
        self.id
    }

    /// Return the [`Value`] name
    pub(crate) const fn name(&self) -> &String {
        &self.name
    }

    /// Create a new `Value`
    pub(crate) const fn new(id: ValueId, name: String) -> Self {
        Self { id, name }
    }
}

impl ToSql for Value {
    fn to_sql(&self) -> rsq::Result<ToSqlOutput> {
        let string = serde_json::to_string(self)
            .map_err(|e| rsq::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::from(string))
    }
}

#[allow(clippy::wildcard_enum_match_arm)]
impl FromSql for Value {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(d) | ValueRef::Blob(d) => serde_json::from_slice(d),
            _s => {
                // let val = s.as_i64();
                return Err(FromSqlError::InvalidType);
            },
        }
        .map_err(|err| FromSqlError::Other(Box::new(err)))
    }
}

impl TryFrom<&Row<'_>> for Value {
    type Error = rsq::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id:   row.get("id")?,
            name: row.get("name")?,
        })
    }
}

// ====================== Values ======================

/// A vector of [`Value`]s
#[derive(Debug, Clone)]
pub(crate) struct Values {
    /// The inner vector of [`Value`]s
    inner: Vec<Value>,
}

from_vec!(Value, Values);

impl Values {
    /// Create a new set of [`Values`]
    pub(crate) fn new(v: Vec<Value>) -> Self {
        Self { inner: v }
    }

    /// Add a [`Value`] to the set of [`Values`]
    pub(crate) fn push(&mut self, file: Value) {
        self.inner.push(file);
    }
}
