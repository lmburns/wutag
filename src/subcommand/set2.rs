#![allow(unused)]

use super::{debug_registry_path, App};
use crate::{
    bold_entry, err,
    registry::{
        common::hash,
        types::{
            file::{File, FileId},
            filetag::FileTag,
            tag::{Tag as DbTag, TagId, TagValueCombo},
            value::{Value, ValueId},
        },
    },
    util::{collect_stdin_paths, fmt_err, fmt_path, fmt_tag, glob_builder, reg_ok, regex_builder},
    wutag_debug, wutag_error, wutag_fatal, wutag_info, wutag_warning,
};
use anyhow::{Context, Result};
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
    tag::{DirEntryExt, Tag, DEFAULT_COLOR},
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

    /// Specify any number of `tag`=`value` pairs
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
    pub(crate) pairs: Option<Vec<(String, String)>>,

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

        let pairs = opts.pairs.as_ref().map(|pairs| {
            pairs
                .iter()
                .map(|(t, v)| {
                    (
                        reg.tag_by_name(t).or_else(|_| {
                            reg.insert_tag(t, opts.color.as_ref().unwrap_or(&"white".to_string()))
                        }),
                        reg.value_by_name(v, false).or_else(|_| reg.insert_value(t)),
                    )
                })
                .collect::<Vec<_>>()
        });

        println!("{pairs:#?}");

        let tags = &opts
            .tags
            .iter()
            .map(|t| {
                reg.tag_by_name(t).or_else(|_| {
                    reg.insert_tag(t, opts.color.as_ref().unwrap_or(&"white".to_string()))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        wutag_debug!("TAG: {:#?}", tags);

        // reg_ok(
        //     &Arc::new(re),
        //     &Arc::new(self.clone()),
        //     |entry: &ignore::DirEntry| {
        //         let reg = self.registry.lock().expect("poisioned lock");
        //
        //         println!("FILES: {:#?}", reg.files_by_flags("e"));

        // if !self.quiet {
        //     println!(
        //         "{}:",
        //         fmt_path(entry.path(), self.base_color, self.ls_colors)
        //     );
        // }

        // let path = entry.path();

        // println!("RETRIEVED: {:#?}", reg.tag_by_name("t"));

        // let tv_pairs = opts.pairs.and_then(|pairs| {
        //     pairs
        //         .iter()
        //         .map(|(t, v)| {
        //             let tag = if let Ok(tag) = reg.tag_by_name(t) {
        //                 tag
        //             } else if let Some(color) = &opts.color {
        //                 reg.insert_tag(t, color)
        //                     .unwrap_or_else(|e| wutag_error!("{}", e))
        //             };
        //
        //             let value = if let Ok(value) = reg.value_by_name(v, false) {
        //                 value
        //             } else {
        //                 reg.insert_value(v)
        //                     .unwrap_or_else(|e| wutag_error!("{}", e))
        //             };
        //
        //             (tag, value)
        //         })
        //         .collect_vec()
        // });

        // Check if it exists
        // let mut file = reg.file_by_path(path);
        // if file.is_err() {
        //     log::debug!("{}: creating fingerprint", path.display());
        //
        //     // Possibly check --force
        //     let hash = hash::blake3_hash(path, None)?;
        //     let count = reg.file_count_by_hash(hash.to_string())?;
        //
        //     if count != 0 {
        //         wutag_warning!(
        //             "{} is a duplicate entry\n{}: {}",
        //             path.display(),
        //             "b3sum".magenta(),
        //             hash.to_string()
        //         );
        //     }
        //
        //     log::debug!("{}: inserting file", path.display());
        //     file = reg.insert_file(path);
        // }
        //
        // // The file was either retrieved first or returned after inserting
        // let file = file?;
        //
        // if !opts.explicit {
        //     log::debug!("{}: determining existing file tags", path.display());
        //     let existing_ft = reg.filetags_by_fileid(file.id(), false)?;
        //
        //     log::debug!("{}: determining implied tags", path.display());
        //     // let new_implications = reg.implications_for()
        // }

        //         Ok(())
        //     },
        // );

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
