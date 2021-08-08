//! Options used by the main executable
use std::{path::PathBuf, str::FromStr};

use anyhow::Error;
use clap::{AppSettings, Clap, crate_version};

// crate_description
// use colored::*;

// pub const APP_VERSION: &str = "0.4.1";
pub const APP_NAME: &str = "wutag";
pub const APP_AUTHOR: &str = "\
      \x1b[01;38;5;13mWojciech Kępka\x1b[0m <\x1b[01;38;5;10mWwojciech@wkepka.dev\x1b[0m> \
    \n\x1b[01;38;5;13mLucas Burns\x1b[0m    <\x1b[01;38;5;10mlmb@lmburns.com\x1b[0m>";
pub static APP_ABOUT: &str = "\
    \x1b[0;33mDESCRIPTION: \x1b[0;31mTag files and manage them with color\x1b[0m";

#[derive(Clap)]
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
    #[clap(short, long, next_line_help = true,
        long_about = "When specified, the program will look for files starting from the provided \
        path, otherwise default to current working directory. Only applies to subcommands that \
        take a pattern as a positional argument"
    )]
    /// Specify starting path for filesystem traversal
    pub dir: Option<PathBuf>,
    #[clap(long, short, next_line_help = true,
        long_about = "\
        Increase maximum recursion depth of filesystem traversal to specified value (default: 2). \
        Only applies to subcommands that take a pattern as a positional argument."
    )]
    /// Increase maximum recursion depth (default: 2)
    pub max_depth: Option<usize>,
    #[clap(long = "registry", short, next_line_help = true)]
    /// Specify a different registry to use
    pub reg: Option<PathBuf>,
    #[clap(long, short = 'i',
        long_about = "\
        Turn the glob into a case insensitive one (default: case sensitive). \
        Only applies to subcommands that take a pattern as a positional argument."
    )]
    /// Case insensitively search
    pub case_insensitive: bool,
    #[clap(long, short,
        long_about = "\
        Apply operation to files that are already tagged instead of traversing into local directories \
        or directories specified with '-d|--dir'. Only applies to 'search', 'list', 'rm', and 'clear'."
    )]
    /// List all tags and files instead of locally
    pub global: bool,
    #[clap(long, short, env = "NO_COLOR")]
    /// Do not colorize the output
    pub no_color: bool,
    #[clap(subcommand)]
    pub cmd: Command,
}

// It seems that 'name' has to be defined to use 'requires' or 'conflicts_with'
#[derive(Clap)]
pub enum ListObject {
    Tags,
    Files {
        #[clap(name = "with_tags", long = "with-tags", short = 't')]
        /// Display tags along with the files
        with_tags: bool,
        #[clap(
            name = "formatted", conflicts_with = "garrulous",
            long = "format", short, requires = "with_tags",
            long_about = "Format the tags and files output into columns. Requires '--with-tags'"
        )]
        /// Format the tags and files output into columns
        formatted: bool,
        #[clap(
            name = "garrulous", conflicts_with = "formatted",
            long, short = 'G', requires = "with_tags"
        )]
        /// Display tags and files on separate lines
        garrulous: bool
    },
}

#[derive(Clap)]
pub struct ListOpts {
    #[clap(subcommand)]
    /// The object to list. Valid values are: 'tags', 'files'.
    pub object: ListObject,
    #[clap(long, short)]
    /// If provided output will be raw so that it can be easily piped to other commands
    pub raw: bool,
}

#[derive(Clap)]
pub struct SetOpts {
    /// Clear tags before setting the tags (may implement a set and add command separately)
    #[clap(long, short)]
    pub clear: bool,
    /// A glob pattern like '*.png'.
    pub pattern: String,
    #[clap(required = true)]
    pub tags: Vec<String>,
}

#[derive(Clap)]
pub struct RmOpts {
    #[clap(long, short)]
    /// A glob pattern like '*.png'.
    pub pattern: String,
    pub tags: Vec<String>,
}

#[derive(Clap)]
pub struct ClearOpts {
    /// A glob pattern like '*.png'.
    pub pattern: String,
}

#[derive(Clap)]
pub struct SearchOpts {
    #[clap(required = true)]
    pub tags: Vec<String>,
    #[clap(long, short)]
    /// If provided output will be raw so that it can be easily piped to other commands
    pub raw: bool,
    #[clap(long, short)]
    /// If set to 'true' all entries containing any of provided tags will be returned
    pub any: bool,
}

#[derive(Clap)]
pub struct CpOpts {
    /// Path to the file from which to copy tags from
    pub input_path: PathBuf,
    /// A glob pattern like '*.png'.
    pub pattern: String,
}

#[derive(Clap)]
pub struct EditOpts {
    /// The tag to edit
    pub tag: String,
    #[clap(long, short)]
    /// Set the color of the tag to the specified color. Accepted values are hex colors like
    /// '0x000000' or '#1F1F1F' or just plain 'ff000a'. The colors are case insensitive meaning
    /// '1f1f1f' is equivalent to '1F1F1F'.
    pub color: String,
}

#[derive(Clap)]
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

#[derive(Clap)]
pub struct CompletionsOpts {
    /// A shell for which to print completions. Available shells are: bash, elvish, fish,
    /// powershell, zsh
    pub shell: Shell,
}

#[derive(Clap)]
pub enum Command {
    #[clap(alias = "ls")]
    /// Lists all available tags or files.
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
    #[clap(display_order = 1000)]
    /// Prints completions for the specified shell to stdout.
    PrintCompletions(CompletionsOpts),
    /// Clean the cached tag registry.
    CleanCache,
}
