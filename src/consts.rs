//! Constants used throughout this crate. This is mainly to 'prettify' the
//! output with color

use clap::crate_description;
use colored::Color::{
    self, Blue, BrightBlue, BrightCyan, BrightGreen, BrightMagenta, BrightRed, BrightYellow, Cyan,
    Green, Magenta, Red, White, Yellow,
};
use once_cell::sync::Lazy;
use std::env;

/// Yellow ANSI escape sequence
pub(crate) const YELLOW: &str = "\x1b[0;33m";
/// Green ANSI escape sequence
pub(crate) const GREEN: &str = "\x1b[0;32m";
/// Bright Cyan ANSI escape sequence
pub(crate) const BRCYAN: &str = "\x1b[38;5;14m";
/// Bright Green ANSI escape sequence
pub(crate) const BRGREEN: &str = "\x1b[38;5;10m";
/// Bright Red ANSI escape sequence
pub(crate) const BRRED: &str = "\x1b[38;5;9m";
/// Bold Red ANSI escape sequence
pub(crate) const BRED: &str = "\x1b[01;38;5;1m";
/// Restore ANSI escape sequence
pub(crate) const RES: &str = "\x1b[0m";

/// The app name to use for `clap`
pub(crate) const APP_NAME: &str = "wutag";

/// Default base color for printing files
pub(crate) const DEFAULT_BASE_COLOR: Color = Blue;
/// Default border color for printing formatted tags
#[cfg(feature = "prettify")]
pub(crate) const DEFAULT_BORDER_COLOR: cli_table::Color = cli_table::Color::White;
/// Default max depth passed to [WalkParallel](ignore::WalkParallel)
pub(crate) const DEFAULT_MAX_DEPTH: usize = 2;
/// Default colors used for tags
pub(crate) const DEFAULT_COLORS: &[Color] = &[
    Red,
    Green,
    Blue,
    Yellow,
    Cyan,
    White,
    Magenta,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
];

/// Colored options used in the output of `--help`
pub(crate) static APP_ABOUT: Lazy<String> = Lazy::new(|| {
    format!(
        "{}DESCRIPTION: {}{}{}",
        YELLOW,
        GREEN,
        crate_description!(),
        RES
    )
});

/// Specify the file-type(s) to filter by. Can be repeated
#[rustfmt::skip]
pub(crate) static FILE_TYPE: Lazy<String> = Lazy::new(|| {
    format!("Filter results based on file-type. Does not work with '-g|--global'.\n  \
        'f' or 'file':       regular file\n  \
        'd' or 'dir':        directory\n  \
        'l' or 'symlink':    symlink\n  \
        'b' or 'block':      block device\n  \
        'c' or 'char':       character device\n  \
        's' or 'socket':     socket\n  \
        'F' or 'fifo':       fifo\n  \
        'x' or 'executable': executable\n \
        'e' or 'empty':      file or directory with 0 size\n\n \
    More than one file-type can be searched for by repeating the argument.\n \
    For example:\n \
        1. Searches for directories and symlinks\n \
        - {}wutag search -td -tl '*'{}\n \
        2. Searches for empty files\n \
        - {}wutag search -te -tf '*'{}\n \
        3. Searches for empty directories\n \
        - {}wutag search -te -td '*'{}",
        GREEN, RES,
        GREEN, RES,
        GREEN, RES,
        )
});

/// Colorized message to explain the -X flag to execute commands on tagged files
#[rustfmt::skip]
pub(crate) static EXEC_BATCH_EXPL: Lazy<String> = Lazy::new(|| {
    format!(
        "Execute a command on each search result.\n  \
       '{}{{}}{}':   path (of the current search result)\n  \
       '{}{{/}}{}':  basename\n  \
       '{}{{//}}{}': parent directory\n  \
       '{}{{.}}{}':  path without file extension\n  \
       '{}{{/.}}{}': basename without file extension\n  \
       '{}{{..}}{}': expands to wutag -d <parent_of_match> (only applies to -x/--exec)\n  \
       '{}{{@}}{}': same as above but with color (only applies to -x/--exec)\n  \
       '{}{{@s}}{}': shorthand to set tag\n  \
        \t  e.g., {}wutag search '*.rs' -x {{@s}} new_tag{} (only applies to -x/--exec)\n  \
       '{}{{@r}}{}': shorthand to remove tag (only applies to -x/--exec)\n  \
       '{}{{@c}}{}': shorthand to clear tag, no other arg required (only applies to -x/--exec)\n  \
        \t  e.g., {}wutag search '*.rs' -x {{@c}}{}",
       GREEN, RES, GREEN, RES,
       GREEN, RES, GREEN, RES,
       GREEN, RES, GREEN, RES,
       GREEN, RES, GREEN, RES,
       BRCYAN, RES, GREEN, RES,
       GREEN, RES, BRCYAN, RES
    )
});

/// Colorized message to explain the -x flag to execute commands on tagged files
pub(crate) static EXEC_EXPL: Lazy<String> = Lazy::new(|| {
    format!(
        "{}\n  An example of using this is:\n  \t {}wutag -g search <tag> -x {{..}} set {{/}} \
         <tag2>{}",
        *EXEC_BATCH_EXPL, BRCYAN, RES
    )
});

/// Colorized message to override the generated help message
pub(crate) static OVERRIDE_HELP: Lazy<String> = Lazy::new(|| {
    format!(
        "{}wutag{} [{}FLAGS{}/{}OPTIONS{}] <{}SUBCOMMAND{}> [{}TAGS{}/{}FLAGS{}]",
        BRED, RES, GREEN, RES, GREEN, RES, YELLOW, RES, GREEN, RES, GREEN, RES
    )
});

/// Colorized message displayed after the help message
pub(crate) static AFTER_HELP: Lazy<String> = Lazy::new(|| {
    format!(
        "See {}wutag{} {}--help{} for longer explanations of some base options.\nUse {}--help{} \
         after a subcommand for explanations of more options.",
        BRED, RES, GREEN, RES, GREEN, RES
    )
});

/// Colorized message about the app's authors
#[rustfmt::skip]
pub(crate) static APP_AUTHORS: Lazy<String> = Lazy::new(|| format!(
    "{}Wojciech K\u{119}pka{} <{}Wwojciech@wkepka.dev{}>\n\
    {}Lucas Burns{}   <{}lmb@lmburns.com{}>",
    BRRED, RES, BRGREEN, RES, BRRED, RES, BRGREEN, RES,
));

/// Editor to use when viewing tags
pub(crate) static DEFAULT_EDITOR: Lazy<String> = Lazy::new(|| {
    env::var("EDITOR").unwrap_or_else(|_| {
        env::var("VISUAL").unwrap_or_else(|_| {
            {
                if which::which("nvim").is_ok() {
                    "nvim"
                } else if which::which("vim").is_ok() {
                    "vim"
                } else {
                    "vi"
                }
            }
            .to_owned()
        })
    })
});

/// Encryption-specific constants
#[cfg(feature = "encrypt-gpgme")]
pub(crate) mod encrypt {
    use super::{env, Lazy};

    /// The `umask` of the registry file
    #[allow(clippy::unwrap_used)]
    pub(crate) static REGISTRY_UMASK: Lazy<u32> = Lazy::new(|| {
        u32::from_str_radix(
            &env::var("WUTAG_REGISTRY_UMASK").unwrap_or_else(|_| "600".to_owned()),
            8,
        )
        .unwrap_or_else(|_| u32::from_str_radix("600", 8).unwrap())
    });
}
