//! `clean-cache` - Remove all paths that are found within the `Registry`

use super::App;
use crate::{utils, wutag_error, wutag_info, xattr::tag::DirEntryExt};
use anyhow::Result;
use colored::Colorize;

// TODO: List all files, go through and remove extended attributes
// TODO: Use `delete_dangling_tags` as an option instead of clearing entire DB

impl App {
    /// Clean the database by removing all entries
    pub(crate) fn clean_cache(&mut self) -> Result<()> {
        let reg = self.registry.lock().expect("poisoned lock");
        if utils::prompt("Are you sure you want to clean the registry?", &reg.path()) {
            // Clear xattrs before database is cleared
            for ftag in reg.filetags()?.iter() {
                if let Ok(file) = reg.file(ftag.file_id()) {
                    if let Err(e) = (&file.path()).clear_tags() {
                        wutag_error!(
                            "{}: failed to clear xattrs: {}",
                            self.fmt_path(file.path()),
                            e
                        );
                    } else {
                        println!("CLEARING XATTR: {:?}", ftag);
                        log::debug!("{}: cleared xattrs", self.fmt_path(file.path()));
                    }
                }
            }

            if let Err(e) = reg.clear_registry() {
                wutag_error!("failed to clear registry: {}", e);
            } else {
                println!(
                    "{} {}: {}",
                    "\u{2714}".green().bold(),
                    "CACHE CLEARED".red().bold(),
                    reg.path().clone().to_string_lossy().green().bold()
                );
            }
        } else {
            wutag_info!(
                "{} was not cleared",
                reg.path().to_string_lossy().yellow().bold()
            );
        }

        Ok(())
    }
}
