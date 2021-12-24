//! A wrapper around [`Color`](colored::Color) to allow for custom `impl`s

use anyhow::{Context, Result};
use colored::{Color, Colorize};
use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};
use serde::{Deserialize, Serialize};
use wutag_core::{color::parse_color, tag::Tag as WTag};
