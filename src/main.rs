mod app;
mod config;
mod opt;
mod registry;
mod util;

use clap::Clap;
use colored::Color::{self, *};

use app::App;
use config::Config;
use opt::Opts;

/// Default max depth passed to [GlobWalker](globwalker::GlobWalker)
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

fn main() {
    let config = Config::load_default_location().unwrap_or_default();

    // let opts = Opts::try_parse();

    // Really need to fix this
    let arglen: Vec<String> = std::env::args().collect();

    if arglen.len() > 1 {
        if let Err(e) = App::run(Opts::parse(), config) {
            eprintln!("{}", e);
        }
    } else if let Err(e) = App::run(Opts::base(), config) {
        eprintln!("{}", e);
    }
}
