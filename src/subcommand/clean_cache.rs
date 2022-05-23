//! `clean-cache` - Remove all paths that are found within the `Registry`

use super::App;
use crate::{util::prompt, wutag_error, wutag_info};
use colored::Colorize;

// TODO: List all files, go through and remove extended attributes
// TODO: Use `delete_dangling_tags` as an option instead of clearing entire DB

impl App {
    /// Clean the database by removing all entries
    pub(crate) fn clean_cache(&mut self) {
        if prompt(
            "Are you sure you want to clean the registry?",
            &self.oregistry.path,
        ) {
            self.oregistry.clear();
            if let Err(e) = self.oregistry.save() {
                wutag_error!("{:?}", e);
            } else {
                println!(
                    "{} {}: {}",
                    "\u{2714}".green().bold(),
                    "CACHE CLEARED".red().bold(),
                    self.oregistry
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
                self.oregistry.path.display().to_string().yellow().bold()
            );
        }
    }
}
