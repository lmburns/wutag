//! Options used by the main executable
use std::{env, path::PathBuf, str::FromStr};

use anyhow::Error;
use clap::{crate_description, crate_version, AppSettings, Clap, ValueHint};
use lazy_static::lazy_static;

pub(crate) const YELLOW: &str = "\x1b[0;33m";
pub(crate) const GREEN: &str = "\x1b[0;32m";
pub(crate) const BRCYAN: &str = "\x1b[38;5;14m";
pub(crate) const BRGREEN: &str = "\x1b[38;5;10m";
pub(crate) const BRRED: &str = "\x1b[38;5;9m";
pub(crate) const BRED: &str = "\x1b[01;38;5;1m";
pub(crate) const RES: &str = "\x1b[0m";

pub(crate) const APP_NAME: &str = "wutag";

lazy_static! {
    static ref APP_ABOUT: String = format!(
        "{}DESCRIPTION: {}{}{}", YELLOW, GREEN, crate_description!(), RES
    );
    #[rustfmt::skip]
    static ref EXEC_BATCH_EXPL: String = format!(
        "Execute a command on each search result.\n  \
       '{}{{}}{}':   path (of the current search result)\n  \
       '{}{{/}}{}':  basename\n  \
       '{}{{//}}{}': parent directory\n  \
       '{}{{.}}{}':  path without file extension\n  \
       '{}{{/.}}{}': basename without file extension\n  \
       '{}{{..}}{}': expands to wutag -d <parent_of_match> (only applies to -x/--exec)\n",
       GREEN, RES, GREEN, RES, GREEN, RES, GREEN, RES, GREEN, RES, GREEN, RES
    );
    static ref EXEC_EXPL: String = format!(
        "{}\n \
       \t An example of using this is:\n  \
       \t {}wutag -g search <tag> -x {{..}} set {{/}} <tag2>{}",
       EXEC_BATCH_EXPL.to_string(), BRCYAN, RES
    );
    static ref OVERRIDE_HELP: String = format!(
        "{}wutag{} [{}FLAGS{}/{}OPTIONS{}] <{}SUBCOMMAND{}> [{}TAGS{}/{}FLAGS{}]",
        BRED, RES, GREEN, RES, GREEN, RES, YELLOW, RES, GREEN, RES, GREEN, RES
    );
    static ref AFTER_HELP: String = format!(
        "See {}wutag{} {}--help{} for longer explanations of some base options.\n\
        Use {}--help{} after a subcommand for explanations of more options.",
        BRED, RES, GREEN, RES, GREEN, RES
    );
    #[rustfmt::skip]
    static ref APP_AUTHORS: String = format!(
        "{}Wojciech Kępka{} <{}Wwojciech@wkepka.dev{}>\n\
        {}Lucas Burns{}   <{}lmb@lmburns.com{}>",
        BRRED, RES, BRGREEN, RES, BRRED, RES, BRGREEN, RES,
    );
}

#[derive(Clap, Default, Debug)]
#[clap(
    version = crate_version!(),
    author = APP_AUTHORS.as_ref(),
    about = APP_ABOUT.as_ref(),
    global_setting = AppSettings::ColoredHelp,
    global_setting = AppSettings::ColorAuto,
    global_setting = AppSettings::DisableHelpSubcommand,  // Disables help (use -h)
    global_setting = AppSettings::VersionlessSubcommands, // Shows no --version
    global_setting = AppSettings::InferSubcommands,       // l, li, lis == list
    after_help = AFTER_HELP.as_ref(),
    override_usage = OVERRIDE_HELP.as_ref()
)]
pub(crate) struct Opts {
    /// Specify starting path for filesystem traversal
    #[clap(short, long, next_line_help = true,
        value_hint = ValueHint::DirPath,
        long_about = "\
        When specified, the program will look for files starting from the provided \
        path, otherwise default to current working directory. Only applies to subcommands that \
        take a pattern as a positional argument"
    )]
    pub(crate) dir:              Option<PathBuf>,
    /// Increase maximum recursion depth [default: 2]
    #[clap(
        long,
        short,
        next_line_help = true,
        value_name = "depth",
        long_about = "\
        Increase maximum recursion depth of filesystem traversal to specified value (default: 2). \
                      Only applies to subcommands that take a pattern as a positional argument."
    )]
    pub(crate) max_depth:        Option<usize>,
    /// Specify a different registry to use
    #[clap(long = "registry", short = 'R', next_line_help = true,
        value_hint = ValueHint::FilePath, env = "WUTAG_REGISTRY"
    )]
    pub(crate) reg:              Option<PathBuf>,
    /// Case insensitively search
    #[clap(
        long,
        short = 'i',
        long_about = "\
        Turn the glob into a case insensitive one (default: case sensitive). Only applies to \
                      subcommands that take a pattern as a positional argument."
    )]
    pub(crate) case_insensitive: bool,
    /// Case insensitively search
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
        long,
        short,
        long_about = "\
        Apply operation to files that are already tagged instead of traversing into local \
                      directories or directories specified with '-d|--dir'. Only applies to \
                      'search', 'list', 'rm', and 'clear'."
    )]
    pub(crate) global:           bool,
    /// Respect 'LS_COLORS' environment variable when coloring the output
    #[clap(long, short = 'l')]
    pub(crate) ls_colors:        bool,
    /// When to colorize output
    #[clap(long = "color", short = 'c', value_name = "when",
        next_line_help = true, possible_values = &["never", "auto", "always"],
        long_about = "\
        When to colorize output (usually meant for piping). Valid values are: always, \
        auto, never. The always selection only applies to the path as of now."
    )]
    pub(crate) color_when:       Option<String>,
    #[clap(long, short, global = true, parse(from_occurrences))]
    /// Display debugging messages on 4 levels (i.e., -vv..)
    pub(crate) verbose:          u8,
    #[clap(subcommand)]
    pub(crate) cmd:              Command,
}

// setting = ArgSettings::MultipleOccurrences,

impl Opts {
    /// Allows a default command to run if no arguments are passed
    pub(crate) fn get_args() -> Self {
        if env::args_os().len() > 1 {
            Self::parse()
        } else {
            Self {
                // May switch to local
                global: true,
                cmd: Command::default(),
                ..Default::default()
            }
        }
    }
}

impl Default for Command {
    fn default() -> Self {
        Self::List(ListOpts {
            object: ListObject::Files {
                with_tags: true,
                formatted: true,
                garrulous: false,
            },
            raw:    false,
        })
    }
}

// It seems that 'name' has to be defined to use 'requires' or 'conflicts_with'
#[derive(Clap, Debug)]
pub(crate) enum ListObject {
    Tags {
        #[clap(long = "completions", short = 'c', hidden = true)]
        for_completions: bool,
    },
    Files {
        /// Display tags along with the files
        #[clap(name = "with_tags", long = "with-tags", short = 't')]
        with_tags: bool,
        /// Format the tags and files output into columns
        #[clap(
            name = "formatted",
            conflicts_with = "garrulous",
            long = "format",
            short,
            requires = "with_tags",
            long_about = "Format the tags and files output into columns. Requires '--with-tags'"
        )]
        formatted: bool,
        /// Display tags and files on separate lines
        #[clap(
            name = "garrulous",
            conflicts_with = "formatted",
            long,
            short = 'G',
            requires = "with_tags"
        )]
        garrulous: bool,
    },
}

#[derive(Clap, Debug)]
pub(crate) struct ListOpts {
    /// The object to list. Valid values are: 'tags', 'files'.
    #[clap(subcommand)]
    pub(crate) object: ListObject,
    /// If provided output will be raw so that it can be easily piped to other
    /// commands
    #[clap(long, short)]
    pub(crate) raw:    bool,
}

#[derive(Clap, Clone, Debug)]
pub(crate) struct SetOpts {
    /// Clear all tags before setting them
    #[clap(long, short)]
    pub(crate) clear:   bool,
    /// A glob pattern like "*.png".
    /// Explicitly select color for tag
    #[clap(long, short = 'C', takes_value = true)]
    pub(crate) color:   Option<String>,
    pub(crate) pattern: String,
    pub(crate) tags:    Vec<String>,
}

#[derive(Clap, Debug)]
pub(crate) struct RmOpts {
    /// A glob pattern like "*.png".
    pub(crate) pattern: String,
    pub(crate) tags:    Vec<String>,
}

#[derive(Clap, Debug)]
pub(crate) struct ClearOpts {
    /// A glob pattern like "*.png".
    pub(crate) pattern: String,
}

#[derive(Clap, Debug)]
pub(crate) struct SearchOpts {
    #[clap(required = true)]
    pub(crate) tags:    Vec<String>,
    /// If provided output will be raw so that it can be easily piped to other
    /// commands
    #[clap(long, short)]
    pub(crate) raw:     bool,
    /// If set to 'true' all entries containing any of provided tags will be
    /// returned
    #[clap(long, short)]
    pub(crate) any:     bool,
    /// Execute a command on each individual file
    #[rustfmt::skip]
    #[clap(
        name = "exec", long = "exec", short = 'x',
        takes_value = true, min_values = 1, value_name = "cmd",
        value_terminator = ";", allow_hyphen_values = true,
        long_about = EXEC_EXPL.as_ref()
    )]
    pub(crate) execute:       Option<Vec<String>>,
    /// Execute a command on the batch of matching files
    #[clap(
        long = "exec-batch", short = 'X', takes_value = true,
        min_values = 1, allow_hyphen_values = true,
        value_terminator = ";", value_name = "cmd",
        conflicts_with = "exec",
        long_about = EXEC_BATCH_EXPL.as_ref()
    )]
    pub(crate) execute_batch: Option<Vec<String>>,
}

#[derive(Clap, Debug)]
pub(crate) struct CpOpts {
    /// Path to the file from which to copy tags from
    #[clap(value_hint = ValueHint::FilePath)]
    pub(crate) input_path: PathBuf,
    /// A glob pattern like "*.png".
    pub(crate) pattern:    String,
}

#[derive(Clap, Debug)]
pub(crate) struct EditOpts {
    /// The tag to edit
    pub(crate) tag:   String,
    #[clap(long, short)]
    /// Set the color of the tag to the specified color. Accepted values are hex
    /// colors like '0x000000' or '#1F1F1F' or just plain 'ff000a'. The
    /// colors are case insensitive meaning '1f1f1f' is equivalent to
    /// '1F1F1F'.
    pub(crate) color: String,
}

#[derive(Clap, Debug)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Shell {
    Bash,
    Elvish,
    Fish,
    PowerShell,
    Zsh,
}

impl FromStr for Shell {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &s.to_lowercase()[..] {
            "bash" => Ok(Shell::Bash),
            "elvish" => Ok(Shell::Elvish),
            "fish" => Ok(Shell::Fish),
            "powershell" => Ok(Shell::PowerShell),
            "zsh" => Ok(Shell::Zsh),
            _ => Err(Error::msg(format!("invalid shell `{}`", s))),
        }
    }
}

impl Shell {
    pub(crate) fn variants() -> [&'static str; 5] {
        ["bash", "elvish", "fish", "powershell", "zsh"]
    }
}

#[derive(Clap, Debug)]
pub(crate) struct CompletionsOpts {
    /// Shell to print completions. Available shells are: bash, elvish, fish,
    /// powershell, zsh
    #[clap(long, possible_values = &Shell::variants())]
    pub(crate) shell: Shell,
}

#[derive(Clap, Debug)]
pub(crate) enum Command {
    /// Lists all available tags or files.
    #[clap(aliases = &["ls", "l", "li", "lis"])]
    // Have to do this to be compatible with InferSubcommands
    List(ListOpts),
    /// Set tag(s) on files that match the given pattern
    Set(SetOpts),
    /// Remove tag(s) from the files that match the provided pattern
    #[clap(aliases = &["remove", "r"])]
    Rm(RmOpts),
    /// Clears all tags of the files that match the provided pattern
    Clear(ClearOpts),
    /// Searches for files that have all of the provided 'tags'
    Search(SearchOpts),
    /// Copies tags from the specified file to files that match a pattern
    Cp(CpOpts),
    /// Edits a tag
    Edit(EditOpts),
    /// Prints completions for the specified shell to stdout
    #[clap(display_order = 1000)]
    PrintCompletions(CompletionsOpts),
    /// Clean the cached tag registry
    CleanCache,
}
