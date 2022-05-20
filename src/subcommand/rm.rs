#![allow(unused)]
#![allow(clippy::unnested_or_patterns)]

/// Remove subcommand
use super::{debug_registry_path, parse_tag_val, App};
use crate::{
    bold_entry, err,
    filesystem::osstr_to_bytes,
    registry::{
        types::{
            filetag::FileTag,
            tag::{DirEntryExt, Tag, TagValueCombo},
            value::Value,
            ID,
        },
        Error,
    },
    util::{crawler, fmt_err, fmt_path, fmt_tag, glob_builder, regex_builder},
    wutag_error, wutag_info,
};
use anyhow::{Context, Result};
use clap::{Args, ValueHint};
use colored::Colorize;
use itertools::Itertools;
use std::{borrow::Cow, ffi::OsStr, fs, path::PathBuf, sync::Arc};
use wutag_core::tag::list_tags;

/// Arguments to the `rm` subcommand
#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct RmOpts {
    /// Follow symlinks before removing tags and/or values
    #[clap(
        name = "follow-symlinks",
        long,
        short = 'f',
        takes_value = false,
        long_help = "Should the symlink be dereferenced before the tag or value is removed from \
                     the file"
    )]
    pub(crate) follow_symlinks: bool,

    // XXX: Implement
    /// Remove all tags and/or values on a file
    #[clap(
        name = "all",
        long,
        short = 'a',
        takes_value = false,
        long_help = "Remove all tags on the results from the search. A list of tags is not \
                     required if this argument is present. By default this will delete the tag \
                     from the database if no other files have these tags"
    )]
    pub(crate) all: bool,

    /// Specify any number of tag=value pairs to delete
    #[clap(
        name = "pairs",
        long,
        short = 'p',
        takes_value = true,
        conflicts_with_all = &["values", "tags"],
        multiple_occurrences = true,
        parse(try_from_str = parse_tag_val),
        long_help = "\
            Delete a tag and its value. If a value is left dangling (i.e., doesn't correspond \
            to a tag), it will remain in the database if `keep_dangling` is true in the \
            configuration file. Otherwise, it will be removed",
    )]
    pub(crate) pairs: Vec<(String, String)>,

    /// Indicate the item(s) in the given list are values intead of tags
    #[clap(
        name = "values",
        long,
        short = 'V',
        takes_value = false,
        long_help = "By default the item(s) after the pattern are tag(s). To instead act on \
                     values, pass this flag. For example: `wutag rm -V <pattern> <val1> <val2>`"
    )]
    pub(crate) values: bool,

    /// When deleting tags from the database, delete the tag's values as well
    #[clap(
        name = "with-values",
        long,
        short = 'w',
        takes_value = false,
        conflicts_with_all = &["values"],
        long_help = "Delete a tag's values when it is deleted. This option conflicts with \
            `--untag`, as it requires the `tag=value` pair to be given together for the \
            pair to be untagged from the file"
    )]
    pub(crate) with_values: bool,

    /// A glob pattern like "*.png" (or regex).
    #[clap(name = "pattern", takes_value = true, required = true)]
    pub(crate) pattern: String,

    /// Tags or values (requires --values) to remove from the matching pattern
    #[clap(
        name = "tags",
        takes_value = true,
        conflicts_with = "all",
        // required_unless_present_any = &["all"]
    )]
    pub(crate) tags: Vec<String>,
}

macro_rules! red_entry {
    ($t:ident) => {
        $t.name().red().bold()
    };
}

// FEATURE: Pass one value for many tags
// XXX: Implement case sensitive
// XXX: REMOVE `keep_dangling`
// MAYBE: Switch default from delete to untag

// TODO: Remove untag/delete, act as before

// TODO: Check if xattr is present but not in the registry
// TODO: Check remove tag, if it has a value, reset value_id to 0

// TODO: Add stdin method for remove

impl App {
    /// `rm` subcommand
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn rm(&mut self, opts: &RmOpts) -> Result<()> {
        // Global will match a glob only against files that are tagged
        // Could add a fixed string option
        log::debug!("RmOpts: {:#?}", opts);
        debug_registry_path(&self.registry);

        // println!("RmOpts: {:#?}", opts);

        let re = regex_builder(
            &{
                if self.pat_regex {
                    String::from(&opts.pattern)
                } else if self.fixed_string {
                    regex::escape(&opts.pattern)
                } else {
                    glob_builder(&opts.pattern)
                }
            },
            self.case_insensitive,
            self.case_sensitive,
        );
        log::debug!("Compiled pattern: {re}");

        let reg = self.registry.lock().expect("poisoned lock");
        let sensitive = !self.case_insensitive && self.case_sensitive;

        let mut combos = opts
            .pairs
            .iter()
            .map(|(t, v)| {
                (
                    reg.tag_by_name(t).unwrap_or_else(|_| Tag::null(t)),
                    reg.value_by_name(v, sensitive)
                        .unwrap_or_else(|_| Value::new_noid(v)),
                )
            })
            .collect::<Vec<_>>();

        let mut tags = opts
            .tags
            .iter()
            .map(|item| {
                if opts.values {
                    (
                        Tag::null(""),
                        reg.value_by_name(item, sensitive)
                            .unwrap_or_else(|_| Value::new_noid(item)),
                    )
                } else {
                    (
                        reg.tag_by_name(item).unwrap_or_else(|_| Tag::null(item)),
                        Value::new_noid(""),
                    )
                }
            })
            .collect::<Vec<_>>();

        combos.append(&mut tags);

        if self.global {
            // let ctags = opts.tags.iter().collect::<Vec<_>>();
            // let exclude_pattern = regex_builder(
            //     self.exclude.join("|").as_str(),
            //     self.case_insensitive,
            //     self.case_sensitive,
            // );
            //
            // for (&id, entry) in self.oregistry.clone().list_entries_and_ids()
            // {     let search_str: Cow<OsStr> =
            // Cow::Owned(entry.path().as_os_str().to_os_string());
            //     let search_bytes = osstr_to_bytes(search_str.as_ref());
            //     if !self.exclude.is_empty() &&
            // exclude_pattern.is_match(&search_bytes) {
            //         continue;
            //     }
            //
            //     if let Some(ref ext) = self.extension {
            //         if !ext.is_match(&search_bytes) {
            //             continue;
            //         }
            //     }
            //
            //     if re.is_match(&search_bytes) {
            //         list_tags(entry.path())
            //             .map(|tags| {
            //                 tags.iter().fold(Vec::new(), |mut acc, tag| {
            //                     acc.push((
            //                         ctags.iter().find(|c| **c ==
            // &tag.to_string()),
            // tag.clone(),                     ));
            //                     acc
            //                 })
            //             })
            //             .unwrap_or_default()
            //             .iter()
            //             .for_each(|(search, realtag)| {
            //                 if search.is_some() {
            //                     // println!("SEARCH: {:?} REAL: {:?}",
            // search, realtag);
            // self.oregistry.untag_by_name(search.unwrap(), id);
            //                     if !self.quiet {
            //                         println!(
            //                             "{}:",
            //                             fmt_path(entry.path(),
            // self.base_color, self.ls_colors)
            // );                     }
            //
            //                     if let Err(e) =
            // realtag.remove_from(entry.path()) {
            // err!('\t', e, entry);                     } else if
            // !self.quiet {                         print!("\t{}
            // {}", "X".bold().red(), fmt_tag_old(realtag));
            //                     }
            //
            //                     if !self.quiet {
            //                         println!();
            //                     }
            //                 }
            //             });
            //     }
            //     log::debug!("Saving registry...");
            // self.save_registry();
            // }
        } else {
            drop(reg);

            // untag requires tag=value to be removed
            // delete only requires tag
            crawler(
                &Arc::new(re),
                &Arc::new(self.clone()),
                opts.follow_symlinks,
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisoned lock");

                    let path = &(|| -> Result<PathBuf> {
                        if (opts.follow_symlinks || self.follow_symlinks)
                            && fs::symlink_metadata(entry.path())
                                .ok()
                                .map_or(false, |f| f.file_type().is_symlink())
                        {
                            log::debug!("{}: resolving symlink", entry.path().display());
                            return fs::canonicalize(entry.path()).context(format!(
                                "{}: failed to canonicalize",
                                entry.path().display()
                            ));
                        }

                        return Ok(entry.path().to_path_buf());
                    })()?;

                    if let Ok(file) = reg.file_by_path(path) {
                        if !self.quiet {
                            println!("{}:", fmt_path(path, self.base_color, self.ls_colors));
                        }

                        let all_tags = reg.tags_for_file(&file)?;

                        // ------------Check------------   --Result--     ----Passed----
                        // Tag id, name = Value id, name => TAG, VALUE => Pass tag, value
                        // Tag id, name = Value name     => TAG        => Pass tag, value
                        // Tag name     = Value id, name => VALUE      => Pass tag, value
                        // Tag name     = Value name     => NOTHING    => Pass tag, value
                        // Tag id, name = Value          => TAG        => Pass tag
                        // Tag name     = Value          => NOTHING    => Pass tag
                        // Tag          = Value id, name => VALUE      => Pass value
                        // Tag          = Value name     => NOTHING    => Pass value

                        // println!("ALL TAGS: {:#?}", all_tags);
                        for (tag, value) in &combos {
                            // println!("TAG: {:#?}", tag);
                            // println!("VALUE: {:#?}", value);

                            let xattr = path.get_tag(&tag);
                            // println!("XATTR: {:#?}", xattr);
                            // println!("TAG: {:#?}", tag);
                            // println!("VALUE: {:#?}", value);

                            match (
                                tag.is_null_id(),
                                tag.is_null_name(),
                                value.is_null_id(),
                                value.is_null_name(),
                            ) {
                                // Passed: Tag, Value => Found: true, true
                                (false, false, false, false) => {
                                    // TODO: Remove tag
                                    // TODO: Remove value
                                    wutag_info!("== ffff == OK TAG OK VALUE");
                                    log::debug!(
                                        "ffff: (Tag => {}), (Value => {})",
                                        tag.name(),
                                        value.name()
                                    );
                                },
                                // DONE:
                                // Passed: Tag, Value => Found: false, false
                                (true, false, true, false) => {
                                    wutag_info!("== tftf ==");
                                    log::debug!("tftf: (Tag => false), (Value => false)",);

                                    wutag_error!(
                                        "tag ({}) and value ({}) are both not found in the \
                                         registry",
                                        red_entry!(tag),
                                        value.name().bold()
                                    );
                                    continue;
                                },
                                // DONE:
                                // Passed: Tag => Found: true
                                // Passed: Tag, Value => Found: true, false
                                (false, false, true, true) | (false, false, true, false) => {
                                    // TODO: Remove tag

                                    if value.is_null_name() {
                                        wutag_info!("== fftt == OK TAG");
                                        log::debug!(
                                            "fftt: (Tag => {}), (Value => N/A)",
                                            tag.name()
                                        );
                                    } else {
                                        wutag_info!("== fftf == OK TAG");
                                        log::debug!(
                                            "fftf: (Tag => {}) (Value => false)",
                                            tag.name()
                                        );
                                        wutag_error!(
                                            "value ({}) is not found in the registry",
                                            value.name().bold()
                                        );
                                    }

                                    if all_tags.is_empty() {
                                        // This should be unreachable
                                        wutag_error!(
                                            "{}: something went horribly wrong. A tagged file is \
                                             registering as having no tags. Please report to \
                                             lmburns/wutag",
                                            bold_entry!(path)
                                        );
                                        return Ok(());
                                    }

                                    let mut values_ = vec![];
                                    if let Ok(values) = reg.values_by_tagid(tag.id()) {
                                        for value in values.iter().cloned() {
                                            if reg.value_count_by_id(value.id())? == 1 {
                                                values_.push(value);
                                            }
                                        }
                                    }

                                    if reg.tag_count_by_id(tag.id())? == 1 {
                                        log::debug!(
                                            "{}: deleting tag {}",
                                            path.display(),
                                            tag.name()
                                        );

                                        if let Err(e) = reg.delete_tag(tag.id()) {
                                            wutag_error!(
                                                "{}: failed to delete tag {}: {}",
                                                bold_entry!(path),
                                                fmt_tag(tag),
                                                e
                                            );
                                            continue;
                                        }
                                    } else if let Err(e) =
                                        reg.delete_filetag_by_fileid_tagid(file.id(), tag.id())
                                    {
                                        wutag_error!(
                                            "{}: failed to delete filetag {}",
                                            path.display(),
                                            e
                                        );
                                        continue;
                                    }

                                    for value in values_ {
                                        if let Err(e) = reg.delete_value(value.id()) {
                                            wutag_error!(
                                                "{}: failed to delete value {}: {}",
                                                bold_entry!(path),
                                                value.name(),
                                                e
                                            );
                                            // continue;
                                        }
                                    }

                                    // Deal with xattr after database
                                    if xattr.is_err() {
                                        wutag_error!(
                                            "{}: found ({}) in database, though file has no xattrs",
                                            bold_entry!(entry),
                                            fmt_tag(tag)
                                        );
                                    } else if let Err(e) = path.untag(tag) {
                                        wutag_error!("{}: {}", path.display(), e);
                                    } else {
                                        print!("\t{} {}", "X".bold().red(), fmt_tag(tag));
                                    }
                                },
                                // DONE:
                                // Passed: Tag => Found: false
                                (true, false, true, true) => {
                                    wutag_info!("== tftt ==");
                                    log::debug!("tftt: (Tag => false), (Value => N/A)",);

                                    wutag_error!(
                                        "tag ({}) is not found in the registry",
                                        red_entry!(tag)
                                    );
                                    continue;
                                },
                                // Passed: Value => Found: true
                                // Passed: Tag, Value => Found: false, true
                                (true, true, false, false) | (true, false, false, false) => {
                                    // TODO: Remove value

                                    if tag.is_null_name() {
                                        wutag_info!("== ttff == OK VALUE");
                                        log::debug!(
                                            "ttff: (Tag => N/A), (Value => {})",
                                            value.name()
                                        );
                                    } else {
                                        wutag_info!("== tfff == OK VAUE");
                                        log::debug!(
                                            "tfff: (Tag => false), (Value => {})",
                                            value.name()
                                        );

                                        wutag_error!(
                                            "tag ({}) is not found in the registry",
                                            red_entry!(tag)
                                        );
                                    }

                                    let tags = reg.tags_by_fileid_valueid(file.id(), value.id())?;

                                    // If this value is only found once (on this tag/file)
                                    if reg.value_count_by_id(value.id())? == 1 {
                                        log::debug!(
                                            "{}: deleting value {}",
                                            path.display(),
                                            value.name()
                                        );

                                        // Then go ahead and delete it
                                        if let Err(e) = reg.delete_value(value.id()) {
                                            wutag_error!(
                                                "{}: failed to delete value {}: {}",
                                                bold_entry!(path),
                                                value.name(),
                                                e
                                            );
                                            continue;
                                        }

                                        // Otherwise, just remove it from this
                                        // single file
                                    } else if let Err(e) =
                                        reg.update_filetag_valueid(value.id(), file.id())
                                    {
                                        wutag_error!(
                                            "{}: failed to update value {}",
                                            bold_entry!(path),
                                            value.name()
                                        );
                                        continue;
                                    }

                                    for tag in tags.iter() {
                                        if reg.tag_count_by_id(tag.id())? == 1 {
                                            log::debug!(
                                                "{}: implicitly deleting tag {}",
                                                path.display(),
                                                tag.name()
                                            );

                                            if let Err(e) = reg.delete_tag(tag.id()) {
                                                wutag_error!(
                                                    "{}: failed to delete tag {}: {}",
                                                    bold_entry!(path),
                                                    fmt_tag(tag),
                                                    e
                                                );
                                                continue;
                                            }
                                        } else if let Err(e) =
                                            reg.delete_filetag_by_fileid_tagid(file.id(), tag.id())
                                        {
                                            wutag_error!(
                                                "{}: failed to delete filetag {}",
                                                path.display(),
                                                e
                                            );
                                            continue;
                                        }

                                        if path.get_tag(&tag).is_err() {
                                            wutag_error!(
                                                "{}: found ({}) in database, though file has no \
                                                 xattrs",
                                                bold_entry!(entry),
                                                fmt_tag(tag)
                                            );
                                        } else if let Err(e) = path.untag(tag) {
                                            wutag_error!("{}: {}", path.display(), e);
                                        } else {
                                            print!("\t{} {}", "X".bold().red(), fmt_tag(tag));
                                        }
                                    }

                                    // What would be a better way to indicate that this is a value?
                                    print!("\t{} {} (V)", "X".bold().red(), value.name().bold());
                                },
                                // DONE:
                                // Passed: Value => Found: false
                                (true, true, true, false) => {
                                    wutag_info!("== tttf ==");
                                    log::debug!("tttf: (Tag => N/A), (Value => false)",);

                                    wutag_error!(
                                        "value ({}) is not found in the registry",
                                        value.name().bold()
                                    );
                                    continue;
                                },
                                // DONE:
                                // Passed: => Found:
                                //  - Should only happen if clap somehow accepts empties
                                (true, true, true, true) => {
                                    log::debug!("tttt: (Tag => N/A), (Value => N/A)");
                                    wutag_error!(
                                        "you shouldn't use empty strings for tag or value names"
                                    );
                                    continue;
                                },
                                // DONE:
                                _ => {
                                    wutag_error!(
                                        "you shouldn't use empty strings for tag or value names. \
                                        If the issue persists, please report to \
                                        https://github.com/lmburns/wutag"
                                    );
                                    continue;
                                },
                            }
                        }
                    } else {
                        log::trace!("{}: skipping", path.display());
                    }

                    if !self.quiet {
                        println!();
                    }

                    Ok(())
                },
            );
        }

        Ok(())
    }
}
