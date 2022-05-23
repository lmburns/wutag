//! `info` - Show information about the database and the `Tag`s in it

// TODO: Entire subcommand

#![allow(unused)]

use super::App;
use crate::util::{fmt_tag, fmt_tag_old};
use clap::{Args, Subcommand};
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, ColorChoice, Style, Table,
};
use colored::Colorize;
use std::collections::HashMap;

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
    pub(crate) fn info(&mut self, opts: &InfoOpts) {
        log::debug!("InfoOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.oregistry.path.display());

        println!(
            "{}: {}",
            "Registry".magenta(),
            self.oregistry.path.display().to_string().green()
        );

        // -- INFO --
        // List number of files
        // List number of tags
        // List number of values
        // List dangling items
        // List size of database
        // List location of database
        // Could list deleted (take index ID - count found)

        // let (w, _) = crossterm::terminal::size().unwrap_or((80, 40));
        //
        // let mut table = vec![];
        //
        // for tag in self.oregistry.list_tags() {
        //     let mut row = vec![];
        //     let mut cumulative = 0;
        //
        //     cumulative += tag.name().len();
        //
        //     if cumulative < ((w - 4) / 2) as usize {
        //         row.push(tag);
        //     } else {
        //         table.push(row.clone().iter().map(|t|
        // t.cell().justify(Justify::Left)));         row.clear();
        //     }
        // }

        self.oregistry
            .list_tags()
            .map(|tag| {
                if opts.raw {
                    tag.name().white()
                } else {
                    fmt_tag_old(tag)
                }
            })
            .for_each(|t| println!("{}", t));
    }
}
