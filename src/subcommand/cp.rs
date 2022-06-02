//! `cp` - Copy `Tag`s or a `Tag`'s color to another `Tag`

// TODO: Add mv option
// TODO: Add global option to cp

#![allow(unused)]
#![allow(clippy::unnecessary_wraps)]

use super::{parse_tag_val, App};
use crate::{
    bold_entry, err, filesystem as wfs,
    registry::{
        types::{FileTag, Tag, TagValueCombo, Value, ID},
        Error,
    },
    utils::{crawler, fmt, glob_builder, parse_path, regex_builder},
    wutag_error, wutag_fatal,
    xattr::tag::{list_tags, DirEntryExt},
};
use anyhow::{anyhow, Context, Result};
use clap::{Args, ValueHint};
use colored::Colorize;
use rusqlite as rsq;
use std::{borrow::Cow, env, ffi::OsStr, fs, path::PathBuf, sync::Arc};

/// Arguments used for the `cp` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct CpOpts {
    /// Specify an individual tag to copy to the matching file(s)
    #[clap(
        name = "tag",
        long = "tag",
        short = 't',
        takes_value = true,
        long_help = "By default, `cp` will copy all tags and values of those tags to the results \
                     from the query"
    )]
    pub(crate) tag: Option<String>,

    /// Specify any number of tag=value pairs
    #[clap(
        name = "pairs",
        long = "pairs",
        short = 'p',
        takes_value = true,
        conflicts_with_all = &["tag"],
        multiple_occurrences = true,
        parse(try_from_str = parse_tag_val),
        long_help = "Use tag=value pairs to individually specify what the tag's value is",
    )]
    pub(crate) pairs: Vec<(String, String)>,

    // XXX: Implement or remove
    /// Use a glob to match files (must be global)
    #[clap(
        name = "glob",
        short = 'G',
        long = "glob",
        takes_value = false,
        long_help = "Use a glob to match the input path instead of matching files in the local \
                     directory. This argument doesn't do anything, and is a placeholder. If the \
                     global option is used, pattern matching is turned on and file-matching is no \
                     longer used"
    )]
    pub(crate) glob: bool,

    /// Path to the file from which to copy tags from
    #[clap(
        name = "input_path",
        value_hint = ValueHint::FilePath,
        takes_value = false,
        required = true,
        // validator = |t| parse_path(t) // Would be nice to be aware of other options
    )]
    pub(crate) input_path: PathBuf,

    /// A glob, regular expression, or fixed-string
    #[clap(
        name = "pattern",
        takes_value = true,
        required = true,
        value_hint = ValueHint::FilePath,
    )]
    pub(crate) pattern: String,
}

// FEATURE: --only-tags

// TODO: Condense all this duplicate code

impl App {
    /// Copy `Tag`s or a `Tag`'s color to another `Tag`
    pub(crate) fn cp(&mut self, opts: &CpOpts) -> Result<()> {
        log::debug!("CpOpts: {:#?}", opts);

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

        // Maybe try and not have to canonicalize this
        let path = &fs::canonicalize(&opts.input_path).context(format!(
            "{}: failed to canonicalize",
            opts.input_path.display()
        ))?;

        let reg = self.registry.lock().expect("poisoned lock");

        if self.global {
            if !opts.pairs.is_empty() {
                wutag_error!("pairs cannot be used in global mode. Continuing...");
            }

            if opts.tag.is_some() {
                wutag_error!("an individual Tag cannot be given in global mode. Continuing...");
            }

            let exclude_pattern = regex_builder(
                self.exclude.join("|").as_str(),
                self.case_insensitive,
                self.case_sensitive,
            );

            for entry in reg.files(None)?.iter() {
                let search_str: Cow<OsStr> = Cow::Owned(entry.path().as_os_str().to_os_string());
                let search_bytes = wfs::osstr_to_bytes(search_str.as_ref());
                if !self.exclude.is_empty() && exclude_pattern.is_match(&search_bytes) {
                    continue;
                }

                if let Some(ref ext) = self.extension {
                    if !ext.is_match(&search_bytes) {
                        continue;
                    }
                }

                if re.is_match(&search_bytes) {
                    // println!("MATCH: {}", entry.path().display());
                    let entry_path = &entry.path();

                    match reg.filetags_by_fileid(entry.id()) {
                        Ok(filetags) => {
                            log::debug!(
                                "copying all FileTags from {} to {} (globally)",
                                path.display(),
                                entry.path().display()
                            );

                            for ft in filetags.iter() {
                                let mut print_newline = false;

                                let constructed =
                                    FileTag::new(entry.id(), ft.tag_id(), ft.value_id());
                                let tag = reg.tag(ft.tag_id())?;
                                if reg.filetag_exists(&constructed)? {
                                    if ft.value_id().id() == 0 {
                                        wutag_error!(
                                            "{}: already has {}",
                                            bold_entry!(entry_path),
                                            self.fmt_tag(&tag)
                                        );
                                    } else {
                                        let value = reg.value(ft.value_id())?;
                                        wutag_error!(
                                            "{}: already has {}={}",
                                            bold_entry!(entry_path),
                                            self.fmt_tag(&tag),
                                            value.name().color(self.base_color).bold()
                                        );
                                    }
                                    continue;
                                }

                                if let Err(e) = reg.copy_filetag_fileid(ft, entry.id()) {
                                    if ft.value_id().id() == 0 {
                                        wutag_error!(
                                            "{}: failed to copy {}",
                                            bold_entry!(entry_path),
                                            self.fmt_tag(&tag)
                                        );
                                    } else {
                                        let value = reg.value(ft.value_id())?;
                                        wutag_error!(
                                            "{}: failed to copy {}={}",
                                            bold_entry!(entry_path),
                                            self.fmt_tag(&tag),
                                            value.name().color(self.base_color).bold()
                                        );
                                    }
                                    continue;
                                }

                                if let Err(e) = entry_path.tag(&tag) {
                                    wutag_error!("{} {}", e, bold_entry!(entry_path));
                                } else {
                                    log::debug!("{}: writing xattrs", entry_path.display());

                                    if !self.quiet {
                                        print!("\t{} {}", "+".bold().green(), self.fmt_tag(&tag));

                                        if ft.value_id().id() != 0 {
                                            let value = reg.value(ft.value_id())?;
                                            print!(
                                                "={}",
                                                value.name().color(self.base_color).bold()
                                            );
                                        }

                                        print_newline = true;
                                    }
                                }

                                if !self.quiet && print_newline {
                                    println!();
                                }
                            }
                        },
                        Err(_) => {
                            wutag_error!("{}: has no tags", bold_entry!(path),);
                        },
                    }
                }
            }
        } else {
            if let Err(e) = parse_path(path) {
                wutag_error!("{}: {}", bold_entry!(path), e);
            }

            match reg.file_by_path(path) {
                Ok(file) => {
                    match reg.filetags_by_fileid(file.id()) {
                        Ok(filetags) => {
                            if filetags.is_empty() {
                                wutag_fatal!(
                                    "{}: has no tags. Please clean your registry or give this \
                                     file tags",
                                    bold_entry!(path)
                                );
                            }

                            let mut combos = opts
                                .pairs
                                .iter()
                                .map(|(t, v)| -> Result<TagValueCombo> {
                                    let tag = reg.tag_by_name(t).map(|tag| {
                                        if filetags.iter().any(|ft| ft.tag_id() == tag.id()) {
                                            tag
                                        } else {
                                            wutag_fatal!(
                                                "{}: does not have tag {}",
                                                bold_entry!(path),
                                                tag
                                            );
                                        }
                                    })?;

                                    let values = reg.values_by_fileid_tagid(file.id(), tag.id())?;

                                    let value = reg.value_by_name(v, false).map(|value| {
                                        if values.iter().any(|inner| *inner == value) {
                                            value
                                        } else {
                                            wutag_fatal!(
                                                "{}: {} does not have value {}",
                                                bold_entry!(path),
                                                self.fmt_tag(&tag),
                                                value
                                            );
                                        }
                                    })?;

                                    let combo = TagValueCombo::new(tag.id(), value.id());

                                    Ok(combo)
                                })
                                .collect::<Result<Vec<_>>>()?;

                            let mut opt_tag = opts.tag.as_ref().map_or_else(Vec::new, |tag| {
                                if let Ok(found) = reg.tag_by_name(tag) {
                                    if filetags.iter().any(|ft| ft.tag_id() == found.id()) {
                                        vec![TagValueCombo::new(found.id(), ID::null())]
                                    } else {
                                        wutag_fatal!(
                                            "{}: does not have tag {}",
                                            bold_entry!(path),
                                            tag
                                        );
                                    }
                                } else {
                                    wutag_fatal!(
                                        "{}: tag not found in registry {}",
                                        bold_entry!(path),
                                        tag
                                    );
                                }
                            });

                            combos.append(&mut opt_tag);

                            drop(reg);

                            crawler(
                                &Arc::new(re),
                                &Arc::new(self.clone()),
                                |entry: &ignore::DirEntry| {
                                    let reg = self.registry.lock().expect("poisoned lock");

                                    // The destination files
                                    let path = &self.resolve_symlink(entry.path())?;

                                    if !self.quiet {
                                        println!("{}:", self.fmt_path(entry.path()));
                                    }

                                    let mut dest = reg.file_by_path(entry.path());

                                    if dest.is_err() {
                                        log::debug!("{}: inserting file", entry.path().display());
                                        dest = reg.insert_file(entry.path());
                                    }

                                    let dest = dest?;
                                    let dest_path = &dest.path();
                                    let mut print_newline = false;

                                    // This means to copy all tags and values
                                    if combos.is_empty() {
                                        log::debug!(
                                            "copying all FileTags from {} to {}",
                                            path.display(),
                                            dest.path().display()
                                        );
                                        for ft in filetags.iter() {
                                            let constructed =
                                                FileTag::new(dest.id(), ft.tag_id(), ft.value_id());
                                            let tag = reg.tag(ft.tag_id())?;
                                            if reg.filetag_exists(&constructed)? {
                                                if ft.value_id().id() == 0 {
                                                    wutag_error!(
                                                        "{}: already has {}",
                                                        bold_entry!(entry),
                                                        self.fmt_tag(&tag)
                                                    );
                                                } else {
                                                    let value = reg.value(ft.value_id())?;
                                                    wutag_error!(
                                                        "{}: already has {}={}",
                                                        bold_entry!(entry),
                                                        self.fmt_tag(&tag),
                                                        value.name().color(self.base_color).bold()
                                                    );
                                                }
                                                continue;
                                            }

                                            if let Err(e) = reg.copy_filetag_fileid(ft, dest.id()) {
                                                if ft.value_id().id() == 0 {
                                                    wutag_error!(
                                                        "{}: failed to copy {}",
                                                        bold_entry!(entry),
                                                        self.fmt_tag(&tag)
                                                    );
                                                } else {
                                                    let value = reg.value(ft.value_id())?;
                                                    wutag_error!(
                                                        "{}: failed to copy {}={}",
                                                        bold_entry!(entry),
                                                        self.fmt_tag(&tag),
                                                        value.name().color(self.base_color).bold()
                                                    );
                                                }
                                                continue;
                                            }

                                            if let Err(e) = dest_path.tag(&tag) {
                                                wutag_error!("{} {}", e, bold_entry!(dest_path));
                                            } else {
                                                log::debug!(
                                                    "{}: writing xattrs",
                                                    dest_path.display()
                                                );

                                                if !self.quiet {
                                                    print!(
                                                        "\t{} {}",
                                                        "+".bold().green(),
                                                        self.fmt_tag(&tag)
                                                    );

                                                    if ft.value_id().id() != 0 {
                                                        let value = reg.value(ft.value_id())?;
                                                        print!(
                                                            "={}",
                                                            value
                                                                .name()
                                                                .color(self.base_color)
                                                                .bold()
                                                        );
                                                    }

                                                    print_newline = true;
                                                }
                                            }
                                        }
                                    } else {
                                        log::debug!(
                                            "copying selected FileTags from {} to {}",
                                            path.display(),
                                            dest.path().display()
                                        );
                                        for combo in &combos {
                                            let constructed = FileTag::new(
                                                dest.id(),
                                                combo.tag_id(),
                                                combo.value_id(),
                                            );
                                            let tag = reg.tag(combo.tag_id())?;

                                            if reg.filetag_exists(&constructed)? {
                                                if combo.value_id().id() == 0 {
                                                    wutag_error!(
                                                        "{}: already has {}",
                                                        bold_entry!(entry),
                                                        self.fmt_tag(&tag),
                                                    );
                                                } else {
                                                    let value = reg.value(combo.value_id())?;
                                                    wutag_error!(
                                                        "{}: already has {}={}",
                                                        bold_entry!(entry),
                                                        self.fmt_tag(&tag),
                                                        value.name().color(self.base_color).bold()
                                                    );
                                                }
                                                continue;
                                            }

                                            let to_insert = FileTag::new(
                                                file.id(),
                                                combo.tag_id(),
                                                combo.value_id(),
                                            );
                                            if let Err(e) =
                                                reg.copy_filetag_fileid(&to_insert, dest.id())
                                            {
                                                if combo.value_id().id() == 0 {
                                                    wutag_error!(
                                                        "{}: failed to copy {}",
                                                        bold_entry!(entry),
                                                        self.fmt_tag(&tag),
                                                    );
                                                } else {
                                                    let value = reg.value(combo.value_id())?;
                                                    wutag_error!(
                                                        "{}: failed to copy {}={}",
                                                        bold_entry!(entry),
                                                        self.fmt_tag(&tag),
                                                        value.name().color(self.base_color).bold()
                                                    );
                                                }
                                                continue;
                                            }

                                            if let Err(e) = dest_path.tag(&tag) {
                                                wutag_error!("{} {}", e, bold_entry!(dest_path));
                                            } else {
                                                log::debug!(
                                                    "{}: writing xattrs",
                                                    dest_path.display()
                                                );

                                                if !self.quiet {
                                                    print!(
                                                        "\t{} {}",
                                                        "+".bold().green(),
                                                        self.fmt_tag(&tag),
                                                    );

                                                    if combo.value_id().id() != 0 {
                                                        let value = reg.value(combo.value_id())?;
                                                        print!(
                                                            "={}",
                                                            value
                                                                .name()
                                                                .color(self.base_color)
                                                                .bold()
                                                        );
                                                    }

                                                    print_newline = true;
                                                }
                                            }
                                        }
                                    }

                                    if !self.quiet && print_newline {
                                        println!();
                                    }

                                    Ok(())
                                },
                            );
                        },
                        Err(_) => {
                            wutag_error!("{}: has no tags", bold_entry!(path),);
                        },
                    }
                },
                Err(_) => {
                    wutag_error!("{}: not found in the registry", bold_entry!(path));
                },
            }
        }

        Ok(())
    }
}
