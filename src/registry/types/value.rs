//! A value that a [`Tag`] can have. In the database, the table is named
//! `xattr`, since it is an extended attribute of the [`Tag`] itself

use super::{
    super::querier::{COMPARISON_OPS, CONDITIONAL_RES, FUNC_NAMES, OTHER_RES},
    from_vec, impl_vec, validate_name, ID,
};
use anyhow::{anyhow, Result};
use colored::Colorize;
use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};
use serde::{Deserialize, Serialize};
use std::fmt;

// ╭──────────────────────────────────────────────────────────╮
// │                         ValueId                          │
// ╰──────────────────────────────────────────────────────────╯

/// Alias to [`ID`](super::ID)
pub(crate) type ValueId = ID;

/// A vector of [`ValueId`]s
#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct ValueIds {
    inner: Vec<ValueId>,
}

from_vec!(ValueId, ValueIds);
impl_vec!(ValueIds, ValueId);

// ╭──────────────────────────────────────────────────────────╮
// │                          Value                           │
// ╰──────────────────────────────────────────────────────────╯

/// The representation of a "[`Tag`]'s tag"
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub(crate) struct Value {
    /// The value's unique identifier
    id:   ValueId,
    /// The string representation of the value
    name: String,
}

impl Value {
    validate_name!("value name", "values");

    /// Return the [`ValueId`]
    pub(crate) const fn id(&self) -> ValueId {
        self.id
    }

    /// Return the [`Value`] name
    pub(crate) const fn name(&self) -> &String {
        &self.name
    }

    /// Create a new [`Value`]
    pub(crate) const fn new(id: ValueId, name: String) -> Self {
        Self { id, name }
    }

    /// Create a new [`Value`] with no id
    pub(crate) fn new_noid<S: AsRef<str>>(name: S) -> Self {
        Self {
            id:   ValueId::null(),
            name: name.as_ref().to_owned(),
        }
    }

    /// Check whether the [`Value`] ID is null
    pub(crate) fn is_null_id(&self) -> bool {
        self.id == ValueId::null()
    }

    /// Check whether the [`Value`] name is null
    pub(crate) fn is_null_name(&self) -> bool {
        self.name.is_empty()
    }

    /// Check whether the [`Value`] is null
    pub(crate) fn is_null(&self) -> bool {
        self.is_null_id() && self.is_null_name()
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

// ╭──────────────────────────────────────────────────────────╮
// │                          Values                          │
// ╰──────────────────────────────────────────────────────────╯

/// A vector of [`Value`]s
#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct Values {
    /// The inner vector of [`Value`]s
    inner: Vec<Value>,
}

from_vec!(Value, Values);

impl Values {
    impl_vec!(Value);

    /// Does the inner vector contain a specific [`Value`] by [`ID`]?
    pub(crate) fn contains(&self, other: &Value) -> bool {
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

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

mod test {
    use super::{ValueId, ValueIds};

    #[test]
    fn unique_valueids() {
        let v = vec![1, 2, 5, 5, 3, 1, 7]
            .iter()
            .map(|i| ValueId::new(*i))
            .collect::<Vec<_>>();
        let mut ids = ValueIds::new(v);

        assert!(ids.len() == 7);

        ids.unique();
        assert!(ids.len() == 5);

        assert_eq!(ids, ValueIds {
            inner: vec![1, 2, 3, 5, 7]
                .iter()
                .map(|i| ValueId::new(*i))
                .collect::<Vec<_>>(),
        });
    }
}
