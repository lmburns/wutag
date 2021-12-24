//! [`Uuid`] wrapper

use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};
use std::str::FromStr;
use uuid::Uuid;

/// [`wutag`] [`Uuid`].
///
/// [`Uuid`] wrapper that allows for custom `impl`s
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Wuid {
    /// Inner `Uuid`
    uuid: Uuid,
}

impl Wuid {
    /// Create a new [`Wuid`]
    pub(crate) fn new() -> Self {
        Uuid::new_v4().into()
    }
}

impl From<Uuid> for Wuid {
    fn from(uuid: Uuid) -> Self {
        Self { uuid }
    }
}

impl From<Wuid> for Uuid {
    fn from(wuid: Wuid) -> Self {
        wuid.uuid
    }
}

impl FromStr for Wuid {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            uuid: Uuid::parse_str(s)?,
        })
    }
}

impl ToSql for Wuid {
    fn to_sql(&self) -> rsq::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.uuid.to_string()))
    }
}

impl FromSql for Wuid {
    fn column_result(val: ValueRef) -> Result<Self, FromSqlError> {
        match Self::from_str(val.as_str().expect("failed to convert Wuid to `str`")) {
            Ok(v) => Ok(v),
            Err(err) => Err(FromSqlError::Other(Box::new(err))),
        }
    }
}

impl From<Wuid> for ToSqlOutput<'_> {
    #[inline]
    fn from(t: Wuid) -> Self {
        ToSqlOutput::Owned(t.uuid.into())
    }
}
