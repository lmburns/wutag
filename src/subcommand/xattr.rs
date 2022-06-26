// Testing command that will be used in the transition of serializing values to
// xattr
#![allow(unused)]
#![allow(clippy::unnecessary_wraps)]

use super::{parse_tag_val, App};
use crate::{
    bold_entry, qprint,
    registry::{
        common::hash,
        types::{FileTag, Tag, TagValueCombo, ID},
    },
    utils::{collect_stdin_paths, color::parse_color, crawler, glob_builder, regex_builder},
    wutag_error, wutag_warning,
    xattr::tag::DirEntryExt,
};
use anyhow::{anyhow, Result};
use clap::{Args, ValueHint};
use colored::Colorize;
use rusqlite as rsq;
use std::sync::Arc;

#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct XattrOpts {
    /// A glob, regular expression, or fixed-string
    #[clap(name = "pattern", takes_value = true)]
    pub(crate) pattern: String,

    /// Tag or tags to set on the result of the pattern
    #[clap(
        name = "tags",
        long_help = "Specify a list of tags to set the results of the pattern matching or STDIN to. This \
                     option is required unless `-p`/`--pairs` is used, which allows specifying `tag`, \
                     `value` pairs"
    )]
    pub(crate) tags: Vec<String>,
}

impl App {
    /// Clean the database by removing all entries
    pub(crate) fn xattr(&mut self, opts: &XattrOpts) -> Result<()> {
        let re = regex_builder(
            &{
                if self.pat_regex {
                    String::from(&opts.pattern)
                } else if self.fixed_string {
                    regex::escape(&opts.pattern)
                } else {
                    glob_builder(&opts.pattern, self.wildcard_matches_sep)
                }
            },
            self.case_insensitive,
            self.case_sensitive,
        );

        let reg = self.registry.lock().expect("poisoned lock");

        drop(reg);

        crawler(
            &Arc::new(re),
            &Arc::new(self.clone()),
            |entry: &ignore::DirEntry| {
                let reg = self.registry.lock().expect("poisoned lock");

                // This is needed for single files. The WalkBuilder doesn't seem to list the
                // resolved symlink if it is a single file. However, symbolic directories are
                // traversed
                let path = &self.resolve_symlink(entry.path())?;
                let path_d = path.display();
                qprint!(self, "{}:", self.fmt_path(path));

                let tags = path.list_tags()?;
                println!("TAGS: {:#?}", tags);

                Ok(())
            },
        );

        Ok(())
    }
}
