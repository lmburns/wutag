#![allow(unused)]

use std::path::PathBuf;

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
    /// Do not show errors that tag already exists
    #[clap(name = "quiet", long, short = 'q')]
    quiet: bool,
}

impl App {
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn testing(&mut self, opts: &TestingOpts) -> Result<()> {
        log::debug!("Using registry: {}", self.registry.path.display());

        println!("TESTING");

        let path = PathBuf::from("./my.db");
        let conn = rusqlite::Connection::open(&path)?;
        let reg = crate::registry::Registry::new(&path, conn, self.follow_symlinks)?;
        reg.init();

        let txn = crate::registry::transaction::Txn::new(&reg)?;

        let test = PathBuf::from("./justfile");
        let ret = txn.insert_file(test)?;
        println!("RET: {:#?}", ret);
        txn.commit()?;

        Ok(())
    }
}
