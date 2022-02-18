#![allow(unused)]

use super::{
    uses::{
        bold_entry, collect_stdin_paths, err, fmt_err, fmt_path, fmt_tag, glob_builder,
        parse_color, reg_ok, regex_builder, wutag_error, wutag_fatal, Arc, Args, Colorize,
        DirEntryExt, EntryData, Result, Tag, ValueHint, DEFAULT_COLOR,
    },
    App,
};

use super::set::SetOpts;

impl App {
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn set2(&mut self, opts: &SetOpts) -> Result<()> {
        log::debug!("SetOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());

        Ok(())
    }
}
