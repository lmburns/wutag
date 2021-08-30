use clap::crate_description;
use colored::Color::{
    self, Blue, BrightBlue, BrightCyan, BrightGreen, BrightMagenta, BrightRed, BrightYellow, Cyan,
    Green, Magenta, Red, White, Yellow,
};
use once_cell::sync::Lazy;

pub(crate) const YELLOW: &str = "\x1b[0;33m";
pub(crate) const GREEN: &str = "\x1b[0;32m";
pub(crate) const BRCYAN: &str = "\x1b[38;5;14m";
pub(crate) const BRGREEN: &str = "\x1b[38;5;10m";
pub(crate) const BRRED: &str = "\x1b[38;5;9m";
pub(crate) const BRED: &str = "\x1b[01;38;5;1m";
pub(crate) const RES: &str = "\x1b[0m";

pub(crate) const APP_NAME: &str = "wutag";

/// Default base color for printing files
pub(crate) const DEFAULT_BASE_COLOR: Color = Blue;
/// Default border color for printing formatted tags
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

// Colored options used in the output of `--help`
pub(crate) static APP_ABOUT: Lazy<String> = Lazy::new(|| {
    format!(
        "{}DESCRIPTION: {}{}{}",
        YELLOW,
        GREEN,
        crate_description!(),
        RES
    )
});
// Specify the file-type(s) to filter by. Can be repeated
pub(crate) static FILE_TYPE: Lazy<String> = Lazy::new(|| {
    "Filter results based on file-type. Does not work with '-g|--global'.\n  'f' or 'file':       \
     regular file\n  'd' or 'dir':        directory\n  'l' or 'symlink':    symlink\n  'b' or \
     'block':      block device\n  'c' or 'char':       character device\n  's' or 'socket':     \
     socket\n  'F' or 'fifo':       fifo\n  'x' or 'executable': executable\n 'e' or 'empty':      \
     file or directory with 0 size
        "
    .to_string()
});
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
pub(crate) static EXEC_EXPL: Lazy<String> = Lazy::new(|| {
    format!(
        "{}\n  An example of using this is:\n  \t {}wutag -g search <tag> -x {{..}} set {{/}} \
         <tag2>{}",
        EXEC_BATCH_EXPL.to_string(),
        BRCYAN,
        RES
    )
});
pub(crate) static OVERRIDE_HELP: Lazy<String> = Lazy::new(|| {
    format!(
        "{}wutag{} [{}FLAGS{}/{}OPTIONS{}] <{}SUBCOMMAND{}> [{}TAGS{}/{}FLAGS{}]",
        BRED, RES, GREEN, RES, GREEN, RES, YELLOW, RES, GREEN, RES, GREEN, RES
    )
});
pub(crate) static AFTER_HELP: Lazy<String> = Lazy::new(|| {
    format!(
        "See {}wutag{} {}--help{} for longer explanations of some base options.\nUse {}--help{} \
         after a subcommand for explanations of more options.",
        BRED, RES, GREEN, RES, GREEN, RES
    )
});
#[rustfmt::skip]
pub(crate) static APP_AUTHORS: Lazy<String> = Lazy::new(|| format!(
    "{}Wojciech K\u{119}pka{} <{}Wwojciech@wkepka.dev{}>\n\
    {}Lucas Burns{}   <{}lmb@lmburns.com{}>",
    BRRED, RES, BRGREEN, RES, BRRED, RES, BRGREEN, RES,
));
pub(crate) static DEFAULT_EDITOR: Lazy<String> =
    Lazy::new(|| std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string()));
