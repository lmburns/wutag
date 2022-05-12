#![allow(unused)]

use super::{debug_registry_path, parse_tag_val, App};
use crate::{
    bold_entry, err,
    registry::{
        common::hash,
        types::{
            file::{File, FileId},
            filetag::FileTag,
            tag::{DirEntryExt, Tag, TagId, TagValueCombo},
            value::{Value, ValueId},
            ID,
        },
        Registry,
    },
    util::{collect_stdin_paths, fmt_err, fmt_path, fmt_tag, glob_builder, reg_ok, regex_builder},
    wutag_debug, wutag_error, wutag_fatal, wutag_info, wutag_warning,
};
use anyhow::{anyhow, Context, Result};
use clap::{Args, ValueHint};
use colored::{Color, Colorize};
use itertools::Itertools;
use rusqlite as rsq;
use std::{
    error::Error,
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard},
};
use wutag_core::{
    color::parse_color,
    tag::{Tag as WTag, DEFAULT_COLOR},
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

    /// Force the creation of a new tag
    #[clap(
        name = "force",
        long,
        short = 'f',
        takes_value = false,
        long_help = "Force a tag to be created even if the file does not exist"
    )]
    pub(crate) force: bool,

    // TODO: Implement
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

    // TODO: Implement
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

// TODO: Deal with symlinks
// TODO: Write extended attribute to file
// TODO: Use max-depth, explicit
// TODO: new: force

// TODO: Condense all this duplicate code

impl App {
    /// Set tags on a file
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn set2(&mut self, opts: &Set2Opts) -> Result<()> {
        log::debug!("SetOpts: {:#?}", opts);
        debug_registry_path(&self.registry);

        let mut tags = opts.tags.clone();
        if (opts.stdin || atty::isnt(atty::Stream::Stdin)) && atty::is(atty::Stream::Stdout) {
            tags.push(opts.pattern.clone());
        }

        println!("SETOPTS: {:#?}", opts);

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

        /// A vector of <TagValueCombo>, containing each Tag id and Value id
        let mut combos = opts
            .pairs
            .iter()
            .map(|(t, v)| -> Result<TagValueCombo> {
                let tag = reg.tag_by_name(t).or_else(|_| {
                    println!("CREATING NEW TAG");
                    let tag = opts.color.as_ref().map_or_else(
                        || Tag::random_noid(t, &self.colors),
                        |color| Tag::new_noid(t, color),
                    );

                    reg.insert_tag(&tag)
                })?;

                let value = reg
                    .value_by_name(v, false)
                    .or_else(|_| reg.insert_value(v))?;

                let combo = TagValueCombo::new(tag.id(), value.id());

                Ok(combo)
            })
            .collect::<Result<Vec<_>>>()?;

        // TODO: Prevent tag from being added if the file doesn't exist
        //       configuration option: only_create_tags_if_valid
        let tags = &opts
            .tags
            .iter()
            .map(|t| {
                reg.tag_by_name(t).or_else(|_| {
                    println!("CREATING NEW TAG");
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

        // let mut combos = opts
        //     .pairs
        //     .iter()
        //     .map(|(t, v)| {
        //         let tag = reg.tag_by_name(t).unwrap_or_else(|_| {
        //             opts.color.as_ref().map_or_else(
        //                 || Tag::random_noid(t, &self.colors),
        //                 |color| Tag::new_noid(t, color),
        //             )
        //         });
        //
        //         let value = reg
        //             .value_by_name(v, false)
        //             .unwrap_or_else(|_| Value::new(ID::null(), String::from(v)));
        //
        //         // TagValueCombo::new(tag.id(), value.id())
        //         (tag, value)
        //     })
        //     .collect::<Vec<_>>();
        //
        // let mut tags = opts
        //     .tags
        //     .iter()
        //     .map(|t| {
        //         reg.tag_by_name(t).unwrap_or_else(|_| {
        //             opts.color.as_ref().map_or_else(
        //                 || Tag::random_noid(t, &self.colors),
        //                 |color| Tag::new_noid(t, color),
        //             )
        //         })
        //
        //         // let value = Value::new(ID::null(), String::from(""));
        //         // TagValueCombo::new(tag.id(), ID::null())
        //     })
        //     .collect::<Vec<_>>();

        // Extend the combos, setting value ID to 0
        // combos.append(&mut tags);

        if (opts.stdin || atty::isnt(atty::Stream::Stdin)) && atty::is(atty::Stream::Stdout) {
            log::debug!("Using STDIN");
            for entry in &collect_stdin_paths(&self.base_dir) {
                if !self.quiet {
                    println!("{}:", fmt_path(entry, self.base_color, self.ls_colors));
                }

                let path_d = entry.display();

                let mut file = reg.file_by_path(entry);
                if file.is_err() {
                    log::debug!("{}: creating fingerprint", path_d);

                    // Possibly check --force
                    let hash = hash::blake3_hash(entry, None)?;

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
                    file = reg.insert_file(entry);
                }

                let file = file?;

                if opts.clear {
                    log::debug!("{}: clearing tags", path_d);

                    let ftags = reg.tags_for_file(&file)?;

                    for t in ftags.iter() {
                        // TODO: Check whether implications need deleted when > 1

                        // If the tag has values
                        if let Ok(values) = reg.values_by_tagid(t.id()) {
                            for value in values.iter() {
                                if reg.value_count_by_id(value.id())? == 1 {
                                    reg.delete_value(value.id())?;
                                } else {
                                    reg.delete_filetag(&FileTag::new(
                                        file.id(),
                                        t.id(),
                                        value.id(),
                                    ))?;
                                }
                                if reg.tag_count_by_id(t.id())? == 1 {
                                    reg.delete_tag(t.id())?;
                                }
                            }
                        // If the tag is only connected to this file
                        } else if reg.tag_count_by_id(t.id())? == 1 {
                            for pair in &combos {
                                if t.id() != pair.tag_id() {
                                    reg.delete_tag(t.id())?;
                                }
                            }
                        } else {
                            reg.delete_filetag(&FileTag::new(file.id(), t.id(), ID::null()))?;
                        }

                        match entry.has_tags() {
                            Ok(has_tags) =>
                                if has_tags {
                                    if let Err(e) = entry.clear_tags() {
                                        err!('\t', e, entry);
                                    }
                                },
                            Err(e) => {
                                err!(e, entry);
                            },
                        }
                    }
                }

                if !opts.explicit {
                    log::debug!("{}: determining existing file tags", path_d);
                    let existing_ft = reg
                        .filetags_by_fileid(&reg.txn()?, file.id(), false)
                        .map_err(|e| anyhow!("{}: could not determine file tags: {}", path_d, e))?;

                    let new_impls = reg.implications_for(&reg.txn()?, &combos).map_err(|e| {
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
                    if let Err(e) =
                        reg.insert_filetag(&FileTag::new(file.id(), pair.tag_id(), pair.value_id()))
                    {
                        if let Some(rsq::Error::StatementChangedRows(n)) =
                            e.downcast_ref::<rsq::Error>()
                        {
                            if n == &0 {
                                return Err(anyhow!(
                                    "duplicate entry: path: {}, tag: {}",
                                    path_d,
                                    fmt_tag(&reg.tag(pair.tag_id())?),
                                ));
                            }
                        }

                        return Err(anyhow!("{}: could not apply tags: {}", path_d, e));
                    }

                    let tag = reg.tag(pair.tag_id())?;
                    if let Err(e) = entry.tag(&tag) {
                        log::debug!("Error setting tag for: {}", path_d);
                        if !self.quiet {
                            err!('\t', e, entry);
                        }
                    } else {
                        log::debug!("Setting tag for new entry: {}", path_d);
                        // TODO: Create entry here?

                        if !self.quiet {
                            print!("\t{} {}", "+".bold().green(), fmt_tag(&tag));

                            if pair.value_id().id() != 0 {
                                let value = reg.value(pair.value_id())?;
                                print!("={}", value.name().color(self.base_color).bold());
                            }
                        }
                    }
                }

                if !self.quiet {
                    println!();
                }
            }
        } else {
            // Drop the lock, otherwise this closure loop below will hang forever
            drop(reg);

            reg_ok(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    let reg = self.registry.lock().expect("poisioned lock");

                    if !self.quiet {
                        println!(
                            "{}:",
                            fmt_path(entry.path(), self.base_color, self.ls_colors)
                        );
                    }

                    let path = entry.path();
                    let path_d = path.display();

                    // Check if file path exists in the database
                    let mut file = reg.file_by_path(path);
                    if file.is_err() {
                        log::debug!("{}: creating fingerprint", path_d);

                        // Possibly check --force
                        let hash = hash::blake3_hash(path, None)?;

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

                    if opts.clear {
                        log::debug!("{}: clearing tags", path_d);

                        let ftags = reg.tags_for_file(&file)?;

                        for t in ftags.iter() {
                            // TODO: Check whether implications need deleted when > 1

                            // If the tag has values
                            if let Ok(values) = reg.values_by_tagid(t.id()) {
                                for value in values.iter() {
                                    if reg.value_count_by_id(value.id())? == 1 {
                                        reg.delete_value(value.id())?;
                                    } else {
                                        reg.delete_filetag(&FileTag::new(
                                            file.id(),
                                            t.id(),
                                            value.id(),
                                        ))?;
                                    }
                                    if reg.tag_count_by_id(t.id())? == 1 {
                                        reg.delete_tag(t.id())?;
                                    }
                                }
                            // If the tag is only connected to this file
                            } else if reg.tag_count_by_id(t.id())? == 1 {
                                for pair in &combos {
                                    if t.id() != pair.tag_id() {
                                        reg.delete_tag(t.id())?;
                                    }
                                }
                            } else {
                                reg.delete_filetag(&FileTag::new(file.id(), t.id(), ID::null()))?;
                            }

                            match entry.has_tags() {
                                Ok(has_tags) =>
                                    if has_tags {
                                        if let Err(e) = entry.clear_tags() {
                                            err!('\t', e, entry);
                                        }
                                    },
                                Err(e) => {
                                    err!(e, entry);
                                },
                            }
                        }
                    }

                    if !opts.explicit {
                        log::debug!("{}: determining existing file tags", path_d);
                        let existing_ft = reg
                            .filetags_by_fileid(&reg.txn()?, file.id(), false)
                            .map_err(|e| {
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
                            if let Some(rsq::Error::StatementChangedRows(n)) =
                                e.downcast_ref::<rsq::Error>()
                            {
                                if n == &0 {
                                    return Err(anyhow!(
                                        "duplicate entry: path: {}, tag: {}",
                                        path_d,
                                        fmt_tag(&reg.tag(pair.tag_id())?),
                                    ));
                                }
                            }

                            return Err(anyhow!("{}: could not apply tags: {}", path_d, e));
                        }

                        let tag = reg.tag(pair.tag_id())?;
                        if let Err(e) = entry.tag(&tag) {
                            log::debug!("Error setting tag for: {}", path_d);
                            if !self.quiet {
                                err!('\t', e, entry);
                            }
                        } else {
                            log::debug!("Setting tag for new entry: {}", path_d);
                            // TODO: Create entry here?

                            if !self.quiet {
                                print!("\t{} {}", "+".bold().green(), fmt_tag(&tag));

                                if pair.value_id().id() != 0 {
                                    let value = reg.value(pair.value_id())?;
                                    print!("={}", value.name().color(self.base_color).bold());
                                }
                            }
                        }
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

    // fn do_set_action<T: DirEntryExt>(&self, opts: &Set2Opts, entry: T, combos:
    // &mut Vec<TagValueCombo>) {     let reg =
    // self.registry.lock().expect("poisioned lock");
    //
    //     let path = entry.path();
    //     let path_d = path.display();
    //     let mut file = reg.file_by_path(path);
    //
    //     if file.is_err() {
    //         log::debug!("{}: creating fingerprint", path_d);
    //
    //         // Possibly check --force
    //         let hash = hash::blake3_hash(path, None)?;
    //
    //         if self.show_duplicates && !self.quiet &&
    // reg.file_count_by_hash(hash.to_string())? != 0         {
    //             wutag_warning!(
    //                 "{} is a duplicate entry\n{}: {}",
    //                 path_d,
    //                 "b3sum".magenta(),
    //                 hash.to_string()
    //             );
    //         }
    //
    //         log::debug!("{}: inserting file", path_d);
    //         file = reg.insert_file(path);
    //     }
    //
    //     let file = file?;
    //
    //     if opts.clear {
    //         log::debug!("{}: clearing tags", path_d);
    //
    //         let ftags = reg.tags_for_file(&file)?;
    //
    //         for t in ftags.iter() {
    //             // TODO: Check whether implications need deleted when > 1
    //
    //             // If the tag has values
    //             if let Ok(values) = reg.values_by_tagid(t.id()) {
    //                 for value in values.iter() {
    //                     if reg.value_count_by_id(value.id())? == 1 {
    //                         reg.delete_value(value.id())?;
    //                     } else {
    //                         reg.delete_filetag(&FileTag::new(file.id(), t.id(),
    // value.id()))?;                     }
    //                     if reg.tag_count_by_id(t.id())? == 1 {
    //                         reg.delete_tag(t.id())?;
    //                     }
    //                 }
    //             // If the tag is only connected to this file
    //             } else if reg.tag_count_by_id(t.id())? == 1 {
    //                 for pair in combos {
    //                     if t.id() != pair.tag_id() {
    //                         reg.delete_tag(t.id())?;
    //                     }
    //                 }
    //             } else {
    //                 reg.delete_filetag(&FileTag::new(file.id(), t.id(),
    // ID::null()))?;             }
    //
    //             match entry.has_tags() {
    //                 Ok(has_tags) =>
    //                     if has_tags {
    //                         if let Err(e) = entry.clear_tags() {
    //                             err!('\t', e, entry);
    //                         }
    //                     },
    //                 Err(e) => {
    //                     err!(e, entry);
    //                 },
    //             }
    //         }
    //     }
    //
    //     if !opts.explicit {
    //         log::debug!("{}: determining existing file tags", path_d);
    //         let existing_ft = reg
    //             .filetags_by_fileid(&reg.txn()?, file.id(), false)
    //             .map_err(|e| anyhow!("{}: could not determine file tags: {}",
    // path_d, e))?;
    //
    //         let new_impls = reg
    //             .implications_for(&reg.txn()?, &combos)
    //             .map_err(|e| anyhow!("{}: couldn't determine implied tags: {}",
    // path_d, e))?;
    //
    //         let mut revised = vec![];
    //         for pair in combos {
    //             if existing_ft
    //                 .any(|ft| ft.tag_id() == pair.tag_id() && ft.value_id() ==
    // pair.value_id())                 || new_impls.implies(pair)
    //             {
    //                 continue;
    //             }
    //
    //             revised.push(pair.clone());
    //         }
    //
    //         combos = &mut revised;
    //     }
    //
    //     for pair in combos {
    //         if let Err(e) =
    //             reg.insert_filetag(&FileTag::new(file.id(), pair.tag_id(),
    // pair.value_id()))         {
    //             if let Some(rsq::Error::StatementChangedRows(n)) =
    // e.downcast_ref::<rsq::Error>() {                 if n == &0 {
    //                     return Err(anyhow!(
    //                         "duplicate entry: path: {}, tag: {}",
    //                         path_d,
    //                         fmt_tag(&reg.tag(pair.tag_id())?),
    //                     ));
    //                 }
    //             }
    //
    //             return Err(anyhow!("{}: could not apply tags: {}", path_d, e));
    //         }
    //
    //         let tag = reg.tag(pair.tag_id())?;
    //         if let Err(e) = entry.tag(&tag) {
    //             log::debug!("Error setting tag for: {}", path_d);
    //             if !self.quiet {
    //                 err!('\t', e, entry);
    //             }
    //         } else {
    //             log::debug!("Setting tag for new entry: {}", path_d);
    //             // TODO: Create entry here?
    //
    //             if !self.quiet {
    //                 print!("\t{} {}", "+".bold().green(), fmt_tag(&tag));
    //
    //                 if pair.value_id().id() != 0 {
    //                     let value = reg.value(pair.value_id())?;
    //                     print!("={}",
    // value.name().color(self.base_color).bold());                 }
    //             }
    //         }
    //     }
    //
    //     if !self.quiet {
    //         println!();
    //     }
    //
    //     Ok(())
    // }
}
