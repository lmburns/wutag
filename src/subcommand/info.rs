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
    /// Show the number of deleted items (see --help for calculation)
    #[clap(
        long,
        short,
        long_help = "\
            Note that this is calculated by taking the current number of items in the database and \
                     subtracting the maximum of the auto-incrementing index"
    )]
    deleted: bool,

    /// Show the averages for each item
    #[clap(long, short)]
    mean: bool,

    /// Show everything about the registry (all --flags)
    #[clap(long, short)]
    full: bool,

    /// Do not use color in output
    #[clap(long, short)]
    raw: bool,
}

impl App {
    /// Show information about the database
    pub(crate) fn info(&mut self, opts: &InfoOpts) -> Result<()> {
        log::debug!("InfoOpts: {:#?}", opts);

        let reg = self.registry.lock().expect("poisoned lock");

        /// Shorthand for using the `base_color` to color a string
        let c = |s: &str| -> String {
            if opts.raw {
                s.to_string()
            } else {
                s.color(self.base_color).bold().to_string()
            }
        };

        let path = reg.path().display().to_string();

        // Registry
        println!(
            "{}: {}",
            c("Registry"),
            tern::t!(opts.raw ? path : path.yellow().to_string())
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
        let tag_count = reg.tag_count()?;
        println!("{}: {}", c("Tags"), tag_count);

        // Tag, value count
        println!("{}: {}", c("Tags (with values)"), reg.tag_value_count()?);

        // Value count
        let value_count = reg.value_count()?;
        println!("{}: {}", c("Values"), value_count);

        // File count
        let file_count = reg.file_count()?;
        println!("{}: {}", c("Files"), file_count);

        // File-tag count
        let filetag_count = reg.filetag_count()?;
        println!("{}: {}", c("Taggings"), filetag_count);

        // Dangling tags
        println!("{}: {}", c("Dangling tags"), reg.dangling_tags()?.len());

        if opts.mean || opts.full {
            println!();

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
        }

        if opts.deleted || opts.full {
            println!();

            println!("{}: {}", c("Deleted tags"), reg.tag_max()? - tag_count);
            println!(
                "{}: {}",
                c("Deleted values"),
                reg.value_max()? - value_count
            );
            println!("{}: {}", c("Deleted files"), reg.file_max()? - file_count);
        }

        // Could list deleted (take index ID - count found)

        Ok(())
    }
}
