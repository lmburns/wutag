//! `clear` - Clear tags from a given path

use super::App;
use crate::{
    err,
    filesystem::osstr_to_bytes,
    util::{fmt_err, fmt_ok, fmt_path, glob_builder, reg_ok, regex_builder},
};
use clap::Args;
use colored::Colorize;
use std::{borrow::Cow, ffi::OsStr, sync::Arc};
use wutag_core::tag::{clear_tags, has_tags, DirEntryExt};

/// Arguments used for the `clear` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct ClearOpts {
    /// A glob pattern like "*.png".
    pub(crate) pattern: String,
}

impl App {
    /// Clear `Tag`s from a given path
    pub(crate) fn clear(&mut self, opts: &ClearOpts) {
        log::debug!("ClearOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.oregistry.path.display());
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);

        if self.global {
            let exclude_pattern = regex_builder(
                self.exclude.join("|").as_str(),
                self.case_insensitive,
                self.case_sensitive,
            );
            for (&id, entry) in self.oregistry.clone().list_entries_and_ids() {
                let search_str: Cow<OsStr> = Cow::Owned(entry.path().as_os_str().to_os_string());
                let search_bytes = &osstr_to_bytes(search_str.as_ref());
                if !self.exclude.is_empty() && exclude_pattern.is_match(search_bytes) {
                    continue;
                }

                if let Some(ref ext) = self.extension {
                    if !ext.is_match(search_bytes) {
                        continue;
                    }
                }

                if re.is_match(search_bytes) {
                    self.oregistry.clear_entry(id);
                    match has_tags(entry.path()) {
                        Ok(has_tags) =>
                            if has_tags && !self.quiet {
                                println!(
                                    "{}:",
                                    fmt_path(entry.path(), self.base_color, self.ls_colors)
                                );
                                if let Err(e) = clear_tags(entry.path()) {
                                    err!('\t', e, entry);
                                } else if !self.quiet {
                                    println!("\t{}", fmt_ok("cleared"));
                                }
                            },
                        Err(e) => {
                            err!(e, entry);
                        },
                    }
                }
            }
            log::debug!("Saving registry...");
            self.save_registry();
        } else {
            reg_ok(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    if let Some(id) = self.oregistry.find_entry(entry.path()) {
                        self.oregistry.clear_entry(id);
                    }

                    match entry.has_tags() {
                        Ok(has_tags) =>
                            if has_tags && !self.quiet {
                                println!(
                                    "{}:",
                                    fmt_path(entry.path(), self.base_color, self.ls_colors)
                                );
                                if let Err(e) = entry.clear_tags() {
                                    err!('\t', e, entry);
                                } else if !self.quiet {
                                    println!("\t{}", fmt_ok("cleared"));
                                }
                            },
                        Err(e) => {
                            err!(e, entry);
                        },
                    }
                    log::debug!("Saving registry...");
                    self.save_registry();

                    Ok(())
                },
            );
        }
    }
}
