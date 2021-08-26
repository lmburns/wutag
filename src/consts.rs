use clap::crate_description;
use colored::Color::{self, *};
use lazy_static::lazy_static;

pub const YELLOW: &str = "\x1b[0;33m";
pub const GREEN: &str = "\x1b[0;32m";
pub const BRCYAN: &str = "\x1b[38;5;14m";
pub const BRGREEN: &str = "\x1b[38;5;10m";
pub const BRRED: &str = "\x1b[38;5;9m";
pub const BRED: &str = "\x1b[01;38;5;1m";
pub const RES: &str = "\x1b[0m";

pub const APP_NAME: &str = "wutag";

/// Default base color for printing files
pub const DEFAULT_BASE_COLOR: Color = Color::Blue;
/// Default border color for printing formatted tags
pub const DEFAULT_BORDER_COLOR: cli_table::Color = cli_table::Color::White;
/// Default max depth passed to [WalkParallel](ignore::WalkParallel)
pub const DEFAULT_MAX_DEPTH: usize = 2;
/// Default colors used for tags
pub const DEFAULT_COLORS: &[Color] = &[
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
lazy_static! {
    pub static ref APP_ABOUT: String = format!(
        "{}DESCRIPTION: {}{}{}", YELLOW, GREEN, crate_description!(), RES
    );
        // Specify the file-type(s) to filter by. Can be repeated
    pub static ref FILE_TYPE: String =
        "Filter results based on file-type. Does not work with '-g|--global'.\n  \
            'f' or 'file':       regular file\n  \
            'd' or 'dir':        directory\n  \
            'l' or 'symlink':    symlink\n  \
            'b' or 'block':      block device\n  \
            'c' or 'char':       character device\n  \
            's' or 'socket':     socket\n  \
            'F' or 'fifo':       fifo\n  \
            'x' or 'executable': executable\n \
            'e' or 'empty':      file or directory with 0 size
        ".to_string();
    #[rustfmt::skip]
    pub static ref EXEC_BATCH_EXPL: String = format!(
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
    );
    pub static ref EXEC_EXPL: String = format!(
        "{}\n  \
       An example of using this is:\n  \
       \t {}wutag -g search <tag> -x {{..}} set {{/}} <tag2>{}",
       EXEC_BATCH_EXPL.to_string(), BRCYAN, RES
    );
    pub static ref OVERRIDE_HELP: String = format!(
        "{}wutag{} [{}FLAGS{}/{}OPTIONS{}] <{}SUBCOMMAND{}> [{}TAGS{}/{}FLAGS{}]",
        BRED, RES, GREEN, RES, GREEN, RES, YELLOW, RES, GREEN, RES, GREEN, RES
    );
    pub static ref AFTER_HELP: String = format!(
        "See {}wutag{} {}--help{} for longer explanations of some base options.\n\
        Use {}--help{} after a subcommand for explanations of more options.",
        BRED, RES, GREEN, RES, GREEN, RES
    );
    #[rustfmt::skip]
    pub static ref APP_AUTHORS: String = format!(
        "{}Wojciech Kępka{} <{}Wwojciech@wkepka.dev{}>\n\
        {}Lucas Burns{}   <{}lmb@lmburns.com{}>",
        BRRED, RES, BRGREEN, RES, BRRED, RES, BRGREEN, RES,
    );
    pub static ref DEFAULT_EDITOR: String = std::env::var("EDITOR")
        .unwrap_or_else(|_| "vim".to_string());
}
