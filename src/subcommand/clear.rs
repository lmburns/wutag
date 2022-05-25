//! `clear` - Clear tags from a given path

#![allow(unused)]

use super::App;
use crate::{
    bold_entry, err,
    filesystem::osstr_to_bytes,
    registry::types::{
        tag::{clear_tags, has_tags, DirEntryExt},
        ID,
    },
    util::{crawler, fmt_path, glob_builder, regex_builder},
    wutag_error, wutag_info,
};
use clap::Args;
// use colored::Colorize;
use std::sync::Arc;

/// Arguments used for the `clear` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct ClearOpts {
    /// Follow symlinks before clearing tags or values
    #[clap(
        name = "follow-symlinks",
        long,
        short = 'f',
        takes_value = false,
        long_help = "Should the symlink be dereferenced before the tag(s) are cleared from the \
                     file"
    )]
    pub(crate) follow_symlinks: bool,

    // /// Clear values instead of tags
    // #[clap(
    //     name = "values",
    //     long,
    //     short = 'V',
    //     takes_value = false,
    //     long_help = "Should the symlink be dereferenced before the tag is removed from the file"
    // )]
    // pub(crate) values: bool,
    /// A glob pattern like "*.png".
    pub(crate) pattern: String,
}

impl App {
    /// Clear `Tag`s from a given path
    pub(crate) fn clear(&mut self, opts: &ClearOpts) {
        log::debug!("ClearOpts: {:#?}", opts);

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

        // If not needed, delete
        let reg = self.registry.lock().expect("poisoned lock");

        if self.global {
            // let exclude_pattern = regex_builder(
            //     self.exclude.join("|").as_str(),
            //     self.case_insensitive,
            //     self.case_sensitive,
            // );
            // for (&id, entry) in self.oregistry.clone().list_entries_and_ids()
            // {     let search_str: Cow<OsStr> =
            // Cow::Owned(entry.path().as_os_str().to_os_string());
            //     let search_bytes = &osstr_to_bytes(search_str.as_ref());
            //     if !self.exclude.is_empty() &&
            // exclude_pattern.is_match(search_bytes) {
            //         continue;
            //     }
            //
            //     if let Some(ref ext) = self.extension {
            //         if !ext.is_match(search_bytes) {
            //             continue;
            //         }
            //     }
            //
            //     if re.is_match(search_bytes) {
            //         self.oregistry.clear_entry(id);
            //         match has_tags(entry.path()) {
            //             Ok(has_tags) =>
            //                 if has_tags && !self.quiet {
            //                     println!(
            //                         "{}:",
            //                         fmt_path(entry.path(), self.base_color,
            // self.ls_colors)                     );
            //                     if let Err(e) = clear_tags(entry.path()) {
            //                         err!('\t', e, entry);
            //                     } else if !self.quiet {
            //                         println!("\t{}", fmt_ok("cleared"));
            //                     }
            //                 },
            //             Err(e) => {
            //                 err!(e, entry);
            //             },
            //         }
            //     }
            // }
            // log::debug!("Saving registry...");
            // self.save_registry();
        } else {
            drop(reg);

            crawler(
                &Arc::new(re),
                &Arc::new(self.clone()),
                opts.follow_symlinks,
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisoned lock");

                    if let Ok(file) = reg.file_by_path(entry.path()) {
                        let path = &file.path();
                        let ftags = reg.tags_for_file(&file)?;

                        for t in ftags.iter() {
                            // If the tag has values
                            if let Ok(values) = reg.values_by_tagid(t.id()) {
                                for value in values.iter() {
                                    if reg.value_count_by_id(value.id())? == 1 {
                                        reg.delete_value(value.id())?;
                                    } else {
                                        reg.delete_filetag(file.id(), t.id(), value.id())?;
                                    }
                                }
                                if reg.tag_count_by_id(t.id())? == 1 {
                                    reg.delete_tag(t.id())?;
                                }
                            // If the tag is only connected to this file
                            // } else if reg.tag_count_by_id(t.id())? == 1 {
                            //     for pair in &combos {
                            //         if t.id() != pair.tag_id() {
                            //             reg.delete_tag(t.id())?;
                            //         }
                            //     }
                            } else {
                                reg.delete_filetag(file.id(), t.id(), ID::null())?;
                            }

                            match path.has_tags() {
                                Ok(has_tags) =>
                                    if has_tags {
                                        if let Err(e) = path.clear_tags() {
                                            wutag_error!("\t{} {}", e, bold_entry!(path));
                                        }
                                    },
                                Err(e) => {
                                    wutag_error!("{} {}", e, bold_entry!(path));
                                },
                            }
                        }
                    }

                    // if let Some(id) = self.oregistry.find_entry(entry.path()) {
                    //     self.oregistry.clear_entry(id);
                    // }
                    //
                    // match entry.has_tags() {
                    //     Ok(has_tags) =>
                    //         if has_tags && !self.quiet {
                    //             println!(
                    //                 "{}:",
                    //                 fmt_path(entry.path(), self.base_color, self.ls_colors)
                    //             );
                    //             if let Err(e) = entry.clear_tags() {
                    //                 err!('\t', e, entry);
                    //             } else if !self.quiet {
                    //                 println!("\t{}", fmt_ok("cleared"));
                    //             }
                    //         },
                    //     Err(e) => {
                    //         err!(e, entry);
                    //     },
                    // }
                    // log::debug!("Saving registry...");
                    // self.save_registry();

                    Ok(())
                },
            );
        }
    }
}
