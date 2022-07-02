#![allow(unused)]
/// Set a tag or tag and value on the result of a regular expression or glob
use super::{parse_tag_val, App};
use crate::{
    bold_entry, qprint,
    registry::{
        common::hash,
        types::{FileTag, Tag, TagValueCombo, Value, ID},
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

/// Options used for the `set` subcommand
#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct SetOpts {
    /// Clear the tags on the match(es) before the new one(s) are set
    #[clap(
        name = "clear",
        long = "clear",
        short = 'c',
        long_help = "This is like a 'reset' for the file(s) matching the pattern. They are cleared of all \
                     tags before the new ones are applied"
    )]
    pub(crate) clear: bool,

    /// Explicitly select color for tag
    #[clap(
        name = "color",
        long = "color",
        short = 'C',
        takes_value = true,
        validator = |t| parse_color(t)
                            .map_err(|_| "must be a valid hex color")
                            .map(|_| ())
                            .map_err(|e| e.to_string()),
        long_help = "\
        Specify a color for a tag. Without this flag the colors are taken from \
        the configuration file. If there aren't any in the configuration file, default colors \
        are used. A color can start with '0x', '#', or nothing. For example '#FF00FF', '0xFF00FF', \
        'FF00FF'. The following colors, as well as the same colors with a 'bright' prefix are also \
        valid:\n \tblack, red, green, yellow, blue, magenta, purple, cyan, white
                     "
    )]
    pub(crate) color: Option<String>,

    /// Arguments are expected to be passed through stdin
    #[clap(
        name = "stdin",
        long = "stdin",
        short = 's',
        long_help = "Arguments are expected to be passed through stdin; however, this argument is not \
                     explicitly required to have arguments be accepted through stdin. A simple `<cmd> | \
                     wutag <opts> set <tag>` can be used"
    )]
    pub(crate) stdin: bool,

    // TODO: Implement
    // /// Apply tags to the result of a query instead of a pattern match
    // #[clap(
    //     name = "query",
    //     long,
    //     short = 'Q',
    //     takes_value = true,
    //     alias = "where",
    //     long_help = "Instead of using a glob or regular expression to search for files, using a
    // \                  query. See the syntax by TODO"
    // )]
    // pub(crate) query: Option<String>,

    // If the -V/--value option is used, that value is applied to all mentioned tags
    /// Specify any number of tag=value pairs
    #[clap(
        name = "pairs",
        long = "pairs",
        short = 'p',
        takes_value = true,
        conflicts_with = "value",
        multiple_occurrences = true,
        parse(try_from_str = parse_tag_val),
        long_help = "Use tag=value pairs to individually specify what the tag's value \
            is",
    )]
    pub(crate) pairs: Vec<(String, String)>,

    // TODO: Implement
    /// Specify a value to set all the tag(s) to
    #[clap(
        name = "value",
        long = "value",
        short = 'V',
        takes_value = true,
        long_help = "Set a value to each of the matching tags. To set different values for different \
                     tags, use the tag=value syntax"
    )]
    pub(crate) value: Option<String>,

    /// A glob, regular expression, or fixed-string
    #[clap(
        name = "pattern",
        takes_value = true,
        required_unless_present = "stdin",
        value_hint = ValueHint::FilePath,
    )]
    pub(crate) pattern: String,

    /// Tag or tags to set on the result of the pattern
    #[clap(
        name = "tags",
        conflicts_with = "pairs",
        long_help = "Specify a list of tags to set the results of the pattern matching or STDIN to. This \
                     option is required unless `-p`/`--pairs` is used, which allows specifying `tag`, \
                     `value` pairs"
    )]
    pub(crate) tags: Vec<String>,
}

// TEST: Multiple tag=value pairs
// TEST: -d  <dir> set ...

// FEATURE: Pass one value for many tags

// TODO: Condense all this duplicate code
// TODO: Write xattr as tag=value
// TODO: Check file permissions

impl App {
    /// Set tags on a file
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn set(&mut self, opts: &SetOpts) -> Result<()> {
        log::debug!("SetOpts: {:#?}", opts);

        let mut tags = opts.tags.clone();
        if (opts.stdin || atty::isnt(atty::Stream::Stdin)) && atty::is(atty::Stream::Stdout) {
            tags.push(opts.pattern.clone());
        }

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
        log::debug!("Is a TTY?: {}", atty::is(atty::Stream::Stdout));

        let reg = self.registry.lock().expect("poisoned lock");

        let mut combos = opts
            .pairs
            .iter()
            .map(|(t, v)| {
                (
                    reg.tag_by_name(t).unwrap_or_else(|_| Tag::null(t)),
                    reg.value_by_name(v).unwrap_or_else(|_| Value::new_noid(v)),
                )
            })
            .collect::<Vec<_>>();

        let mut tags = opts
            .tags
            .iter()
            .map(|t| {
                (
                    reg.tag_by_name(t).unwrap_or_else(|_| Tag::null(t)),
                    Value::new_noid(""),
                )
            })
            .collect::<Vec<_>>();

        combos.append(&mut tags);

        if (opts.stdin || atty::isnt(atty::Stream::Stdin)) && atty::is(atty::Stream::Stdout) {
            log::debug!("Using STDIN");
            for entry in &collect_stdin_paths(&self.base_dir) {
                let path = &self.resolve_symlink(entry.path())?;
                let path_d = path.display();
                qprint!(self, "{}:", self.fmt_path(path));

                // Check if file path exists in the database
                let mut file = reg.file_by_path(path);

                if file.is_err() {
                    // Possibly check --force
                    log::debug!("{}: creating fingerprint", path_d);
                    let hash = hash::blake3_hash(&path, None)?;

                    if self.show_duplicates && !self.quiet && reg.file_count_by_hash(hash.to_string())? != 0
                    {
                        wutag_warning!(
                            "{} is a duplicate entry\n{}: {}",
                            path_d,
                            "b3sum".magenta(),
                            hash.to_string()
                        );
                    }

                    log::debug!("{}: inserting file", path_d);
                    file = reg.insert_file(path);
                }

                let file = file?;
                let path = &file.path();
                let path_d = path.display();

                if opts.clear {
                    log::debug!("{}: clearing tags", path_d);

                    let files_tags = reg.tags_for_file(&file)?;

                    for tag in files_tags.iter() {
                        // let values = reg.unique_values_by_tag(tag.id())?;
                        let values = reg.values_by_fileid_tagid(file.id(), tag.id())?;

                        if reg.tag_count_by_id(tag.id())? == 1 {
                            reg.delete_tag_only(tag.id())?;
                        }

                        if values.is_empty() {
                            reg.delete_filetag_only(file.id(), tag.id(), ID::null())?;
                        } else {
                            for value in values.iter() {
                                if reg.value_count_by_id(value.id())? == 1 {
                                    reg.delete_value_only(value.id())?;
                                }

                                reg.delete_filetag_only(file.id(), tag.id(), value.id())?;
                            }
                        }

                        match path.has_tags_or_values() {
                            Ok(has_tags) =>
                                if has_tags {
                                    // This also clears values
                                    if let Err(e) = path.clear_tags() {
                                        wutag_error!("{}: {}", bold_entry!(path), e);
                                    }
                                },
                            Err(e) => {
                                wutag_error!("{}: {}", bold_entry!(path), e);
                            },
                        }
                    }
                }

                // Collecting these in a vector to print later makes it look better
                let mut duplicate_errors = vec![];
                // Try and do better about tracking whether newline should be added
                // This assumes an error
                let mut print_newline = false;

                for (mut tag, mut value) in combos.iter().cloned() {
                    // Tag was not found within the registry
                    // Check again, because it could've been cleared
                    if reg.tag(tag.id()).is_err() {
                        log::debug!("creating new tag: {}", tag.name());
                        let t = opts.color.as_ref().map_or_else(
                            || Tag::random_noid(tag.name(), &self.colors),
                            |color| Tag::new_noid(tag.name(), color),
                        );

                        tag = reg.insert_tag(&t)?;
                    }

                    // Value was not found in registry and is actually passed
                    if value.is_null_id() && !value.is_null_name() {
                        log::debug!("creating new value: {}", value.name());
                        value = reg.insert_value(value)?;
                    }

                    let ft = FileTag::new(file.id(), tag.id(), value.id());
                    if let Err(e) = reg.insert_filetag(&ft) {
                        if let Some(rsq::Error::StatementChangedRows(n)) = e.downcast_ref::<rsq::Error>() {
                            if let Err(e) = path.get_tag(tag.name()) {
                                wutag_error!(
                                    "{}: found in registry, though file has no xattrs: {}",
                                    bold_entry!(path),
                                    e
                                );

                                if let Err(e) = path.tag(&tag, Some(&value)) {
                                    wutag_error!("{}: {}", bold_entry!(path), e);
                                }

                                // if !value.is_null_id() {
                                //     if let Err(e) = path.value(&tag, &value)
                                // {
                                //         wutag_error!("{}: {}",
                                // bold_entry!(path), e);
                                //     }
                                // }
                            }

                            // Don't return an error so the xattrs can be written
                            if n == &0 {
                                duplicate_errors.push(format!(
                                    "{}: duplicate entry with tag: {}",
                                    bold_entry!(path),
                                    self.fmt_tag(&reg.tag(tag.id())?),
                                ));
                            }

                            continue;
                        }

                        return Err(anyhow!("{}: could not apply tags: {}", path_d, e));
                    }

                    // Deal with xattr after database
                    if let Err(e) = path.tag(&tag, Some(&value)) {
                        wutag_error!("{} {}", e, bold_entry!(path));
                    } else {
                        log::debug!("{}: writing xattrs", path_d);
                        if !self.quiet {
                            print!("\t{} {}", "+".bold().green(), self.fmt_tag(&tag));

                            if !value.is_null_id() {
                                // Set the value
                                // if let Err(e) = path.value(&tag, &value) {
                                //     wutag_error!("{}: {}", bold_entry!(path), e);
                                // }

                                let value = reg.value(value.id())?;
                                print!("={}", value.name().color(self.base_color).bold());
                            }

                            print_newline = true;
                        }
                    }
                }

                if !self.quiet && print_newline {
                    println!();
                }

                for error in &duplicate_errors {
                    wutag_error!("{error}");
                }
            }
        } else {
            // Drop the lock, otherwise this closure loop below will hang forever
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

                    // Check if file path exists in the database
                    let mut file = reg.file_by_path(path);

                    if file.is_err() {
                        // Possibly check --force
                        log::debug!("{}: creating fingerprint", path_d);
                        let hash = hash::blake3_hash(&path, None)?;

                        if self.show_duplicates
                            && !self.quiet
                            && reg.file_count_by_hash(hash.to_string())? != 0
                        {
                            wutag_warning!(
                                "{} is a duplicate entry\n{}: {}",
                                path_d,
                                "b3sum".magenta(),
                                hash.to_string()
                            );
                        }

                        log::debug!("{}: inserting file", path_d);
                        file = reg.insert_file(path);
                    }

                    let file = file?;
                    let path = &file.path();
                    let path_d = path.display();

                    if opts.clear {
                        log::debug!("{}: clearing tags", path_d);

                        let files_tags = reg.tags_for_file(&file)?;

                        for tag in files_tags.iter() {
                            // let values = reg.unique_values_by_tag(tag.id())?;
                            let values = reg.values_by_fileid_tagid(file.id(), tag.id())?;

                            if reg.tag_count_by_id(tag.id())? == 1 {
                                reg.delete_tag_only(tag.id())?;
                            }

                            if values.is_empty() {
                                reg.delete_filetag_only(file.id(), tag.id(), ID::null())?;
                            } else {
                                for value in values.iter() {
                                    if reg.value_count_by_id(value.id())? == 1 {
                                        reg.delete_value_only(value.id())?;
                                    }

                                    reg.delete_filetag_only(file.id(), tag.id(), value.id())?;
                                }
                            }

                            match path.has_tags_or_values() {
                                Ok(has_tags) =>
                                    if has_tags {
                                        // This also clears values
                                        if let Err(e) = path.clear_tags() {
                                            wutag_error!("{}: {}", bold_entry!(path), e);
                                        }
                                    },
                                Err(e) => {
                                    wutag_error!("{}: {}", bold_entry!(path), e);
                                },
                            }
                        }
                    }

                    // Collecting these in a vector to print later makes it look better
                    let mut duplicate_errors = vec![];
                    // Try and do better about tracking whether newline should be added
                    // This assumes an error
                    let mut print_newline = false;

                    for (mut tag, mut value) in combos.iter().cloned() {
                        // Tag was not found within the registry
                        // Check again, because it could've been cleared
                        if reg.tag(tag.id()).is_err() {
                            log::debug!("creating new tag: {}", tag.name());
                            let t = opts.color.as_ref().map_or_else(
                                || Tag::random_noid(tag.name(), &self.colors),
                                |color| Tag::new_noid(tag.name(), color),
                            );

                            tag = reg.insert_tag(&t)?;
                        }

                        // Value was not found in registry and is actually passed
                        if value.is_null_id() && !value.is_null_name() {
                            log::debug!("creating new value: {}", value.name());
                            value = reg.insert_value(value)?;
                        }

                        let ft = FileTag::new(file.id(), tag.id(), value.id());
                        if let Err(e) = reg.insert_filetag(&ft) {
                            if let Some(rsq::Error::StatementChangedRows(n)) =
                                e.downcast_ref::<rsq::Error>()
                            {
                                if let Err(e) = path.get_tag(tag.name()) {
                                    wutag_error!(
                                        "{}: found in registry, though file has no xattrs: {}",
                                        bold_entry!(path),
                                        e
                                    );

                                    if let Err(e) = path.tag(&tag, Some(&value)) {
                                        wutag_error!("{}: {}", bold_entry!(path), e);
                                    }

                                    // if !value.is_null_id() {
                                    //     if let Err(e) = path.value(&tag,
                                    // &value) {
                                    //         wutag_error!("{}: {}",
                                    // bold_entry!(path), e);
                                    //     }
                                    // }
                                }

                                // Don't return an error so the xattrs can be written
                                if n == &0 {
                                    duplicate_errors.push(format!(
                                        "{}: duplicate entry with tag: {}",
                                        bold_entry!(path),
                                        self.fmt_tag(&reg.tag(tag.id())?),
                                    ));
                                }

                                continue;
                            }

                            return Err(anyhow!("{}: could not apply tags: {}", path_d, e));
                        }

                        // Deal with xattr after database
                        if let Err(e) = path.tag(&tag, Some(&value)) {
                            wutag_error!("{} {}", e, bold_entry!(path));
                        } else {
                            log::debug!("{}: writing xattrs", path_d);
                            if !self.quiet {
                                print!("\t{} {}", "+".bold().green(), self.fmt_tag(&tag));

                                if !value.is_null_id() {
                                    // Set the value
                                    // if let Err(e) = path.value(&tag, &value) {
                                    //     wutag_error!("{}: {}", bold_entry!(path), e);
                                    // }

                                    let value = reg.value(value.id())?;
                                    print!("={}", value.name().color(self.base_color).bold());
                                }

                                print_newline = true;
                            }
                        }
                    }

                    if !self.quiet && print_newline {
                        println!();
                    }

                    for error in &duplicate_errors {
                        wutag_error!("{error}");
                    }

                    Ok(())
                },
            );
        }

        Ok(())
    }
}
