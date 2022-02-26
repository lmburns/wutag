#![allow(unused)]

use anyhow::Context;
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
};

use crate::registry::types::ID;
use e2p_fileflags::{FileFlags, Flags};

use super::{
    uses::{
        bold_entry, collect_stdin_paths, err, fmt_err, fmt_path, fmt_tag, glob_builder,
        parse_color, reg_ok, regex_builder, wutag_error, wutag_fatal, Arc, Args, Colorize,
        DirEntryExt, EntryData, Result, Tag, ValueHint, DEFAULT_COLOR,
    },
    App,
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
        log::debug!("Using registry: {}", self.oregistry.path.display());

        reg.insert_file(PathBuf::from("./cliff.toml"))?;
        reg.insert_file(PathBuf::from("./LICENSE"))?;
        reg.insert_file(PathBuf::from("./README.md"))?;
        reg.insert_file(PathBuf::from("./Cargo.lock"))?;
        reg.insert_file(PathBuf::from("./CHANGELOG.md"))?;

        reg.insert_tag("tag1", "#FF00FF")?;
        reg.insert_tag("tag2", "red")?;
        reg.insert_tag("tag2", "blue")?;

        reg.insert_value("2022")?;

        reg.insert_filetag(&FileTag::new(
            FileId::new(1),
            TagId::new(1),
            ValueId::new(1),
        ))?;

        println!("INIT TESTING");

        Ok(())
    }
}
