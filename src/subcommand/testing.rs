#![allow(unused)]

use anyhow::Context;
use std::path::PathBuf;

use crate::registry::types::ID;

use super::{
    uses::{
        bold_entry, collect_stdin_paths, err, fmt_err, fmt_path, fmt_tag, glob_builder,
        parse_color, reg_ok, regex_builder, wutag_error, wutag_fatal, Arc, Args, Colorize,
        DirEntryExt, EntryData, Result, Tag, ValueHint, DEFAULT_COLOR,
    },
    App,
};

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
        log::debug!("Using registry: {}", self.registry.path.display());

        let path = PathBuf::from("./my.db");
        let conn = rusqlite::Connection::open(&path)?;
        let reg = crate::registry::Registry::new(&path, conn, self.follow_symlinks)?;
        reg.init()?;

        let txn = crate::registry::transaction::Txn::new(&reg)?;

        // let test = PathBuf::from("./justfile");
        // let ret = txn.insert_file(test)?;

        // let test = PathBuf::from("./LICENSE");
        // let ret = txn.insert_file(test)?;
        //
        // let test = PathBuf::from("./README.md");
        // let ret = txn.insert_file(test)?;

        let q = crate::registry::querier::ast::query::Query::new(&opts.query.join(" "), None);
        let parsed = q.parse();

        if let Ok(p) = parsed {
            println!("parsed: {:#?}", p);
        }

        // txn.test_regex()?;
        // let fs = txn.select_files_by_glob("name", "j*")?;
        // println!("RES: {:#?}", fs);
        // txn.commit()?;

        // println!("MATCH: {:#?}", f);
        // println!("files: {:#?}", f.inner()[1]);

        Ok(())
    }
}
