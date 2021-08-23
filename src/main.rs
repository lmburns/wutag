mod app;
mod comp_helper;
mod config;
mod exe;
mod filesystem;
mod macros;
mod opt;
mod registry;
mod util;

use colored::Color::{self, *};
// use std::io::Write;

use app::App;
use config::Config;
use log::LevelFilter;
use opt::Opts;

/// Default base color for printing files
pub(crate) const DEFAULT_BASE_COLOR: Color = Color::Blue;
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

fn main() {
    let config = Config::load_default_location().unwrap_or_default();
    let args = Opts::get_args();

    env_logger::Builder::new()
        .filter(None, match &args.verbose {
            1 => LevelFilter::Info,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Debug,
            4 => LevelFilter::Trace,
            _ => LevelFilter::Off,
        })
        .init();

    if let Err(e) = App::run(args, config) {
        eprintln!("{}", e);
    }
}
