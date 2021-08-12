//! Options used by the main executable
use std::{env, path::PathBuf, str::FromStr};

use anyhow::Error;
use clap::{crate_version, AppSettings, Clap, ValueHint};

pub(crate) const APP_NAME: &str = "wutag";
pub(crate) const APP_AUTHOR: &str = "\
      \x1b[01;38;5;3mWojciech Kępka\x1b[0m <\x1b[01;38;5;10mWwojciech@wkepka.dev\x1b[0m> \
                                     \n\x1b[01;38;5;3mLucas Burns\x1b[0m    \
                                     <\x1b[01;38;5;10mlmb@lmburns.com\x1b[0m>";
pub(crate) static APP_ABOUT: &str = "\
    \x1b[0;33mDESCRIPTION: \x1b[0;32mTag files and manage them with color\x1b[0m";

#[derive(Clap, Default, Debug)]
#[clap(
    version = crate_version!(),
    author = APP_AUTHOR,
    about = APP_ABOUT,
    global_setting = AppSettings::ColoredHelp,
    global_setting = AppSettings::ColorAlways,
    global_setting = AppSettings::DisableHelpSubcommand,  // Disables help (use -h)
    global_setting = AppSettings::VersionlessSubcommands, // Shows no --version
    global_setting = AppSettings::InferSubcommands,       // l, li, lis == list
    after_help = "See wutag --help for some longer explanations of options",
    override_usage = "wutag [FLAGS] [OPTIONS] <SUBCOMMAND> xx",
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
    #[clap(long = "registry", short, next_line_help = true,
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
    #[clap(long, short = '/')]
    pub(crate) ls_colors:        bool,
    /// When to colorize output
    #[clap(long = "color", short = 'c', value_name = "when",
        next_line_help = true, possible_values = &["never", "auto", "always"],
        long_about = "\
        When to colorize output (usually meant for piping). Valid values are: always, \
        auto, never. The always selection only applies to the path as of now."
    )]
    pub(crate) color_when:       Option<String>,
    #[clap(subcommand)]
    pub(crate) cmd:              Command,
}

// /// Do not colorize the output
// #[clap(long, short, env = "NO_COLOR", takes_value = false)]
// pub(crate) no_color: bool,

impl Opts {
    /// Default command to run if no arguments are passed
    pub(crate) fn base() -> Self {
        Self {
            global: false,
            cmd: Command::default(),
            ..Default::default()
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

#[derive(Clap, Debug)]
pub(crate) struct SetOpts {
    /// Clear all tags before setting them
    #[clap(long, short)]
    pub(crate) clear:   bool,
    /// A glob pattern like "*.png".
    pub(crate) pattern: String,
    pub(crate) tags:    Vec<String>,
}

#[derive(Clap, Debug)]
pub(crate) struct RmOpts {
    /// Use extended glob features
    #[clap(long = "extended", short = 'x')]
    pub(crate) extended_glob: bool,
    /// A glob pattern like "*.png".
    pub(crate) pattern:       String,
    pub(crate) tags:          Vec<String>,
}

#[derive(Clap, Debug)]
pub(crate) struct ClearOpts {
    /// A glob pattern like "*.png".
    pub(crate) pattern: String,
}

#[derive(Clap, Debug)]
pub(crate) struct SearchOpts {
    #[clap(required = true)]
    pub(crate) tags: Vec<String>,
    /// If provided output will be raw so that it can be easily piped to other
    /// commands
    #[clap(long, short)]
    pub(crate) raw:  bool,
    /// If set to 'true' all entries containing any of provided tags will be
    /// returned
    #[clap(long, short)]
    pub(crate) any:  bool,
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
