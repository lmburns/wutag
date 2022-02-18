#![allow(unused)]

use super::{
    uses::{
        bold_entry, collect_stdin_paths, err, fmt_err, fmt_path, fmt_tag, glob_builder,
        parse_color, reg_ok, regex_builder, wutag_error, wutag_fatal, Arc, Args, Colorize, Context,
        DirEntryExt, EntryData, Result, Tag, ValueHint, DEFAULT_COLOR,
    },
    App,
};

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
    // -
    // /// A glob pattern like "*.png".
    // #[clap(
    //     required_unless_present = "stdin", // Would be nice to have a default_value_if_present
    //     value_hint = ValueHint::FilePath,
    // )]
    // pub(crate) pattern: String,
    // pub(crate) tags:    Vec<String>,
}

// TODO: Use max-depth, quiet,
// TODO: new: explicit, force, no-dereference

impl App {
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn set2(&mut self, opts: &Set2Opts) -> Result<()> {
        log::debug!("SetOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.oregistry.path.display());

        let reg = self.registry.lock().expect("poisioned lock");

        println!("{:#?}", reg.path());

        Ok(())
    }
}
