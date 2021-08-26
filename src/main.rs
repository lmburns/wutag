mod comp_helper;
mod config;
mod consts;
mod exe;
mod filesystem;
mod macros;
mod opt;
mod registry;
mod subcommand;
mod util;

use config::Config;
use log::LevelFilter;
use opt::Opts;
use subcommand::App;

fn main() {
    let config = Config::load_default_location().unwrap_or_default();
    let args = Opts::get_args();

    env_logger::Builder::new()
        .filter(None, match &args.verbose {
            1 => LevelFilter::Warn,
            2 => LevelFilter::Info,
            3 => LevelFilter::Debug,
            4 => LevelFilter::Trace,
            _ => LevelFilter::Off,
        })
        .init();

    if let Err(e) = App::run(args, config) {
        eprintln!("{}", e);
    }
}
