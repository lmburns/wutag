#![allow(unused)]
//! `merge` - Merge tags on results of a query

// XXX: Implement

use super::App;
use crate::{
    bold_entry, filesystem as wfs, g, qprint, r,
    registry::types::{FileTag, ValueId},
    utils::{crawler, fmt, glob_builder, regex_builder},
    wutag_error, wutag_fatal,
    xattr::tag::DirEntryExt,
};
use anyhow::Result;
use clap::{Args, ValueHint};
use colored::Colorize;
// use rayon::prelude::*;
use std::{borrow::Cow, ffi::OsStr, fmt::Debug, sync::Arc};

/// Arguments used for the `merge` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct MergeOpts {
    /// Merge values from files instead of tags
    #[clap(
        name = "values",
        long = "values",
        short = 'V',
        takes_value = false,
        long_help = "By default this command will clear all tags and values from the results. With this \
                     flag only the values will be removed"
    )]
    pub(crate) values: bool,

    /// A glob, regular expression, or fixed-string pattern
    #[clap(
        name = "pattern",
        long = "pattern",
        short = 'p',
        takes_value = true,
        value_hint = ValueHint::FilePath,
        long_help = "A pattern which limits which file's tags or values will be merged. \
            To merge all items in the registry that have a tag or value, use the --global flag."
    )]
    pub(crate) pattern: Option<String>,

    /// The tag or value that is being merged into
    #[clap(
        name = "dest",
        value_name = "dest",
        takes_value = true,
        long_help = "This is the item that will have other items merged into, leaving this item intact"
    )]
    pub(crate) dest: String,

    /// The tag(s)/value(s) that are going to be merged
    #[clap(
        name = "source",
        value_name = "source",
        takes_value = true,
        long_help = "This is the item that will be renamed to the <dest> item"
    )]
    pub(crate) source: Vec<String>,
}

impl App {
    /// Merge [`Tag`]s or [`Value`]s from a given path
    pub(crate) fn merge(&mut self, opts: &MergeOpts) -> Result<()> {
        log::debug!("MergeOpts: {:#?}", opts);

        let pattern = opts
            .pattern
            .as_ref()
            .map_or_else(|| String::from("*"), ToOwned::to_owned);

        let re = regex_builder(
            &{
                if self.pat_regex {
                    pattern
                } else if self.fixed_string {
                    regex::escape(&pattern)
                } else {
                    glob_builder(&pattern, self.wildcard_matches_sep)
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

            let mut tags_to_delete = vec![];

            let files = reg.files(None)?;
            for entry in files.iter() {
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

                if re.is_match(search_bytes) {
                    let dest_by_name = reg.tag_by_name(&opts.dest);
                    match dest_by_name {
                        Ok(dest) => {
                            let fmt_dest = self.fmt_tag(&dest);

                            for tag in &opts.source {
                                // Make sure the source tags are also in the registry
                                //
                                // This check isn't necessary, the file could just be checked
                                // However, this will provide more useful errors
                                let inner_tag = reg.tag_by_name(tag);
                                match inner_tag {
                                    Ok(source) => {
                                        let fmt_source = self.fmt_tag(&source);

                                        if source == dest {
                                            wutag_error!("unable to merge tag {} into itself", fmt_dest);
                                            continue;
                                        }

                                        // If the file has the source tag
                                        if reg.file_has_tag(entry.id(), source.id())? {
                                            qprint!(self, "{}:", self.fmt_path(path));

                                            // If this file is the only one with the tag, delete it
                                            if reg.tag_count_by_id(source.id())? == 1 {
                                                log::debug!(
                                                    "{}: deleting tag {}",
                                                    path.display(),
                                                    source.name()
                                                );

                                                tags_to_delete.push(source.clone());
                                            }

                                            // If the file has both the source tag and dest tag
                                            // then do not try and update the tag
                                            if reg.file_has_tag(entry.id(), dest.id())? {
                                                println!("\t{} already exists", fmt_dest);
                                            } else {
                                                if let Err(e) = reg.update_filetag_tagid(
                                                    source.id(),
                                                    dest.id(),
                                                    entry.id(),
                                                ) {
                                                    wutag_error!("{}: {}", bold_entry!(path), e);
                                                    continue;
                                                }

                                                let values = reg.unique_values_by_tag(source.id())?;

                                                for value in values.iter() {
                                                    log::debug!(
                                                        "{}: deleting value {}",
                                                        path.display(),
                                                        value.name()
                                                    );

                                                    if let Err(e) = reg.delete_value(value.id()) {
                                                        wutag_error!(
                                                            "{}: value {}: {}",
                                                            bold_entry!(path),
                                                            value.name().color(self.base_color).bold(),
                                                            e
                                                        );
                                                        // continue;
                                                    }
                                                }

                                                // Get the actual extended attribute
                                                if let Err(e) = path.get_tag(&source) {
                                                    wutag_error!(
                                                        "{}: failed to get extended attributes: {}",
                                                        bold_entry!(path),
                                                        e
                                                    );
                                                    continue;
                                                }

                                                // TODO: Set correct value
                                                if let Err(e) = path.replace_tag(&source, &dest, None) {
                                                    wutag_error!(
                                                        "{}: failed to swap xattr {} {} {}: {}",
                                                        bold_entry!(path),
                                                        fmt_source,
                                                        g!("=>"),
                                                        fmt_dest,
                                                        e
                                                    );
                                                    continue;
                                                }

                                                println!("\t{} {} {}", fmt_source, g!("=>"), fmt_dest,);
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        // FIXME: This occurs if there's a single file with
                                        //        a tag that is being replaced. There needs to
                                        //        be a global object to track these, or don't
                                        //        print an errror
                                        //
                                        wutag_error!(
                                            "tag {} is not found within the registry: {}",
                                            tag.bold(),
                                            e
                                        );
                                        continue;
                                    },
                                }
                            }
                        },
                        Err(e) => {
                            wutag_fatal!("failed to find tag {}: {}", opts.dest.to_string().bold(), e);
                        },
                    }

                    for tag in &tags_to_delete {
                        if let Err(e) = reg.delete_tag(tag.id()) {
                            wutag_error!("{}: tag {}: {}", bold_entry!(path), self.fmt_tag(tag), e);
                        }
                    }
                }
            }
        } else {
            drop(reg);

            crawler(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisoned lock");
                    let path = &self.resolve_symlink(entry.path())?;

                    let mut tags_to_delete = vec![];
                    let mut values_to_delete = vec![];

                    // If the file is found in the registry
                    if let Ok(file) = reg.file_by_path(path) {
                        let path = &file.path();
                        println!("\n===== FILE: {:#?} =====", path);

                        // ╭───────╮
                        // │ Value │
                        // ╰───────╯
                        if opts.values {
                            println!("REMOVING VALUES");

                            // Match destination value
                            let dest_value = reg.value_by_name(&opts.dest);
                            match dest_value {
                                Ok(dest) => {
                                    println!("FOUND DEST: {}", dest.name());

                                    let fmt_dest = dest.name().color(self.base_color).bold();

                                    // Match source value(s)
                                    for value in &opts.source {
                                        let value_name = reg.value_by_name(&value);
                                        match value_name {
                                            Ok(source) => {
                                                println!("FOUND SOURCE: {}", source.name());

                                                let fmt_source =
                                                    source.name().color(self.base_color).bold();

                                                if source == dest {
                                                    wutag_error!(
                                                        "unable to merge tag {} into itself",
                                                        fmt_dest
                                                    );
                                                    continue;
                                                }

                                                if reg.file_has_value(file.id(), source.id())? {
                                                    qprint!(self, "{}:", self.fmt_path(path));
                                                    println!("FILE HAS VALUE: {:#?}", source.name());

                                                    if reg.value_count_by_id(source.id())? == 1 {
                                                        log::debug!(
                                                            "{}: deleting value {}",
                                                            path.display(),
                                                            source.name()
                                                        );

                                                        println!("COUNT: == 1");
                                                        values_to_delete.push(source.clone());
                                                    }

                                                    if reg.file_has_value(file.id(), dest.id())? {
                                                        println!("FILE ALSO HAS SOURCE");
                                                    } else {
                                                        println!(
                                                            "FILETAG: src: {} dest: {} file: {}",
                                                            source.id(),
                                                            dest.id(),
                                                            file.id()
                                                        );
                                                        if let Err(e) = reg.update_filetag_valueid(
                                                            source.id(),
                                                            dest.id(),
                                                            file.id(),
                                                        ) {
                                                            println!("FAILED updating filetag");
                                                            wutag_error!("{}: {}", bold_entry!(path), e);
                                                            continue;
                                                        }
                                                        println!("UPDATED FILETAG");
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                // wutag_error!(
                                                //     "value {} is not found within the registry: {}",
                                                //     value.bold(),
                                                //     e
                                                // );
                                                continue;
                                            },
                                        }
                                    }
                                },
                                Err(e) => {
                                    wutag_fatal!("failed to find value {}: {}", opts.dest.bold(), e);
                                },
                            }
                        } else {
                            // ╭─────╮
                            // │ Tag │
                            // ╰─────╯
                            println!("REMOVING TAGS");

                            // Make sure the destination tag is found within the registry
                            let dest_by_name = reg.tag_by_name(&opts.dest);
                            match dest_by_name {
                                Ok(dest) => {
                                    println!("FOUND DEST: {:#?}", dest.name());
                                    let fmt_dest = self.fmt_tag(&dest);

                                    for tag in &opts.source {
                                        println!("SOURCE ITER: {:#?}", tag);
                                        // Make sure the source tags are also in the registry
                                        //
                                        // This check isn't necessary, the file could just be checked
                                        // However, this will provide more useful errors
                                        let inner_tag = reg.tag_by_name(tag);
                                        match inner_tag {
                                            Ok(source) => {
                                                println!("FOUND SOURCE: {:#?}", source.name());
                                                let fmt_source = self.fmt_tag(&source);

                                                if source == dest {
                                                    wutag_error!(
                                                        "unable to merge tag {} into itself",
                                                        fmt_dest
                                                    );
                                                    continue;
                                                }

                                                // If the file has the source tag
                                                if reg.file_has_tag(file.id(), source.id())? {
                                                    qprint!(self, "{}:", self.fmt_path(path));
                                                    println!("FILE HAS TAG SOURCE: {}", source.name());

                                                    // If this file is the only one with the tag, delete it
                                                    if reg.tag_count_by_id(source.id())? == 1 {
                                                        log::debug!(
                                                            "{}: deleting tag {}",
                                                            path.display(),
                                                            source.name()
                                                        );
                                                        println!("TAG COUNT == 1: {}", source.name());

                                                        tags_to_delete.push(source.clone());
                                                    }

                                                    // NOTE: Maybe support merging (tag, value) pairs

                                                    // If the file has both the source tag and dest tag
                                                    // then do not update the tag
                                                    if reg.file_has_tag(file.id(), dest.id())? {
                                                        println!("FILE HAS DEST: {}", dest.name());

                                                        qprint!(
                                                            self,
                                                            "\t{} already exists, {} {}",
                                                            fmt_dest,
                                                            r!("X"),
                                                            fmt_source
                                                        );

                                                        if let Err(e) = path.untag(&source) {
                                                            wutag_error!(
                                                                "{}: failure deleting source tag: {}",
                                                                bold_entry!(path),
                                                                e
                                                            );
                                                        }
                                                    } else {
                                                        println!("FILE DOESN'T HAVE DEST: {}", dest.name());
                                                        if let Err(e) = reg.update_filetag_tagid(
                                                            source.id(),
                                                            dest.id(),
                                                            file.id(),
                                                        ) {
                                                            println!("FAILED updating filetag");
                                                            wutag_error!("{}: {}", bold_entry!(path), e);
                                                            continue;
                                                        }

                                                        // Get the actual extended attribute
                                                        if let Err(e) = path.get_tag(&source) {
                                                            wutag_error!(
                                                                "{}: failed to get extended attributes: {}",
                                                                bold_entry!(path),
                                                                e
                                                            );
                                                            continue;
                                                        }

                                                        // TODO: Set correct value
                                                        if let Err(e) =
                                                            path.replace_tag(&source, &dest, None)
                                                        {
                                                            wutag_error!(
                                                                "{}: failed to swap xattr {} {} {}: {}",
                                                                bold_entry!(path),
                                                                fmt_source,
                                                                g!("=>"),
                                                                fmt_dest,
                                                                e
                                                            );
                                                            continue;
                                                        }

                                                        println!(
                                                            "\t{} {} {}",
                                                            fmt_source,
                                                            g!("=>"),
                                                            fmt_dest
                                                        );
                                                    }

                                                    let values = reg.unique_values_by_tag(source.id())?;
                                                    println!("SOURCE VALUES: {:#?}", values);

                                                    for value in values.iter() {
                                                        log::debug!(
                                                            "{}: deleting value {}",
                                                            path.display(),
                                                            value.name()
                                                        );
                                                        println!("DELETING VALUE: {:#?}", value);

                                                        if let Err(e) = reg.delete_value(value.id()) {
                                                            wutag_error!(
                                                                "{}: value {}: {}",
                                                                bold_entry!(path),
                                                                value.name().color(self.base_color).bold(),
                                                                e
                                                            );
                                                            // continue;
                                                        }
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                // FIXME: This occurs if there's a single file with
                                                //        a tag that is being replaced. There needs to
                                                //        be a global object to track these, or don't
                                                //        print an errror
                                                //
                                                wutag_error!(
                                                    "tag {} is not found within the registry: {}",
                                                    tag.bold(),
                                                    e
                                                );
                                                continue;
                                            },
                                        }
                                    }
                                },
                                Err(e) => {
                                    wutag_fatal!("failed to find tag {}: {}", opts.dest.bold(), e);
                                },
                            }
                        }

                        for tag in &tags_to_delete {
                            if let Err(e) = reg.delete_tag(tag.id()) {
                                wutag_error!("{}: tag {}: {}", bold_entry!(path), self.fmt_tag(tag), e);
                            }
                        }

                        for value in &values_to_delete {
                            if let Err(e) = reg.delete_value(value.id()) {
                                wutag_error!(
                                    "{}: value {}: {}",
                                    bold_entry!(path),
                                    value.name().color(self.base_color).bold(),
                                    e
                                );
                            }
                        }
                    }

                    Ok(())
                },
            );
        }

        Ok(())
    }
}
