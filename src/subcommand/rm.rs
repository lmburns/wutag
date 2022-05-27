#![allow(clippy::unnested_or_patterns)]

/// Remove tags/values from other files/tags respectively
use super::{parse_tag_val, App};
use crate::{
    bold_entry,
    filesystem::osstr_to_bytes,
    registry::types::{
        tag::{DirEntryExt, Tag},
        value::Value,
    },
    util::{crawler, fmt_path, glob_builder, regex_builder},
    wutag_error, wutag_info,
};
use anyhow::{anyhow, Context, Result};
use clap::{Args, ValueHint};
use colored::Colorize;
use std::{borrow::Cow, ffi::OsStr, fs, path::PathBuf, sync::Arc};

/// Arguments to the `rm` subcommand
#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct RmOpts {
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
            Delete a tag and its value from the database. If a tag is not connected to a file, \
            and/or a value is not connected to any tag, these items will be removed from the \
            database"
    )]
    pub(crate) pairs: Vec<(String, String)>,

    /// Indicate the item(s) in the given list are values instead of tags
    #[clap(
        name = "values",
        long,
        short = 'V',
        takes_value = false,
        long_help = "By default the item(s) after the pattern are tag(s). To instead act on \
                     values, pass this flag. For example: `wutag rm -V <pattern> <val1> <val2>`"
    )]
    pub(crate) values: bool,

    /// A glob pattern like "*.png" (or regex).
    #[clap(
        name = "pattern",
        takes_value = true,
        required = true,
        value_hint = ValueHint::FilePath,
)]
    pub(crate) pattern: String,

    /// Tags or values (requires --values) to remove from the matching pattern
    #[clap(name = "tags", takes_value = true, required_unless_present = "pairs")]
    pub(crate) tags: Vec<String>,
}

/// Used to display errors for unknown tags.
/// They cannot be formatted with `fmt_tag` because the do not exist
macro_rules! red_entry {
    ($t:ident) => {
        $t.name().red().bold()
    };
}

// XXX: Implement case sensitive
// TODO: Add stdin method for remove

impl App {
    /// `rm` subcommand
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn rm(&mut self, opts: &RmOpts) -> Result<()> {
        // Global will match a glob only against files that are tagged
        // Could add a fixed string option
        log::debug!("RmOpts: {:#?}", opts);

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

        // Delete a Tag from the database
        let delete_tag = |tag: &Tag, path: &PathBuf| -> Result<()> {
            log::debug!("{}: deleting tag {}", path.display(), tag.name());
            if let Err(e) = reg.delete_tag(tag.id()) {
                wutag_error!(
                    "{}: failed to delete tag {}: {}",
                    bold_entry!(path),
                    self.fmt_tag(tag),
                    e
                );

                return Err(anyhow!("anything"));
            }
            Ok(())
        };

        // Delete a Value from the database
        let delete_value = |value: &Value, path: &PathBuf| -> Result<()> {
            log::debug!("{}: deleting value {}", path.display(), value.name());
            if let Err(e) = reg.delete_value(value.id()) {
                wutag_error!(
                    "{}: failed to delete value {}: {}",
                    bold_entry!(path),
                    value.name().color(self.base_color).bold(),
                    e
                );
                return Err(anyhow!("doesn't matter what this says"));
            }

            print!(
                "\t{} {} (V)",
                "X".bold().red(),
                value.name().color(self.base_color).bold()
            );
            Ok(())
        };

        // Remove an extended attribute from a [`PathBuf`]
        let handle_xattr = |tag: &Tag, path: &PathBuf| {
            log::debug!("removing xattr for Tag({})", tag.name());
            if path.get_tag(tag).is_err() {
                wutag_error!(
                    "{}: found ({}) in database, though file has no xattrs",
                    bold_entry!(path),
                    self.fmt_tag(tag)
                );
            } else if let Err(e) = path.untag(tag) {
                wutag_error!("{}: {}", path.display(), e);
            } else {
                print!("\t{} {}", "X".bold().red(), self.fmt_tag(tag));
            }
        };

        if self.global {
            let exclude_pattern = regex_builder(
                self.exclude.join("|").as_str(),
                self.case_insensitive,
                self.case_sensitive,
            );

            for file in reg.files(None)?.iter() {
                let path = &file.path();

                let search_str: Cow<OsStr> = Cow::Owned(path.as_os_str().to_os_string());
                let search_bytes = osstr_to_bytes(search_str.as_ref());
                if !self.exclude.is_empty() && exclude_pattern.is_match(&search_bytes) {
                    continue;
                }

                if let Some(ref ext) = self.extension {
                    if !ext.is_match(&search_bytes) {
                        continue;
                    }
                }

                if re.is_match(&search_bytes) {
                    if !self.quiet {
                        println!("{}:", fmt_path(path, self));
                    }

                    for (tag, value) in &combos {
                        match (
                            tag.is_null_id(),
                            tag.is_null_name(),
                            value.is_null_id(),
                            value.is_null_name(),
                        ) {
                            // Passed: Tag, Value => Found: true, true
                            (false, false, false, false) => {
                                // wutag_info!("== ffff == OK TAG OK VALUE");
                                log::debug!(
                                    "ffff: (Tag => {}), (Value => {})",
                                    tag.name(),
                                    value.name()
                                );

                                let mut values_ = vec![];
                                if let Ok(values) = reg.values_by_tagid(tag.id()) {
                                    for value in values.iter().cloned() {
                                        if reg.value_count_by_id(value.id())? == 1 {
                                            values_.push(value);
                                        } else if !self.quiet {
                                            wutag_info!(
                                                "the value {} is found on other files, therefore \
                                                 won't be deleted",
                                                value.name()
                                            );
                                        }
                                    }
                                }

                                if reg.tag_count_by_id(tag.id())? == 1 {
                                    if delete_tag(tag, path).is_err() {
                                        continue;
                                    }
                                } else if let Err(e) =
                                    reg.delete_filetag(file.id(), tag.id(), value.id())
                                {
                                    wutag_error!(
                                        "{}: failed to delete FileTag {}",
                                        path.display(),
                                        e
                                    );
                                    continue;
                                }

                                // Deal with xattr after database
                                handle_xattr(tag, path);

                                for value in &values_ {
                                    if delete_value(value, path).is_err() {
                                        continue;
                                    }
                                }
                            },

                            // Passed: Tag, Value => Found: false, false
                            (true, false, true, false) => {
                                // wutag_info!("== tftf ==");
                                log::debug!("tftf: (Tag => false), (Value => false)",);

                                wutag_error!(
                                    "tag ({}) and value ({}) are both not found in the registry",
                                    red_entry!(tag),
                                    value.name().color(self.base_color).bold(),
                                );
                                continue;
                            },

                            // Passed: Tag => Found: true
                            // Passed: Tag, Value => Found: true, false
                            (false, false, true, true) | (false, false, true, false) => {
                                // Remove tag

                                if value.is_null_name() {
                                    // wutag_info!("== fftt == OK TAG");
                                    log::debug!("fftt: (Tag => {}), (Value => N/A)", tag.name());
                                } else {
                                    // wutag_info!("== fftf == OK TAG");
                                    log::debug!("fftf: (Tag => {}) (Value => false)", tag.name());
                                    wutag_error!(
                                        "value ({}) is not found in the registry",
                                        value.name().color(self.base_color).bold(),
                                    );
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
                                    if delete_tag(tag, path).is_err() {
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

                                // Deal with xattr after database tag, but before value
                                // so that way 'X <tag> \t X (V) <value>' is printed
                                handle_xattr(tag, path);

                                for value in &values_ {
                                    if delete_value(value, path).is_err() {
                                        continue;
                                    }
                                }
                            },

                            // Passed: Tag => Found: false
                            (true, false, true, true) => {
                                // wutag_info!("== tftt ==");
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
                                // Remove value

                                if tag.is_null_name() {
                                    // wutag_info!("== ttff == OK VALUE");
                                    log::debug!("ttff: (Tag => N/A), (Value => {})", value.name());
                                } else {
                                    // wutag_info!("== tfff == OK VAUE");
                                    log::debug!(
                                        "tfff: (Tag => false), (Value => {})",
                                        value.name()
                                    );

                                    wutag_error!(
                                        "tag ({}) is not found in the registry",
                                        red_entry!(tag)
                                    );

                                    // Decide whether this should be continued
                                    continue;
                                }

                                let tags = reg.tags_by_fileid_valueid(file.id(), value.id())?;

                                // If this value is only found once (on this tag/file)
                                if reg.value_count_by_id(value.id())? == 1 {
                                    // Then go ahead and delete it
                                    if let Err(e) = reg.delete_value_only(value.id()) {
                                        wutag_error!(
                                            "{}: failed to delete value {}: {}",
                                            bold_entry!(path),
                                            value.name().color(self.base_color).bold(),
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
                                        "{}: failed to update value {}: {}",
                                        bold_entry!(path),
                                        value.name().color(self.base_color).bold(),
                                        e
                                    );
                                    continue;
                                }

                                for tag in tags.iter() {
                                    if reg.tag_count_by_id(tag.id())? == 1 {
                                        if delete_tag(tag, path).is_err() {
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

                                    handle_xattr(tag, path);
                                }

                                // What would be a better way to indicate that this is a value?
                                print!(
                                    "\t{} {} (V)",
                                    "X".bold().red(),
                                    value.name().color(self.base_color).bold(),
                                );
                            },

                            // Passed: Value => Found: false
                            (true, true, true, false) => {
                                // wutag_info!("== tttf ==");
                                log::debug!("tttf: (Tag => N/A), (Value => false)",);

                                wutag_error!(
                                    "value ({}) is not found in the registry",
                                    value.name().color(self.base_color).bold(),
                                );
                                continue;
                            },

                            // Passed: => Found:
                            //  - Should only happen if clap somehow accepts empties
                            (true, true, true, true) => {
                                log::debug!("tttt: (Tag => N/A), (Value => N/A)");
                                wutag_error!(
                                    "you shouldn't use empty strings for tag or value names"
                                );
                                continue;
                            },

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

                    if !self.quiet {
                        println!();
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

                    let path = &(|| -> Result<PathBuf> {
                        if self.follow_symlinks
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
                            println!("{}:", fmt_path(path, self));
                        }

                        // TODO: Reduce duplicate code
                        // These are duplicated from above, because the [`InnerConnection`] of the
                        // SQLite database in unable to be shared across threads. Whenever these
                        // closures are created here, they capture the [`Registry`] that has been
                        // cloned inside the asynchronous loop.
                        //
                        // These closures are all created on each file that is found
                        //
                        // This should be fixed; though, it is here for now to reduce duplicate
                        // code. If these are instead functions, each will need to have 4-5
                        // parameters, and some issues arise dealing with lifetimes due to this
                        // being an asynchronous block.

                        let delete_tag = |tag: &Tag| -> Result<()> {
                            log::debug!("{}: deleting tag {}", path.display(), tag.name());
                            if let Err(e) = reg.delete_tag(tag.id()) {
                                wutag_error!(
                                    "{}: failed to delete tag {}: {}",
                                    bold_entry!(path),
                                    self.fmt_tag(tag),
                                    e
                                );

                                return Err(anyhow!("anything"));
                            }
                            Ok(())
                        };

                        let delete_value = |value: &Value| -> Result<()> {
                            log::debug!("{}: deleting value {}", path.display(), value.name());
                            if let Err(e) = reg.delete_value(value.id()) {
                                wutag_error!(
                                    "{}: failed to delete value {}: {}",
                                    bold_entry!(path),
                                    value.name().color(self.base_color).bold(),
                                    e
                                );
                                return Err(anyhow!("doesn't matter what this says"));
                            }

                            print!(
                                "\t{} {} (V)",
                                "X".bold().red(),
                                value.name().color(self.base_color).bold()
                            );
                            Ok(())
                        };

                        // ------------Check------------   --Result--     ----Passed----
                        // Tag id, name = Value id, name => TAG, VALUE => Pass tag, value
                        // Tag id, name = Value name     => TAG        => Pass tag, value
                        // Tag name     = Value id, name => VALUE      => Pass tag, value
                        // Tag name     = Value name     => NOTHING    => Pass tag, value
                        // Tag id, name = Value          => TAG        => Pass tag
                        // Tag name     = Value          => NOTHING    => Pass tag
                        // Tag          = Value id, name => VALUE      => Pass value
                        // Tag          = Value name     => NOTHING    => Pass value

                        for (tag, value) in &combos {
                            match (
                                tag.is_null_id(),
                                tag.is_null_name(),
                                value.is_null_id(),
                                value.is_null_name(),
                            ) {
                                // Passed: Tag, Value => Found: true, true
                                (false, false, false, false) => {
                                    // wutag_info!("== ffff == OK TAG OK VALUE");
                                    log::debug!(
                                        "ffff: (Tag => {}), (Value => {})",
                                        tag.name(),
                                        value.name()
                                    );

                                    let mut values_ = vec![];
                                    if let Ok(values) = reg.values_by_tagid(tag.id()) {
                                        for value in values.iter().cloned() {
                                            if reg.value_count_by_id(value.id())? == 1 {
                                                values_.push(value);
                                            } else if !self.quiet {
                                                wutag_info!(
                                                    "the value {} is found on other files, \
                                                     therefore won't be deleted",
                                                    value.name()
                                                );
                                            }
                                        }
                                    }

                                    if reg.tag_count_by_id(tag.id())? == 1 {
                                        if delete_tag(tag).is_err() {
                                            continue;
                                        }
                                    } else if let Err(e) =
                                        reg.delete_filetag(file.id(), tag.id(), value.id())
                                    {
                                        wutag_error!(
                                            "{}: failed to delete FileTag {}",
                                            path.display(),
                                            e
                                        );
                                        continue;
                                    }

                                    // Deal with xattr after database
                                    handle_xattr(tag, path);

                                    for value in &values_ {
                                        if delete_value(value).is_err() {
                                            continue;
                                        }
                                    }
                                },

                                // Passed: Tag, Value => Found: false, false
                                (true, false, true, false) => {
                                    // wutag_info!("== tftf ==");
                                    log::debug!("tftf: (Tag => false), (Value => false)",);

                                    wutag_error!(
                                        "tag ({}) and value ({}) are both not found in the \
                                         registry",
                                        red_entry!(tag),
                                        value.name().color(self.base_color).bold(),
                                    );
                                    continue;
                                },

                                // Passed: Tag => Found: true
                                // Passed: Tag, Value => Found: true, false
                                (false, false, true, true) | (false, false, true, false) => {
                                    // Remove tag

                                    if value.is_null_name() {
                                        // wutag_info!("== fftt == OK TAG");
                                        log::debug!(
                                            "fftt: (Tag => {}), (Value => N/A)",
                                            tag.name()
                                        );
                                    } else {
                                        // wutag_info!("== fftf == OK TAG");
                                        log::debug!(
                                            "fftf: (Tag => {}) (Value => false)",
                                            tag.name()
                                        );
                                        wutag_error!(
                                            "value ({}) is not found in the registry",
                                            value.name().color(self.base_color).bold(),
                                        );
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
                                        if delete_tag(tag).is_err() {
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

                                    // Deal with xattr after database tag, but before value
                                    // so that way 'X <tag> \t X (V) <value>' is printed
                                    handle_xattr(tag, path);

                                    for value in &values_ {
                                        if delete_value(value).is_err() {
                                            continue;
                                        }
                                    }
                                },

                                // Passed: Tag => Found: false
                                (true, false, true, true) => {
                                    // wutag_info!("== tftt ==");
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
                                    // Remove value

                                    if tag.is_null_name() {
                                        // wutag_info!("== ttff == OK VALUE");
                                        log::debug!(
                                            "ttff: (Tag => N/A), (Value => {})",
                                            value.name()
                                        );
                                    } else {
                                        // wutag_info!("== tfff == OK VAUE");
                                        log::debug!(
                                            "tfff: (Tag => false), (Value => {})",
                                            value.name()
                                        );

                                        wutag_error!(
                                            "tag ({}) is not found in the registry",
                                            red_entry!(tag)
                                        );

                                        // Decide whether this should be continued
                                        continue;
                                    }

                                    let tags = reg.tags_by_fileid_valueid(file.id(), value.id())?;

                                    // If this value is only found once (on this tag/file)
                                    if reg.value_count_by_id(value.id())? == 1 {
                                        // Then go ahead and delete it
                                        if let Err(e) = reg.delete_value_only(value.id()) {
                                            wutag_error!(
                                                "{}: failed to delete value {}: {}",
                                                bold_entry!(path),
                                                value.name().color(self.base_color).bold(),
                                                e
                                            );
                                            continue;
                                        }
                                    } else if let Err(e) =
                                        reg.update_filetag_valueid(value.id(), file.id())
                                    {
                                        // Otherwise, just remove it from this single file
                                        wutag_error!(
                                            "{}: failed to update value {}: {}",
                                            bold_entry!(path),
                                            value.name().color(self.base_color).bold(),
                                            e
                                        );
                                        continue;
                                    }

                                    for tag in tags.iter() {
                                        if reg.tag_count_by_id(tag.id())? == 1 {
                                            if delete_tag(tag).is_err() {
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

                                        handle_xattr(tag, path);
                                    }

                                    // What would be a better way to indicate that this is a value?
                                    print!(
                                        "\t{} {} (V)",
                                        "X".bold().red(),
                                        value.name().color(self.base_color).bold(),
                                    );
                                },

                                // Passed: Value => Found: false
                                (true, true, true, false) => {
                                    // wutag_info!("== tttf ==");
                                    log::debug!("tttf: (Tag => N/A), (Value => false)",);

                                    wutag_error!(
                                        "value ({}) is not found in the registry",
                                        value.name().color(self.base_color).bold(),
                                    );
                                    continue;
                                },

                                // Passed: => Found:
                                //  - Should only happen if clap somehow accepts empties
                                (true, true, true, true) => {
                                    log::debug!("tttt: (Tag => N/A), (Value => N/A)");
                                    wutag_error!(
                                        "you shouldn't use empty strings for tag or value names"
                                    );
                                    continue;
                                },

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
