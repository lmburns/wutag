//! Tests used to check the API calls

use super::super::{
    transaction::Txn,
    types::{
        file::{File, FileId, Files, MimeType},
        filetag::{FileTag, FileTags},
        query::{Queries, Query},
        tag::{Tag, TagId, Tags},
        value::{Value, ValueId},
    },
    Registry,
};
use crate::filesystem as wfs;
use anyhow::{Context, Result};
use rusqlite::{self as rsq, params, Connection};
use std::{env, os::unix::prelude::MetadataExt, path::PathBuf, str::FromStr};
use wutag_core::color::parse_color;

const DB_NAME: &str = "./tests/my.db";

macro_rules! get_0 {
    ($f:tt) => {
        $f.get(0).context("first idx")?
    };
}

macro_rules! first_idx {
    ($files:tt) => {
        assert_eq!(get_0!($files).name(), "README.md");
    };
}

fn setup_db() -> Result<Registry> {
    let path = PathBuf::from(DB_NAME);
    let reg = Registry::new(&path, false)?;
    reg.init()?;

    Ok(reg)
}

fn setup_dbfn<F>(mut f: F) -> Result<()>
where
    F: FnMut(&Registry) -> Result<()>,
{
    scopeguard::defer!(wfs::delete_file(DB_NAME));
    let reg = setup_db()?;
    f(&reg);

    // reg.commit()?;

    Ok(())
}
