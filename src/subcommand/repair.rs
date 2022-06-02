// TODO: Manual
// TODO: confirm all options work
#![allow(unused)]

use super::App;
use crate::{
    fail,
    filesystem::contained_path,
    registry::{common::utils::convert_to_datetime, types::File},
    utils::{parse_path, systemtime_to_datetime},
    wutag_error,
};
use anyhow::{Context, Result};
use clap::{Args, ValueHint};
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, Table,
};
use colored::Colorize;
use std::{
    fs::{self, Metadata},
    io,
    os::unix::fs::{MetadataExt, PermissionsExt},
};

use lexiclean::Lexiclean;

#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct RepairOpts {
    /// Do not actually update the registry
    #[clap(name = "dry_run", long = "dry-run", short = 'd')]
    pub(crate) dry_run: bool,

    /// Remove files from the registry that no longer exist on the system
    #[clap(name = "remove", long = "remove", short = 'R')]
    pub(crate) remove: bool,

    /// Restrict the repairing to the current directory, or the path given with
    /// -d
    #[clap(name = "border", long = "restrict", short = 'r')]
    pub(crate) restrict: bool,

    /// Manually set the file's new location
    #[clap(
        name = "manual",
        long = "manual",
        short = 'm',
        takes_value = true,
        number_of_values = 2,
        value_terminator = ";",
        value_hint = ValueHint::FilePath,
        validator = |t| parse_path(t.split_whitespace().collect::<Vec<_>>()[1])
    )]
    pub(crate) manual: Option<Vec<String>>,

    /// Update the hash sum of all files, including unmodified files
    #[clap(
        name = "unmodified",
        long = "unmodified",
        short = 'u',
        takes_value = true
    )]
    pub(crate) unmodified: bool,
}

impl App {
    /// Repair the database by updating file hashes or removing missing files
    pub(crate) fn repair(&mut self, opts: &RepairOpts) -> Result<()> {
        log::debug!("RepairOpts: {:#?}", opts);

        // let mut table = vec![];
        let mut removed = false;

        let mut unmodified = vec![];
        let mut modified = vec![];
        let mut invalid = vec![];

        let reg = self.registry.lock().expect("poisoned lock");

        for entry in reg.files(None)?.iter() {
            if (!self.global || opts.restrict) && !contained_path(entry.path(), &self.base_dir) {
                continue;
            }

            let path = &entry.path();
            match entry.get_fs_metadata() {
                Ok(cm) =>
                    if entry.changed_since()? {
                        unmodified.push(entry);
                    } else {
                        modified.push(entry);
                    },
                Err(e) => {
                    match e.downcast::<io::Error>() {
                        Ok(inner) => {
                            if inner.kind() == io::ErrorKind::NotFound {
                                log::debug!("{}: not found on filesystem", path.display());
                                invalid.push(entry);
                            }

                            if inner.kind() == io::ErrorKind::PermissionDenied {
                                wutag_error!(
                                    "{}: user does not have correct permissions",
                                    self.fmt_path(entry.path())
                                );
                            }
                        },
                        Err(err) => {
                            log::debug!(
                                "{}: failed to downcast error",
                                self.fmt_path(entry.path())
                            );
                        },
                    }
                    continue;
                },
            }
        }

        // let dir = reg.files_by_directory(&self.base_dir)?;
        // for file in dir.iter() {
        //     println!("UDNER: {}", self.fmt_path(&file.path()));
        // }

        // {
        //
        //     if exists && (entry.changed_since()? || opts.unmodified) {
        //         table.push(vec![
        //             if self.global || !opts.restrict {
        //                 self.fmt_path(entry.path())
        //             } else {
        //                 self.fmt_local_path(entry.path())
        //             }
        //             .cell(),
        //             systemtime_to_datetime(*entry.modtime()).red().cell(),
        //             "=>".yellow().cell().justify(Justify::Center),
        //             systemtime_to_datetime(entry.get_current_modtime()?)
        //                 .green()
        //                 .bold()
        //                 .cell(),
        //         ]);
        //
        //         if !opts.dry_run {
        //             self.oregistry.repair_registry(id)?;
        //         }
        //     }
        //
        //     if !exists && opts.remove {
        //         if !opts.dry_run {
        //             self.oregistry.clear_entry(id);
        //         }
        //
        //         if !self.quiet {
        //             println!(
        //                 "{}: {}",
        //                 "Removed".red().bold(),
        //                 self.fmt_path(entry.path()),
        //             );
        //         }
        //         removed = true;
        //     }
        // }
        //
        // if !self.quiet {
        //     if removed && !table.is_empty() {
        //         println!("\n{}:", "Updated".purple().bold());
        //     }
        //     print_stdout(
        //         table
        //             .table()
        //             .border(Border::builder().build())
        //             .separator(Separator::builder().build()),
        //     )
        //     .context("failed to print table")?;
        // }

        Ok(())
    }
}
