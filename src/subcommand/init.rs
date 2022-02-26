//! Initialize the database

use super::App;
use crate::{wutag_error, wutag_info};
use anyhow::Result;
use clap::Args;

// Here for possible future use

/// Options for the `init` subcommand
#[non_exhaustive]
#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct InitOpts {}

impl App {
    /// Initialize the database
    pub(crate) fn init(&self, opts: &InitOpts) -> Result<()> {
        let reg = self.registry.lock().expect("poisoned lock");
        log::debug!("InitOpts: {:#?}", opts);
        log::debug!("Using registry: {:#?}", reg);

        // The file is created when Connection::open is used
        if reg.get_current_version().is_ok() {
            wutag_error!("The database {} has already been initialized", reg);
        } else {
            reg.init()?;
            wutag_info!("Just initialized a registry at: {}", reg);
        }

        Ok(())
    }
}
