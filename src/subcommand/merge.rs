#![allow(unused)]
//! `merge` - Merge tags on results of a query

// XXX: Implement

use super::App;
use crate::{
    bold_entry, filesystem as wfs,
    utils::{crawler, fmt, glob_builder, regex_builder},
    wutag_error,
    xattr::tag::DirEntryExt,
};
use anyhow::Result;
use clap::{Args, ValueHint};
// use rayon::prelude::*;
use std::{borrow::Cow, ffi::OsStr, sync::Arc};

/// Arguments used for the `merge` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct MergeOpts {
    /// Merge values from files instead of tags
    #[clap(
        name = "values",
        long = "values",
        short = 'V',
        takes_value = false,
        long_help = "By default this command will clear all tags and values from the results. \
                     With this flag only the values will be removed"
    )]
    pub(crate) values: bool,

    /// A glob, regular expression, or fixed-string pattern
    #[clap(
        name = "pattern",
        takes_value = true,
        value_hint = ValueHint::FilePath,
    )]
    pub(crate) pattern: String,
}

impl App {
    /// Merge [`Tag`]s or [`Value`]s from a given path
    pub(crate) fn merge(&mut self, opts: &MergeOpts) -> Result<()> {
        log::debug!("MergeOpts: {:#?}", opts);

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

        if self.global {
            let exclude_pattern = regex_builder(
                self.exclude.join("|").as_str(),
                self.case_insensitive,
                self.case_sensitive,
            );

            for entry in reg.files(None)?.iter() {
                let path = &entry.path();
                let search_str: Cow<OsStr> = Cow::Owned(path.as_os_str().to_os_string());
                let search_bytes = &wfs::osstr_to_bytes(search_str.as_ref());
                if !self.exclude.is_empty() && exclude_pattern.is_match(search_bytes) {
                    continue;
                }

                if let Some(ref ext) = self.extension {
                    if !ext.is_match(search_bytes) {
                        continue;
                    }
                }

                // if re.is_match(search_bytes) {
                //     if !self.quiet {
                //         println!("{}:", self.fmt_path(entry.path(),));
                //     }
                // }
            }
        } else {
            drop(reg);

            crawler(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisoned lock");

                    // For each file
                    if let Ok(file) = reg.file_by_path(entry.path()) {
                        let path = &file.path();
                        let tags = reg.tags_for_file(&file)?;

                        if !self.quiet {
                            println!("{}:", self.fmt_path(entry.path(),));
                        }
                    }

                    Ok(())
                },
            );
        }

        Ok(())
    }
}
