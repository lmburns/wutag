//! `clear` - Clear tags on results of a query

#![allow(unused)]

use super::App;
use crate::{
    bold_entry, err,
    filesystem::osstr_to_bytes,
    registry::types::{Tag, ID},
    utils::{crawler, fmt, glob_builder, regex_builder},
    wutag_error, wutag_info,
    xattr::tag::{clear_tags, has_tags, DirEntryExt},
};
use clap::{Args, ValueHint};
use colored::Colorize;
use std::{path::PathBuf, sync::Arc};

/// Arguments used for the `clear` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct ClearOpts {
    /// Clear values from files instead of tags
    #[clap(
        name = "values",
        long,
        short = 'V',
        takes_value = false,
        long_help = "By default this command will clear all tags and values from the results. \
                     With this flag only the values will be removed"
    )]
    pub(crate) values: bool,

    /// A glob, regular expression, or fixed-string pattern
    /// A glob, regular expression, or fixed-string
    #[clap(
        name = "pattern",
        takes_value = true,
        value_hint = ValueHint::FilePath,
    )]
    pub(crate) pattern: String,
}

impl App {
    /// Clear [`Tag`]s or [`Value`]s from a given path
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
            //                         fmt::path(entry.path(), self.base_color,
            // self.ls_colors)                     );
            //                     if let Err(e) = clear_tags(entry.path()) {
            //                         err!('\t', e, entry);
            //                     } else if !self.quiet {
            //                         println!("\t{}", fmt::ok("cleared"));
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
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisoned lock");

                    let handle_xattr = |tag: &Tag, path: &PathBuf| {
                        log::debug!("removing xattr for Tag({})", tag.name());
                        match path.has_tags() {
                            Ok(has_tags) =>
                                if has_tags && !self.quiet {
                                    println!("{}:", self.fmt_path(entry.path(),));
                                    if path.get_tag(tag).is_err() {
                                        wutag_error!(
                                            "{}: found ({}) in database, though file has no xattrs",
                                            bold_entry!(path),
                                            self.fmt_tag(tag)
                                        );
                                    } else if let Err(e) = path.clear_tags() {
                                        wutag_error!("\t{} {}", e, bold_entry!(path));
                                    } else {
                                        println!("\t{}", fmt::ok("cleared"));
                                    }
                                },
                            Err(e) => {
                                wutag_error!("{}: {}", e, bold_entry!(path));
                            },
                        };
                    };

                    // For each file
                    if let Ok(file) = reg.file_by_path(entry.path()) {
                        let path = &file.path();
                        let tags = reg.tags_for_file(&file)?;

                        println!("FILE: {:#?}", path);
                        if opts.values {
                            println!("HAS VALUE FLAG");

                            for value in reg.values_by_fileid(file.id())?.iter() {
                                println!("VALUE: {:#?}", value);

                                if reg.value_count_by_id(value.id())? == 1 {
                                    // Then go ahead and delete it
                                    if let Err(e) = reg.delete_value_only(value.id()) {
                                        println!("FAIL DELETE VALUE");
                                        wutag_error!(
                                            "{}: failed to delete value {}: {}",
                                            bold_entry!(path),
                                            value.name().color(self.base_color).bold(),
                                            e
                                        );
                                        continue;
                                    }
                                }
                                if let Err(e) = reg.update_filetag_valueid(value.id(), file.id()) {
                                    println!("FAIL UPDATE");
                                    // Otherwise, just remove it from this single file
                                    wutag_error!(
                                        "{}: failed to update value {}: {}",
                                        bold_entry!(path),
                                        value.name().color(self.base_color).bold(),
                                        e
                                    );
                                    continue;
                                }

                                print!(
                                    "\t{} {} (V)",
                                    "X".bold().red(),
                                    value.name().color(self.base_color).bold(),
                                );
                            }
                        } else {
                            // For each tag
                            for tag in tags.iter() {
                                let mut values_ = vec![];
                                if let Ok(values) = reg.values_by_tagid(tag.id()) {
                                    // For each value
                                    for value in values.iter().cloned() {
                                        if reg.value_count_by_id(value.id())? == 1 {
                                            values_.push(value);
                                        }
                                    }
                                }

                                if reg.tag_count_by_id(tag.id())? == 1 {
                                    log::debug!("{}: deleting tag {}", path.display(), tag.name());
                                    if let Err(e) = reg.delete_tag(tag.id()) {
                                        wutag_error!(
                                            "{}: failed to delete tag {}: {}",
                                            bold_entry!(path),
                                            self.fmt_tag(tag),
                                            e
                                        );
                                        continue;
                                    }
                                } else if let Err(e) =
                                    reg.delete_filetag_by_fileid_tagid(file.id(), tag.id())
                                {
                                    wutag_error!(
                                        "{}: failed to delete FileTag {}",
                                        path.display(),
                                        e
                                    );
                                    continue;
                                }

                                handle_xattr(tag, path);
                            }
                        }
                    }

                    Ok(())
                },
            );
        }
    }
}
