//! `list` - List `Tag`s or `File`s within the database
#![allow(unused)]

// TODO: list files relative to directory as an option

use super::{
    list::{ListObject, ListOpts},
    uses::{
        contained_path, fmt_local_path, fmt_path, fmt_tag, global_opts, raw_local_path,
        ColorChoice, Colorize, HashMap,
    },
    App,
};
use itertools::Itertools;

#[cfg(feature = "prettify")]
use super::uses::{print_stdout, Border, Cell, Justify, Separator, Style, Table};

impl App {
    /// List `Tags` or `Files` in the database
    pub(crate) fn list2(&self, opts: &ListOpts) {
        log::debug!("ListOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());

        // let mut table = vec![];
        let colorchoice = match self.color_when.as_ref() {
            "always" => ColorChoice::Always,
            "never" => ColorChoice::Never,
            _ => ColorChoice::Auto,
        };

        match opts.object {
            ListObject::Files {
                with_tags,
                formatted,
                border,
                garrulous,
            } => {
                println!("list files");
            },
            ListObject::Tags {
                no_count,
                border,
                explicit,
                one_per_line,
                unique,
                sort,
            } => {
                println!("listing tags");
            },
        }
    }
}
