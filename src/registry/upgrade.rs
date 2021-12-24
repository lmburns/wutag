//! Upgrade the database by recreating tables or modifying the [`Version`]

use super::{common::version::Version, Registry, Txn};
use crate::wutag_info;
use anyhow::{Context, Result};
use colored::Colorize;
use std::convert::TryInto;

use rusqlite::{
    self as rsq, params,
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput},
    Row,
};

// ======================= Txn ========================
// ================= Upgrade Actions ==================

impl Txn<'_> {}
