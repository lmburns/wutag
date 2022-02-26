#![allow(unused)]

use super::App;
use crate::{
    bold_entry, err,
    registry::types::{
        file::FileId,
        filetag::FileTag,
        tag::{Tag as TTag, TagId, TagValueCombo},
        value::ValueId,
    },
    util::{collect_stdin_paths, fmt_err, fmt_path, fmt_tag, glob_builder, reg_ok, regex_builder},
    wutag_error, wutag_fatal,
};
use anyhow::{Context, Result};
use clap::{Args, ValueHint};
use colored::{Color, Colorize};
use std::{error::Error, str::FromStr, sync::Arc};
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
    #[clap(long, short = 'C', takes_value = true,
        validator = |t| parse_color(t)
                            .map_err(|_| "must be a valid hex color")
                            .map(|_| ())
                            .map_err(|e| e.to_string())
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
// TODO: new: explicit, force, no-dereference

impl App {
    /// Set tags on a file
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn set2(&mut self, opts: &Set2Opts) -> Result<()> {
        let reg = self.registry.lock().expect("poisioned lock");
        log::debug!("SetOpts: {:#?}", opts);
        log::debug!("Using registry: {}", reg.path().display());

        println!("{:#?}", opts.clone());
        let mut tags = opts.tags.clone();
        if (opts.stdin || atty::isnt(atty::Stream::Stdin)) && atty::is(atty::Stream::Stdout) {
            tags.push(opts.pattern.clone());
        }

        // let tagz = tags.iter().map(|t| {
        // });

        let tags = reg.tags()?;
        println!("RES: {:#?}", tags);

        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else if self.fixed_string {
            regex::escape(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        log::debug!("Is a TTY?: {}", atty::is(atty::Stream::Stdout));
        log::debug!("Compiled pattern: {}", re);

        // reg_ok(
        //     &Arc::new(re),
        //     &Arc::new(self.clone()),
        //     |entry: &ignore::DirEntry| {
        //         if !self.quiet {
        //             println!(
        //                 "{}:",
        //                 fmt_path(entry.path(), self.base_color, self.ls_colors)
        //             );
        //         }
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

    match (tag.is_empty(), value.is_empty()) {
        (true, true) => Err(Box::new(IoError::new(
            ErrorKind::InvalidInput,
            format!("{}: tag or value cannot be empty", tagval),
        ))),
        (true, false) => Err(Box::new(IoError::new(
            ErrorKind::InvalidInput,
            format!("{}: tag cannot be empty", tagval),
        ))),
        (false, true) => Err(Box::new(IoError::new(
            ErrorKind::InvalidInput,
            format!("{}: value cannot be empty", tagval),
        ))),
        (false, false) => Ok((tag.parse()?, value.parse()?)),
    }
}
