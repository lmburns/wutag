//! `clean-cache` - Remove all paths that are found within the `Registry`

use super::{
    uses::{prompt, wutag_error, wutag_info, Colorize},
    App,
};

impl App {
    /// Clean the database by removing all old entries and start from a clean
    /// slate
    pub(crate) fn clean_cache(&mut self) {
        if prompt(
            "Are you sure you want to clean the registry?",
            &self.registry.path,
        ) {
            self.registry.clear();
            if let Err(e) = self.registry.save() {
                wutag_error!("{:?}", e);
            } else {
                println!(
                    "{} {}: {}",
                    "\u{2714}".green().bold(),
                    "CACHE CLEARED".red().bold(),
                    self.registry
                        .path
                        .clone()
                        .display()
                        .to_string()
                        .green()
                        .bold()
                );
            }
        } else {
            wutag_info!(
                "{} was not cleared",
                self.registry.path.display().to_string().yellow().bold()
            );
        }
    }
}
