//! `info` - Show information about the database and the `Tag`s in it

// TODO: Entire subcommand

#![allow(unused)]
#![allow(clippy::cast_precision_loss)]

use super::App;
use crate::util::{fmt_tag, fmt_tag_old};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, ColorChoice, Style, Table,
};
use colored::{ColoredString, Colorize};
use std::{collections::HashMap, fs, os::unix::fs::MetadataExt};

/// Arguments used for the `info` subcommand
#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct InfoOpts {
    /// TO BE IMPLEMENTED
    /// Do not use color in output
    #[clap(long, short)]
    pub(crate) raw: bool,
}

impl App {
    /// Show information about the database
    pub(crate) fn info(&mut self, opts: &InfoOpts) -> Result<()> {
        log::debug!("InfoOpts: {:#?}", opts);

        let reg = self.registry.lock().expect("poisoned lock");

        /// Shorthand for using the `base_color` to color a string
        let c = |s: &str| -> ColoredString { s.color(self.base_color).bold() };

        // Registry
        println!(
            "{}: {}",
            c("Registry"),
            reg.path().display().to_string().yellow()
        );

        // Filesize
        let label = vec!["b", "Ki", "Mi", "Gi"]; // Gi probably not needed
        let mut i = 0;
        let mut bytes = fs::metadata(reg.path())?.len() as f64;
        while bytes >= 1024.0 {
            bytes /= 1024.0;
            i += 1;
        }

        println!(
            "{}: {:.2}{}",
            c("Size"),
            bytes,
            label.get(i).context("file size is too large")?
        );

        println!();

        // Tag count
        println!("{}: {}", c("Tags"), reg.tag_count()?);

        // Tag, value count
        println!("{}: {}", c("Tags (with values)"), reg.tag_value_count()?);

        // Value count
        let tag_count = reg.value_count()?;
        println!("{}: {}", c("Values"), tag_count);

        // File count
        let file_count = reg.file_count()?;
        println!("{}: {}", c("Files"), file_count);

        // Filetag count
        let filetag_count = reg.filetag_count()?;
        println!("{}: {}", c("Taggings"), filetag_count);

        // Dangling tags
        println!("{}: {}", c("Dangling tags"), reg.dangling_tags()?.len());

        // Means
        println!(
            "{}: {:.2}",
            c("Mean tags per file"),
            if file_count > 0 {
                filetag_count as f32 / file_count as f32
            } else {
                0_f32
            }
        );

        println!(
            "{}: {:.2}",
            c("Mean files per tag"),
            if tag_count > 0 {
                filetag_count as f32 / tag_count as f32
            } else {
                0_f32
            }
        );

        // Could list deleted (take index ID - count found)

        Ok(())
    }
}
