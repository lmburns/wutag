#![allow(unused)]

use super::{debug_registry_path, App};
use crate::{
    bold_entry, err,
    registry::{
        common::hash,
        types::{
            file::{File, FileId},
            filetag::FileTag,
            tag::{Tag, TagId, TagValueCombo},
            value::{Value, ValueId},
            ID,
        },
    },
    util::{collect_stdin_paths, fmt_err, fmt_path, fmt_tag, glob_builder, reg_ok, regex_builder},
    wutag_debug, wutag_error, wutag_fatal, wutag_info, wutag_warning,
};
use anyhow::{anyhow, Context, Result};
use clap::{Args, ValueHint};
use colored::{Color, Colorize};
use itertools::Itertools;
use std::{
    error::Error,
    str::FromStr,
    sync::{Arc, Mutex},
};
use wutag_core::{
    color::parse_color,
    tag::{DirEntryExt, Tag as WTag, DEFAULT_COLOR},
};

/// Options used for the `set` subcommand
#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct Set2Opts {
    /// Clear the tags on the match(es) before the new one(s) are set
    #[clap(
        name = "clear",
        long,
        short = 'c',
        long_help = "This is like a 'reset' for the file(s) matching the pattern. They are \
                     cleared of all tags before the new ones are set"
    )]
    pub(crate) clear: bool,

    /// Explicitly select color for tag
    #[clap(
        long,
        short = 'C',
        takes_value = true,
        validator = |t| parse_color(t)
                            .map_err(|_| "must be a valid hex color")
                            .map(|_| ())
                            .map_err(|e| e.to_string()),
        long_help = "\
        Specify a color for a tag. Without this flag the colors are taken from \
        the configuration file, and if there aren't any there, default colors are selected. \
        A color can start with '0x', '#', or nothing. For example '#FF00FF', '0xFF00FF', \
        'FF00FF'. The following colors are also accepted as they are and with a 'bright' prefix:\n \
            \tblack, red, green, yellow, blue, magenta, purple, cyan, white
                     "
    )]
    pub(crate) color: Option<String>,

    /// Arguments are expected to be passed through stdin
    #[clap(
        name = "stdin",
        long,
        short = 's',
        long_help = "Arguments are expected to be passed through stdin; however, this argument is \
                     not explicitly required to have arguments be accepted through stdin. A \
                     simple `<cmd> | wutag <opts> set <tag>` can be used"
    )]
    pub(crate) stdin: bool,

    /// Explicitly apply given tags even if they're implicit
    #[clap(
        name = "explicit",
        long,
        short = 'e',
        takes_value = false,
        long_help = "Change an implicitly set tag to an explicitly set one"
    )]
    pub(crate) explicit: bool,

    // TODO:
    /// Apply tags to the result of a query instead of a pattern match
    #[clap(
        name = "query",
        long,
        short = 'Q',
        takes_value = true,
        alias = "where",
        long_help = "Instead of using a glob or regular expression to search for files, using a \
                     query. See the syntax by TODO"
    )]
    pub(crate) query: Option<String>,

    /// Specify any number of tag=value pairs
    #[clap(
        name = "pairs",
        long,
        short = 'p',
        takes_value = true,
        conflicts_with = "value",
        multiple_occurrences = true,
        parse(try_from_str = parse_tag_val),
        long_help = "Use tag=value pairs to individually specify what the tag's value \
            is. If the -V/--value option is used, that value is applied to all mentioned tags",
    )]
    pub(crate) pairs: Vec<(String, String)>,

    /// Specify a value to set all the tag(s) to
    #[clap(
        name = "value",
        long,
        short = 'V',
        takes_value = true,
        long_help = "Set a value to each of the matching tags. To set different values for \
                     different tags, use the tag=value syntax"
    )]
    pub(crate) value: Option<String>,

    /// A glob pattern like "*.png".
    #[clap(
        required_unless_present = "stdin",
        value_hint = ValueHint::FilePath,
    )]
    pub(crate) pattern: String,

    /// Tag or tags to set on the result of the pattern
    #[clap(
        name = "tags",
        conflicts_with = "pairs",
        long_help = "Specify a list of tags to set the results of the pattern matching or STDIN \
                     to. This option is required unless `-p`/`--pairs` is used, which allows \
                     specifying `tag`, `value` pairs"
    )]
    pub(crate) tags: Vec<String>,
}

// TODO: Use max-depth, quiet,
// TODO: new: explicit, force

impl App {
    /// Set tags on a file
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn set2(&mut self, opts: &Set2Opts) -> Result<()> {
        log::debug!("SetOpts: {:#?}", opts);
        debug_registry_path(&self.registry);

        println!("{:#?}", opts.clone());

        let mut tags = opts.tags.clone();
        if (opts.stdin || atty::isnt(atty::Stream::Stdin)) && atty::is(atty::Stream::Stdout) {
            tags.push(opts.pattern.clone());
        }

        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else if self.fixed_string {
            regex::escape(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        log::debug!("Is a TTY?: {}", atty::is(atty::Stream::Stdout));
        log::debug!("Compiled pattern: {re}");

        let reg = self.registry.lock().expect("poisioned lock");

        // A vector of <Tag, Value> combinations
        // let mut pairs = vec![];

        /// A vector of <TagValueCombo>, containing each Tag id and Value id
        let mut combos = opts
            .pairs
            .iter()
            .map(|(t, v)| -> Result<TagValueCombo> {
                let tag = reg.tag_by_name(t).or_else(|_| {
                    let tag = opts.color.as_ref().map_or_else(
                        || Tag::random_noid(t, &self.colors),
                        |color| Tag::new_noid(t, color),
                    );

                    reg.insert_tag(&tag)
                })?;

                let value = reg
                    .value_by_name(v, false)
                    .or_else(|_| reg.insert_value(t))?;

                // Specifying combo here prevents having to clone each tag and value
                let combo = TagValueCombo::new(tag.id(), value.id());

                // pairs.push((tag, value));

                Ok(combo)
            })
            .collect::<Result<Vec<_>>>()?;

        let tags = &opts
            .tags
            .iter()
            .map(|t| {
                reg.tag_by_name(t).or_else(|_| {
                    let tag = opts.color.as_ref().map_or_else(
                        || Tag::random_noid(t, &self.colors),
                        |color| Tag::new_noid(t, color),
                    );

                    reg.insert_tag(&tag)
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Extend the combos, setting value ID to 0
        let mut remapped = tags
            .iter()
            .map(|t| TagValueCombo::new(t.id(), ID::null()))
            .collect::<Vec<_>>();

        combos.append(&mut remapped);

        // Drop the lock, otherwise this closure loop below will hang forever
        drop(reg);

        if (opts.stdin || atty::isnt(atty::Stream::Stdin)) && atty::is(atty::Stream::Stdout) {
            log::debug!("Using STDIN");
            for entry in &collect_stdin_paths(&self.base_dir) {
                if !self.quiet {
                    println!("{}:", fmt_path(entry, self.base_color, self.ls_colors));
                }

                // for tag in &tags {
                //     if opts.clear {
                //         log::debug!(
                //             "Using registry in threads: {}",
                //             self.oregistry.path.display()
                //         );
                //         if let Some(id) = self.oregistry.find_entry(entry) {
                //             self.oregistry.clear_entry(id);
                //         }
                //         match entry.has_tags() {
                //             Ok(has_tags) =>
                //                 if has_tags {
                //                     if let Err(e) = entry.clear_tags() {
                //                         wutag_error!("\t{} {}", e,
                // bold_entry!(entry));                     }
                //                 },
                //             Err(e) => {
                //                 wutag_error!("{} {}", e, bold_entry!(entry));
                //             },
                //         }
                //     }
                //
                //     if let Err(e) = entry.tag(tag) {
                //         log::debug!("Error setting tag for: {}",
                // entry.display());         if !opts.quiet {
                //             wutag_error!("{} {}", e, bold_entry!(entry));
                //         }
                //     } else {
                //         log::debug!("Setting tag for new entry: {}",
                // entry.display());         let entry =
                // EntryData::new(entry)?;         let id =
                // self.oregistry.add_or_update_entry(entry);
                //         self.oregistry.tag_entry(tag, id);
                //         if !self.quiet {
                //             print!("\t{} {}", "+".bold().green(),
                // fmt_tag(tag));         }
                //     }
                // }
                // if !self.quiet {
                //     println!();
                // }
            }
        } else {
            reg_ok(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisioned lock");

                    // println!("ENTRY: {:#?}", entry);

                    if !self.quiet {
                        println!(
                            "{}:",
                            fmt_path(entry.path(), self.base_color, self.ls_colors)
                        );
                    }

                    let path = entry.path();
                    let path_d = path.display();

                    // Check if file path exists
                    let mut file = reg.file_by_path(path);
                    if file.is_err() {
                        log::debug!("{}: creating fingerprint", path.display());

                        // Possibly check --force
                        let hash = hash::blake3_hash(path, None)?;

                        if self.show_duplicates && !self.quiet {
                            let count = reg.file_count_by_hash(hash.to_string())?;

                            if count != 0 {
                                wutag_warning!(
                                    "{} is a duplicate entry\n{}: {}",
                                    path.display(),
                                    "b3sum".magenta(),
                                    hash.to_string()
                                );
                            }
                        }

                        log::debug!("{}: inserting file", path_d);
                        file = reg.insert_file(path);
                    }

                    if opts.clear {
                        log::debug!("{}: clearing tags", path_d);
                    }

                    let file = file?;

                    if !opts.explicit {
                        log::debug!("{}: determining existing file tags", path_d);
                        let existing_ft =
                            reg.filetags_by_fileid(file.id(), false).map_err(|e| {
                                anyhow!("{}: could not determine file tags: {}", path_d, e)
                            })?;

                        let new_impls =
                            reg.implications_for(&reg.txn()?, &combos).map_err(|e| {
                                anyhow!("{}: couldn't determine implied tags: {}", path_d, e)
                            })?;

                        let mut revised = vec![];
                        for pair in &combos {
                            if existing_ft.any(|ft| {
                                ft.tag_id() == pair.tag_id() && ft.value_id() == pair.value_id()
                            }) || new_impls.implies(pair)
                            {
                                continue;
                            }

                            revised.push(pair.clone());
                        }

                        combos = revised;
                    }

                    for pair in &combos {
                        if let Err(e) = reg.insert_filetag(&FileTag::new(
                            file.id(),
                            pair.tag_id(),
                            pair.value_id(),
                        )) {
                            return Err(anyhow!("{}: could not apply tags: {}", path_d, e));
                        }
                    }

                    Ok(())
                },
            );
        }

        Ok(())
    }
}

/// Parse a single tag-value pair
fn parse_tag_val<T>(s: &str) -> Result<(T, T), Box<dyn Error + Send + Sync + 'static>>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    use std::io::{Error as IoError, ErrorKind};
    let tagval = format!("expected {}", "tag=value".green());

    let pos = s.find('=').ok_or_else(|| {
        format!(
            "{}: no `{}` found in `{}`",
            tagval,
            "=".yellow(),
            s.magenta()
        )
    })?;

    let tag = &s[..pos];
    let value = &s[pos + 1..];

    let io_err = |s: String| -> Box<IoError> { Box::new(IoError::new(ErrorKind::InvalidInput, s)) };

    match (tag.is_empty(), value.is_empty()) {
        (true, true) => Err(io_err(format!("{}: tag or value cannot be empty", tagval))),
        (true, false) => Err(io_err(format!("{}: tag cannot be empty", tagval))),
        (false, true) => Err(io_err(format!("{}: value cannot be empty", tagval))),
        (false, false) => Ok((tag.parse()?, value.parse()?)),
    }
}
