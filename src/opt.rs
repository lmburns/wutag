//! Options used by the main executable
use std::{path::PathBuf, str::FromStr, env};

use anyhow::Error;
use clap::{AppSettings, Clap, crate_version, ValueHint};

// pub const APP_VERSION: &str = "0.4.3";
pub const APP_NAME: &str = "wutag";
pub const APP_AUTHOR: &str = "\
      \x1b[01;38;5;13mWojciech Kępka\x1b[0m <\x1b[01;38;5;10mWwojciech@wkepka.dev\x1b[0m> \
    \n\x1b[01;38;5;13mLucas Burns\x1b[0m    <\x1b[01;38;5;10mlmb@lmburns.com\x1b[0m>";
pub static APP_ABOUT: &str = "\
    \x1b[0;33mDESCRIPTION: \x1b[0;31mTag files and manage them with color\x1b[0m";

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
)]
pub struct Opts {
    /// Specify starting path for filesystem traversal
    #[clap(short, long, next_line_help = true,
        value_hint = ValueHint::DirPath,
        long_about = "\
        When specified, the program will look for files starting from the provided \
        path, otherwise default to current working directory. Only applies to subcommands that \
        take a pattern as a positional argument"
    )]
    pub dir: Option<PathBuf>,
    /// Increase maximum recursion depth (default: 2)
    #[clap(long, short, next_line_help = true,
        long_about = "\
        Increase maximum recursion depth of filesystem traversal to specified value (default: 2). \
        Only applies to subcommands that take a pattern as a positional argument."
    )]
    pub max_depth: Option<usize>,
    /// Specify a different registry to use
    #[clap(long = "registry", short, next_line_help = true, value_hint = ValueHint::FilePath)]
    pub reg: Option<PathBuf>,
    /// Case insensitively search
    #[clap(long, short = 'i',
        long_about = "\
        Turn the glob into a case insensitive one (default: case sensitive). \
        Only applies to subcommands that take a pattern as a positional argument."
    )]
    pub case_insensitive: bool,
    /// List all tags and files instead of locally
    #[clap(long, short,
        long_about = "\
        Apply operation to files that are already tagged instead of traversing into local directories \
        or directories specified with '-d|--dir'. Only applies to 'search', 'list', 'rm', and 'clear'."
    )]
    pub global: bool,
    /// Do not colorize the output
    #[clap(long, short, env = "NO_COLOR", takes_value = false)]
    pub no_color: bool,
    #[clap(subcommand)]
    pub cmd: Command,
}

impl Opts {
    /// Default command to run if no arguments are passed
    pub fn base() -> Self {
        Self {
            global: true,
            cmd: Command::default(),
            ..Default::default()
        }
    }
}

impl Default for Command {
    fn default() -> Self {
        Self::List(
            ListOpts {
                object: ListObject::Files {
                    with_tags: true,
                    formatted: true,
                    garrulous: false,
                },
                raw: false,
            }
        )
    }
}

// It seems that 'name' has to be defined to use 'requires' or 'conflicts_with'
#[derive(Clap, Debug)]
pub enum ListObject {
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
            name = "formatted", conflicts_with = "garrulous",
            long = "format", short, requires = "with_tags",
            long_about = "Format the tags and files output into columns. Requires '--with-tags'"
        )]
        formatted: bool,
        /// Display tags and files on separate lines
        #[clap(
            name = "garrulous", conflicts_with = "formatted",
            long, short = 'G', requires = "with_tags"
        )]
        garrulous: bool
    },
}

#[derive(Clap, Debug)]
pub struct ListOpts {
    /// The object to list. Valid values are: 'tags', 'files'.
    #[clap(subcommand)]
    pub object: ListObject,
    /// If provided output will be raw so that it can be easily piped to other commands
    #[clap(long, short)]
    pub raw: bool,
}

#[derive(Clap, Debug)]
pub struct SetOpts {
    /// Clear tags before setting the tags (may implement a set and add command separately)
    #[clap(long, short)]
    pub clear: bool,
    /// A glob pattern like "*.png".
    pub pattern: String,
    pub tags: Vec<String>,
}

#[derive(Clap, Debug)]
pub struct RmOpts {
    /// A glob pattern like "*.png".
    pub pattern: String,
    pub tags: Vec<String>,
}

#[derive(Clap, Debug)]
pub struct ClearOpts {
    /// A glob pattern like "*.png".
    pub pattern: String,
}

#[derive(Clap, Debug)]
pub struct SearchOpts {
    #[clap(required = true)]
    pub tags: Vec<String>,
    /// If provided output will be raw so that it can be easily piped to other commands
    #[clap(long, short)]
    pub raw: bool,
    /// If set to 'true' all entries containing any of provided tags will be returned
    #[clap(long, short)]
    pub any: bool,
}

#[derive(Clap, Debug)]
pub struct CpOpts {
    /// Path to the file from which to copy tags from
    #[clap(value_hint = ValueHint::FilePath)]
    pub input_path: PathBuf,
    /// A glob pattern like "*.png".
    pub pattern: String,
}

#[derive(Clap, Debug)]
pub struct EditOpts {
    /// The tag to edit
    pub tag: String,
    #[clap(long, short)]
    /// Set the color of the tag to the specified color. Accepted values are hex colors like
    /// '0x000000' or '#1F1F1F' or just plain 'ff000a'. The colors are case insensitive meaning
    /// '1f1f1f' is equivalent to '1F1F1F'.
    pub color: String,
}

#[derive(Clap, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
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
    pub fn variants() -> [&'static str; 5] {
        ["bash", "elvish", "fish", "powershell", "zsh"]
    }
}

#[derive(Clap, Debug)]
pub struct CompletionsOpts {
    /// Shell to print completions. Available shells are: bash, elvish, fish,
    /// powershell, zsh
    #[clap(long, possible_values = &Shell::variants())]
    pub shell: Shell,
}

#[derive(Clap, Debug)]
pub enum Command {
    /// Lists all available tags or files.
    #[clap(aliases = &["ls", "l", "li", "lis"])] // Have to do this to be compatible with InferSubcommands
    List(ListOpts),
    /// Tags the files that match the given pattern with specified tags.
    Set(SetOpts),
    /// Removes the specified tags of the files that match the provided pattern.
    Rm(RmOpts),
    /// Clears all tags of the files that match the provided pattern.
    Clear(ClearOpts),
    /// Searches for files that have all of the provided 'tags'.
    Search(SearchOpts),
    /// Copies tags from the specified file to files that match a pattern.
    Cp(CpOpts),
    /// Edits a tag.
    Edit(EditOpts),
    /// Prints completions for the specified shell to stdout.
    #[clap(display_order = 1000)]
    PrintCompletions(CompletionsOpts),
    /// Clean the cached tag registry.
    CleanCache,
}
