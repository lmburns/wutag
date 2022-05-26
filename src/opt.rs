//! Options used by the main executable
use anyhow::{anyhow, Result};
use atty::Stream;
use clap::{crate_version, AppSettings, Parser, Subcommand, ValueHint};
use cli_table::ColorChoice;
use std::{env, fs, path::PathBuf, str::FromStr};

use crate::{
    consts::{
        AFTER_HELP, APP_ABOUT, APP_AUTHORS, DEFAULT_EDITOR, FILE_TYPE, OVERRIDE_HELP, RM_LONG_HELP,
    },
    registry::types::Sort,
    subcommand::{
        clear::ClearOpts,
        cp::CpOpts,
        edit::EditOpts,
        info::InfoOpts,
        list::{ListObject, ListOpts},
        print_completions::CompletionsOpts,
        repair::RepairOpts,
        rm::RmOpts,
        search::SearchOpts,
        set::SetOpts,
        view::ViewOpts,
    },
};

// INFO: Fully qualified path is needed after adding 'notify-rust'

/// Command-line options
#[derive(Parser, Default, Clone, Debug, PartialEq)]
#[clap(
    version = crate_version!(),
    author = <String as AsRef<str>>::as_ref(&APP_AUTHORS),
    about = <String as AsRef<str>>::as_ref(&APP_ABOUT),
    after_help =  <String as AsRef<str>>::as_ref(&AFTER_HELP),
    override_usage =  <String as AsRef<str>>::as_ref(&OVERRIDE_HELP),
    color = clap::ColorChoice::Auto,
    max_term_width = 100,
    infer_subcommands = true, // l, li, lis == list
    infer_long_args = true, // Same as above but for args
    disable_help_subcommand = true, // Disables help (use -h)
    dont_collapse_args_in_usage = true,
    subcommand_required = false, // A default command has been setup
    global_setting(AppSettings::DeriveDisplayOrder), // Display in order listed here
)]
pub(crate) struct Opts {
    #[clap(long, short, global = true, parse(from_occurrences))]
    /// Display debugging messages on 4 levels (i.e., -vv..)
    pub(crate) verbose: u8,

    /// Specify starting path for filesystem traversal
    #[clap(
        long, short,
        number_of_values = 1,
        value_name = "dir",
        value_hint = ValueHint::DirPath,
        validator = |t| fs::metadata(t)
                            .map_err(|_| "must be a valid path")
                            .map(|_| ())
                            .map_err(|e| e.to_string()),
        long_help = "\
        When specified, the program will look for files starting from the provided \
        path, otherwise default to current working directory. Only applies to subcommands that \
        take a pattern as a positional argument"
    )]
    pub(crate) dir: Option<PathBuf>,

    /// Set maximum depth to recurse into
    #[clap(
        name = "max-depth",
        long = "max-depth",
        short = 'm',
        value_name = "num",
        validator = |t| t.parse::<usize>()
                            .map_err(|_| "must be a number")
                            .map(|_| ())
                            .map_err(|e| e.to_string()),
        long_help = "\
        Increase maximum recursion depth of filesystem traversal to specified value (default: 2). \
                      Only applies to subcommands that take a pattern as a positional argument."
    )]
    pub(crate) max_depth: Option<usize>,

    /// Specify a different registry to use
    #[clap(
        name = "registry",
        long = "registry",
        short = 'R',
        value_hint = ValueHint::FilePath,
        env = "WUTAG_REGISTRY",
        value_name = "file",
        hide_env = true
    )]
    pub(crate) reg: Option<PathBuf>,

    /// Case insensitively search
    #[clap(
        name = "case_insensitive",
        long = "case-insensitive",
        short = 'i',
        overrides_with_all = &["case_sensitive", "case_insensitive"],
        long_help = "\
        Turn the glob into a case insensitive one (default: case insensitive). Overrides \
        --case-sensitive, and becomes case-sensitive if a search is performed with an \
        uppercase-character. Only applies to subcommands that take a pattern as a positional \
        argument."
    )]
    pub(crate) case_insensitive: bool,

    /// Case sensitively search
    #[clap(
        name = "case_sensitive",
        long = "case-sensitive",
        short = 's',
        overrides_with_all = &["case_sensitive", "case_insensitive"],
        long_help = "\
        Turn the glob into a case sensitive one (default: case sensitive). Overrides \
        --case-insensitive. Only applies to subcommands that take a pattern as a positional \
        argument."
    )]
    pub(crate) case_sensitive: bool,

    /// Search with a regular expressions
    #[clap(
        name = "regex",
        long,
        short = 'r',
        overrides_with = "regex",
        long_help = "\
        Search for files using a regular expression instead of a glob. Only applies to subcommands \
                     that take a pattern as a positional argument."
    )]
    pub(crate) regex: bool,

    // TODO: Configuration option for these ^|
    /// Search with a glob pattern
    #[clap(
        name = "glob",
        long,
        short = 'G',
        overrides_with_all = &["glob", "regex"],
        hide_short_help = true,
        long_help = "\
        Search for files using a glob expression instead of a glob. Only applies to subcommands \
                     that take a pattern as a positional argument. Note: This is the default \
                     behavior"
    )]
    pub(crate) glob: bool,

    /// Search with a literal fixed-string
    #[clap(
        name = "fixed_string",
        long = "fixed-string",
        short = 'F',
        overrides_with = "fixed-string",
        hide_short_help = true,
        long_help = "\
        Search using a fixed-string. It is probably better to use the default pattern searching of \
                     glob"
    )]
    pub(crate) fixed_string: bool,

    /// Apply operation to all tags and files instead of locally
    #[clap(
        name = "global",
        long,
        short,
        long_help = "\
        Apply operation to files that are already tagged instead of traversing into local \
                     directories or directories specified with '-d|--dir'. Only applies to \
                     'search', 'list', 'rm', and 'clear'."
    )]
    pub(crate) global: bool,

    /// Follow symlinks when peforming an action on a file
    #[clap(
        name = "follow",
        long = "follow",
        short = 'L',
        alias = "dereference",
        overrides_with = "follow",
        long_help = "\
        Peform the action (set, remove, modify) on the dereferenced file. This option can also be \
                     set in the configuration file. Overrides configuration and this option can \
                     be overriden with '--no-follow'"
    )]
    pub(crate) follow_links: bool,

    /// Do not follow symlinks when peforming an action on a file
    #[clap(
        hide = true,
        name = "no_follow",
        long = "no-follow",
        alias = "reference",
        overrides_with = "follow",
        long_help = "Overrides '--follow' or a configuration option that wants to follow symlinks"
    )]
    pub(crate) no_follow_links: bool,

    /// Respect 'LS_COLORS' environment variable when coloring the output
    #[clap(long, short = 'l', conflicts_with = "color")]
    pub(crate) ls_colors: bool,

    /// When to colorize output
    #[clap(
        name = "color",
        long = "color",
        short = 'c',
        value_name = "when",
        long_help = "\
        When to colorize output. Valid values are: always, auto, never"
    )]
    pub(crate) color_when: Option<Colorization>,

    /// File-type(s) to filter by: f|file, d|directory, l|symlink, e|empty
    #[clap(
        long = "type",
        short = 't',
        number_of_values = 1,
        multiple_occurrences = true,
        takes_value = true,
        value_name = "filetype",
        hide_possible_values = true,
        possible_values = &[
            "f", "file",
            "d", "dir",
            "l", "symlink",
            "b", "block",
            "c", "char",
            "s", "socket",
            "p", "fifo",
            "x", "executable",
            "e", "empty",
        ],
        long_help = FILE_TYPE.as_ref(),
    )]
    pub(crate) file_type: Option<Vec<String>>,

    #[clap(
        // global = true,
        name = "extension",
        long = "ext",
        short = 'e',
        number_of_values = 1,
        multiple_occurrences = true,
        takes_value = true,
        value_name = "extension",
        long_help = "\
        Specify file extensions to match against (can be used multiple times) instead of using the \
                      glob '*.{rs,go}' or the regex '.*.(rs|go)'. Used like: 'wutag -e rs set '*' \
                      <tag>'. Can be used multiple times: e.g., -e rs -e go.
        "
    )]
    /// Filter results by file extension
    pub(crate) extension: Option<Vec<String>>,

    #[clap(
        name = "exclude",
        long = "exclude",
        short = 'E',
        number_of_values = 1,
        multiple_occurrences = true,
        takes_value = true,
        value_name = "pattern",
        value_hint = ValueHint::DirPath,
        // conflicts_with = "global",
        long_help = "\
        Specify a pattern to exclude from the results. Can be used multiple times: e.g., \
        -E path/here -E path/there.
        "
    )]
    /// Exclude results that match pattern
    pub(crate) exclude: Option<Vec<String>>,

    // TODO: Implement all around
    /// Do not display any output for any command
    #[clap(
        name = "quiet",
        long = "quiet",
        short = 'q',
        long_help = "Do not display any output for any command. Used within the TUI but made \
                     available to users"
    )]
    pub(crate) quiet: bool,

    /// Subcommand to be ran
    #[clap(subcommand)]
    pub(crate) cmd: Command,
}

impl Opts {
    /// Allows a default command to run if no arguments are passed
    pub(crate) fn get_args() -> Self {
        if env::args_os().len() > 1 {
            Self::parse()
        } else {
            Self {
                // May switch to global
                global: false,
                cmd: Command::default(),
                ..Self::default()
            }
        }
    }

    /// Options for viewing a file within the TUI (edit command in TUI, view
    /// command on CLI)
    #[cfg(feature = "ui")]
    pub(crate) fn view_args(pattern: &str) -> Self {
        Self {
            global: true,
            regex: true,
            cmd: Command::View(ViewOpts {
                editor: DEFAULT_EDITOR.to_string(),
                pattern: Some(pattern.to_owned()),
                ..ViewOpts::default()
            }),
            ..Self::default()
        }
    }
}

impl Default for Command {
    fn default() -> Self {
        Self::List(ListOpts {
            object: ListObject::Files {
                with_tags:   true,
                with_values: true,
                formatted:   true,
                border:      false,
                garrulous:   false,
                sort:        Sort::None,
                relative:    false,
            },
            raw:    false,
        })
    }
}

// macro_rules! colorize_help {
//     ($cmd:tt, $($patt:tt)*) => ({
//         format!("{} [FLAGS/OPTIONS] {} [FLAGS/OPTIONS] {}",
//             "wutag".to_string().green(),
//             $cmd.red().bold(),
//             format!($($patt)*)
//         ).as_str()
//     })
// }

// ╭──────────────────────────────────────────────────────────╮
// │                        Subcommand                        │
// ╰──────────────────────────────────────────────────────────╯

// For subcommand inference and aliases to coexist, the subcommand inferences
// must be listed as aliases

/// All subcommands to `wutag`
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub(crate) enum Command {
    /// Lists all available tags or files
    #[clap(
        aliases = &["ls", "l", "li", "lis"],
        // override_usage = "wutag [OPTIONS] list [OPTIONS] <SUBCOMMAND> [OPTIONS]",
        long_about = "\
            List all tagged files or tags under current directory if the global option \
            is not present, else list all tagged files or tags in the registry. Alias: ls"
    )]
    List(ListOpts),

    /// Set tag(s) and/or value(s) on results from a patterned query
    #[clap(
        aliases = &["set", "tag"],
        override_usage = "wutag [OPTIONS] set [OPTIONS] <pattern> <tags>...",
        long_about = "Set tag(s) on files or value(s) on tag(s) that match a given pattern. \
            Alias: tag"
    )]
    Set(SetOpts),

    /// Remove tag(s) from the files that match the provided pattern
    #[clap(
        aliases = &["remove", "del", "delete"],
        override_usage = "wutag [OPTIONS] rm [OPTIONS] <pattern> <tags>...",
        long_about = <String as AsRef<str>>::as_ref(&RM_LONG_HELP)
    )]
    Rm(RmOpts),

    /// Clears all tags of the files that match the provided pattern
    #[clap(override_usage = "wutag [FLAG/OPTIONS] clear [FLAG/OPTIONS] <pattern>")]
    Clear(ClearOpts),

    /// Searches for files that have all of the provided 'tags'
    #[clap(
        aliases = &["query"],
        override_usage = "wutag [FLAG/OPTIONS] search [FLAG/OPTIONS] <pattern>"
    )]
    Search(SearchOpts),

    /// Copies tags from the specified file to files that match a pattern
    #[clap(
        aliases = &["copy"],
        override_usage = "wutag [FLAG/OPTIONS] cp [FLAG/OPTIONS] <input_path> <pattern>"
    )]
    Cp(CpOpts),

    /// View the results in an editor (optional pattern)
    #[clap(
        aliases = &["see", "view", "v"],
        override_usage = "wutag [FLAG/OPTIONS] view [FLAG/OPTIONS] -p [<pattern>]"
    )]
    View(ViewOpts),

    /// Edits a tag's color
    #[clap(
        aliases = &["edit", "e"],
        override_usage = "wutag edit [FLAG/OPTIONS] <tag>"
    )]
    Edit(EditOpts),

    /// Display information about the wutag environment
    Info(InfoOpts),

    /// Repair broken/missing/modified files in the registry
    #[clap(
        aliases = &["fix", "rep", "repa", "repai"],
        override_usage = "wutag [FLAG/OPTIONS] repair [FLAG/OPTIONS]",
        long_about = "\
        Repair broken file paths or update the file's hash in the registry. \
        Alias: fix"
    )]
    Repair(RepairOpts),

    /// Prints completions for the specified shell to dir or stdout
    #[clap(
        display_order = 1000,
        aliases = &["comp", "completions", "print-completions"],
        override_usage = "wutag print-completions --shell <shell> [FLAG/OPTIONS]"
    )]
    PrintCompletions(CompletionsOpts),

    /// Clean the cached tag registry
    #[clap(
        aliases = &["clean", "cache", "rm-cache"],
        override_usage = "wutag [FLAG/OPTIONS] clean-cache",
        long_about = "Clean out the entire registry. Aliases: 'clean', 'cache', 'rm-cache'"
    )]
    CleanCache,

    /// Open a TUI to manage tags
    #[clap(
        aliases = &["tui"],
        override_usage = "wutag [FLAG/OPTIONS] ui",
        long_about = "Start the TUI to manage the registry interactively. Alias: tui"
    )]
    Ui,
}

// ╭──────────────────────────────────────────────────────────╮
// │                       Colorization                       │
// ╰──────────────────────────────────────────────────────────╯

/// When to colorize the output
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum Colorization {
    Always,
    Never,
    Auto,
}

impl From<Colorization> for ColorChoice {
    fn from(c: Colorization) -> Self {
        match c {
            Colorization::Always => Self::Always,
            Colorization::Never => Self::Never,
            Colorization::Auto => Self::Auto,
        }
    }
}

impl Default for Colorization {
    fn default() -> Self {
        Self::Auto
    }
}

impl FromStr for Colorization {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        match s.to_ascii_lowercase().trim() {
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            "auto" =>
                if env::var_os("NO_COLOR").is_none() && atty::is(Stream::Stdout) {
                    Ok(Self::Auto)
                } else {
                    Ok(Self::Never)
                },
            _ => Err(anyhow!(
                "\
Valid values are:
    - auto
    - always
    - never",
            )),
        }
    }
}

// TODO: USE OR DELETE
// "f", "file",
// "d", "dir",
// "l", "symlink",
// "b", "block",
// "c", "char",
// "s", "socket",
// "p", "fifo",
// "x", "executable",
// "e", "empty",

// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
// pub(crate) enum FileType {
//     File,
//     Directory,
//     Symlink,
//     BlockDevice,
//     CharacterDevice,
//     Socket,
//     Fifo,
//     Executable,
//     Empty,
// }

// impl FromStr for FileType {
//     type Err = anyhow::Error;
//
//     fn from_str(s: &str) -> Result<Self, anyhow::Error> {
//         match s.to_ascii_lowercase().trim() {
//             "file" | "f" => Ok(Self::Always),
//             _ => Err(anyhow!(
//                 "\
// Valid values are:
//     - auto
//     - always
//     - never", )), }
//     }
// }
