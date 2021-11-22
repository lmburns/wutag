//! Options used by the main executable
use clap::{crate_version, AppSettings, ArgSettings, Parser, Subcommand, ValueHint};
use std::{env, fs, path::PathBuf};

use crate::{
    consts::{AFTER_HELP, APP_ABOUT, APP_AUTHORS, DEFAULT_EDITOR, FILE_TYPE, OVERRIDE_HELP},
    subcommand::{
        clear::ClearOpts,
        cp::CpOpts,
        edit::EditOpts,
        info::InfoOpts,
        list::{ListObject, ListOpts},
        print_completions::CompletionsOpts,
        rm::RmOpts,
        search::SearchOpts,
        set::SetOpts,
        view::ViewOpts,
    },
};

// INFO: The fully qualified path is needed after adding 'notify-rust' to
// dependencies
#[derive(Parser, Default, Clone, Debug, PartialEq)]
#[clap(
    version = crate_version!(),
    author = <String as AsRef<str>>::as_ref(&APP_AUTHORS),
    about = <String as AsRef<str>>::as_ref(&APP_ABOUT),
    after_help =  <String as AsRef<str>>::as_ref(&AFTER_HELP),
    override_usage =  <String as AsRef<str>>::as_ref(&OVERRIDE_HELP),
    max_term_width = 100,
    color = clap::ColorChoice::Auto,
    global_setting = AppSettings::DisableHelpSubcommand,        // Disables help (use -h)
    global_setting = AppSettings::DeriveDisplayOrder,           // Display in order listed here
    global_setting = AppSettings::HidePossibleValuesInHelp,
    global_setting = AppSettings::InferSubcommands,             // l, li, lis == list
    // global_setting = AppSettings::InferLongArgs,             // Same as above but for args
    // global_setting = AppSettings::UnifiedHelpMessage,     // Options/Flags together
)]
pub(crate) struct Opts {
    #[clap(long, short, global = true, parse(from_occurrences))]
    /// Display debugging messages on 4 levels (i.e., -vv..)
    pub(crate) verbose:          u8,
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
        long_about = "\
        When specified, the program will look for files starting from the provided \
        path, otherwise default to current working directory. Only applies to subcommands that \
        take a pattern as a positional argument"
    )]
    pub(crate) dir:              Option<PathBuf>,
    /// Set maximum depth to recurse into
    #[clap(
        long, short,
        value_name = "num",
        validator = |t| t.parse::<usize>()
                            .map_err(|_| "must be a number")
                            .map(|_| ())
                            .map_err(|e| e.to_string()),
        long_about = "\
        Increase maximum recursion depth of filesystem traversal to specified value (default: 2). \
                      Only applies to subcommands that take a pattern as a positional argument."
    )]
    pub(crate) max_depth:        Option<usize>,
    /// Specify a different registry to use
    #[clap(
        long = "registry", short = 'R',
        value_hint = ValueHint::FilePath,
        env = "WUTAG_REGISTRY",
        value_name = "reg",
        setting = ArgSettings::HideEnv,
    )]
    pub(crate) reg:              Option<PathBuf>,
    /// Case insensitively search
    #[clap(
        name = "case_insensitive",
        long, short = 'i',
        overrides_with_all = &["case_sensitive", "case_insensitive"],
        long_about = "\
        Turn the glob into a case insensitive one (default: case insensitive). Overrides \
        --case-sensitive, and becomes case-sensitive if a search is performed with an \
        uppercase-character. Only applies to subcommands that take a pattern as a positional \
        argument."
    )]
    pub(crate) case_insensitive: bool,
    /// Case sensitively search
    #[clap(
        name = "case_sensitive",
        long, short = 's',
        overrides_with_all = &["case_sensitive", "case_insensitive"],
        long_about = "\
        Turn the glob into a case sensitive one (default: case sensitive). Overrides \
        --case-insensitive. Only applies to subcommands that take a pattern as a positional \
        argument."
    )]
    pub(crate) case_sensitive:   bool,
    /// Search with a regular expressions
    #[clap(
        long,
        short = 'r',
        long_about = "\
        Search for files using a regular expressions instead of a glob. Only applies to \
                      subcommands that take a pattern as a positional argument."
    )]
    pub(crate) regex:            bool,
    /// Apply operation to all tags and files instead of locally
    #[clap(
        name = "global",
        long,
        short,
        long_about = "\
        Apply operation to files that are already tagged instead of traversing into local \
                      directories or directories specified with '-d|--dir'. Only applies to \
                      'search', 'list', 'rm', and 'clear'."
    )]
    pub(crate) global:           bool,
    /// Respect 'LS_COLORS' environment variable when coloring the output
    #[clap(long, short = 'l', conflicts_with = "color")]
    pub(crate) ls_colors:        bool,
    /// When to colorize output
    #[clap(
        name = "color", long = "color", short = 'c',
        value_name = "when",
        possible_values = &["never", "auto", "always"],
        long_about = "\
        When to colorize output (usually meant for piping). Valid values are: always, \
        auto, never. The always selection only applies to the path as of now."
    )]
    pub(crate) color_when:       Option<String>,
    /// File-type(s) to filter by: f|file, d|directory, l|symlink, e|empty
    #[clap(
        long = "type",
        short = 't',
        number_of_values = 1,
        multiple_occurrences = true,
        takes_value = true,
        value_name = "filetype",
        long_about = FILE_TYPE.as_ref(),
    )]
    pub(crate) file_type:        Option<Vec<String>>,
    #[clap(
        long = "ext",
        short = 'e',
        // global = true,
        number_of_values = 1,
        multiple_occurrences = true,
        takes_value = true,
        value_name = "extension",
        long_about = "\
        Specify file extensions to match against (can be used multiple times) instead of using the \
                      glob '*.{rs,go}' or the regex '.*.(rs|go)'. Used like: 'wutag -e rs set '*' \
                      <tag>'. Can be used multiple times: e.g., -e rs -e go.
        "
    )]
    /// Filter results by file extension
    pub(crate) extension:        Option<Vec<String>>,
    #[clap(
        long = "exclude", short = 'E',
        number_of_values = 1,
        multiple_occurrences = true,
        takes_value = true,
        value_name = "pattern",
        value_hint = ValueHint::DirPath,
        // conflicts_with = "global",
        long_about = "\
        Specify a pattern to exclude from the results. Can be used multiple times: e.g., \
        -E path/here -E path/there.
        "
    )]
    /// Exclude results that match pattern
    pub(crate) exclude:          Option<Vec<String>>,
    #[clap(subcommand)]
    pub(crate) cmd:              Command,
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

    /// Options for viewing a file within the TUI
    pub(crate) fn view_args(pattern: &str) -> Self {
        Self {
            global: true,
            regex: true,
            cmd: Command::View(ViewOpts {
                editor: DEFAULT_EDITOR.to_string(),
                pattern: Some(pattern.to_string()),
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
                with_tags: true,
                formatted: true,
                border:    false,
                garrulous: false,
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

#[derive(Subcommand, Debug, Clone, PartialEq)]
pub(crate) enum Command {
    /// Lists all available tags or files.
    #[clap(
        aliases = &["ls", "l", "li", "lis"],
        override_usage = "wutag [FLAG/OPTIONS] list [FLAG/OPTIONS] <SUBCOMMAND> [FLAG/OPTIONS]"
    )]
    List(ListOpts),
    /// Set tag(s) on files that match the given pattern
    #[clap(
        aliases = &["set", "tag"],
        override_usage = "wutag [FLAG/OPTIONS] set [FLAG/OPTIONS] <pattern> <tag>"
    )]
    Set(SetOpts),
    /// Remove tag(s) from the files that match the provided pattern
    #[clap(
        aliases = &["remove", "r"],
        override_usage = "wutag [FLAG/OPTIONS] rm <pattern> <tag>"
    )]
    Rm(RmOpts),
    /// Clears all tags of the files that match the provided pattern
    #[clap(override_usage = "wutag [FLAG/OPTIONS] clear [FLAG/OPTIONS] <pattern>")]
    Clear(ClearOpts),
    /// Searches for files that have all of the provided 'tags'
    #[clap(override_usage = "wutag [FLAG/OPTIONS] search [FLAG/OPTIONS] <pattern>")]
    Search(SearchOpts),
    /// Copies tags from the specified file to files that match a pattern
    #[clap(override_usage = "wutag [FLAG/OPTIONS] cp [FLAG/OPTIONS] <input_path> <pattern>")]
    Cp(CpOpts),
    /// View the results in an editor (optional pattern)
    #[clap(override_usage = "wutag [FLAG/OPTIONS] view [FLAG/OPTIONS] -p [<pattern>]")]
    View(ViewOpts),
    /// Edits a tag's color
    #[clap(override_usage = "wutag edit --color <color> <tag>")]
    Edit(EditOpts),
    /// Display information about the wutag environment
    Info(InfoOpts),
    /// Prints completions for the specified shell to dir or stdout
    #[clap(
        display_order = 1000,
        override_usage = "wutag print-completions --shell <shell>"
    )]
    PrintCompletions(CompletionsOpts),
    /// Clean the cached tag registry
    #[clap(override_usage = "wutag [FLAG/OPTIONS] clean-cache")]
    CleanCache,
    /// Open a TUI to manage tags, requires results from a `search`, or `list`
    #[clap(
        aliases = &["tui"],
        override_usage = "wutag [FLAG/OPTIONS] ui"
    )]
    Ui,
}
