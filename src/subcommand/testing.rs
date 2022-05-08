#![allow(unused)]

use super::App;
use crate::{
    bold_entry, err,
    oregistry::EntryData,
    registry::types::{Tag, ID},
    util::{collect_stdin_paths, fmt_err, fmt_path, fmt_tag, glob_builder, reg_ok, regex_builder},
    wutag_error, wutag_fatal,
};
use anyhow::{Context, Result};
use clap::{Args, ValueHint};
use colored::Colorize;
use e2p_fileflags::{FileFlags, Flags};
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
    sync::Arc,
};
use wutag_core::{
    color::parse_color,
    tag::{DirEntryExt, Tag as WTag, DEFAULT_COLOR},
};

use crate::registry::types::{FileId, FileTag, TagId, ValueId};

#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct TestingOpts {
    #[rustfmt::skip]
    #[clap(
        name = "query",
        min_values = 1,
        long = "query",
        short = 'q',
        value_name = "query",
        value_terminator = ";",
        allow_hyphen_values = true,
        forbid_empty_values = true,
        help_heading = "FFF HEADING",
        value_hint = ValueHint::CommandString,
    )]
    pub(crate) query: Vec<String>,
}

impl App {
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn testing(&mut self, opts: &TestingOpts) -> Result<()> {
        let reg = self.registry.lock().expect("poisioned lock");
        let txn = reg.txn()?;
        log::debug!("Using registry: {}", self.oregistry.path.display());

        reg.insert_file(&txn, PathBuf::from("./cliff.toml"))?;
        // reg.insert_file(PathBuf::from("./LICENSE"))?;
        // reg.insert_file(PathBuf::from("./README.md"))?;
        // reg.insert_file(PathBuf::from("./Cargo.lock"))?;
        // reg.insert_file(PathBuf::from("./CHANGELOG.md"))?;

        reg.insert_tag(&Tag::new_noid("tag1", "#FF00FF"))?;
        reg.insert_tag(&Tag::new_noid("tag2", "0xFF5813"))?;
        reg.insert_tag(&Tag::new_noid("tag3", "A06469"))?;
        reg.insert_tag(&Tag::new_noid("tag4", "red"))?;

        reg.insert_value("2022")?;

        reg.insert_filetag(
            &txn,
            &FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1)),
        )?;

        println!("INIT TESTING");

        Ok(())
    }
}
