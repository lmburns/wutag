// TODO: Manual
// TODO: confirm all options work
#![allow(unused)]

use super::App;
use crate::{
    bold_entry, fail,
    filesystem::contained_path,
    registry::{
        common::{hash, utils::convert_to_datetime},
        types::{file::FileIds, File, Tag},
    },
    utils::{parse_path, systemtime_to_datetime},
    wutag_error,
    xattr::tag::DirEntryExt,
};
use anyhow::{Context, Result};
use clap::{Args, ValueHint};
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, Table,
};
use colored::Colorize;
use itertools::Itertools;
use rayon::prelude::*;
use std::{
    fs::{self, Metadata},
    io,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
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

    /// Restrict the repairing to the CWD, or the path given with -d
    #[clap(name = "border", long = "restrict", short = 'r')]
    pub(crate) restrict: bool,

    // XXX: Implement
    /// Manually set a file's new location
    #[clap(
        name = "manual",
        long = "manual",
        short = 'm',
        takes_value = true,
        number_of_values = 2,
        // value_terminator = ";",
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

        // All files that are in the registry and in the search path
        let mut all = vec![];
        // Files that are the same as they were when entered into registry
        let mut unmodified = vec![];
        // Files that have been modified in some since being added to registry
        let mut modified = vec![];
        // Files that cannot be located on the system anymore
        let mut invalid = vec![];

        let reg = self.registry.lock().expect("poisoned lock");

        for entry in reg.files(None)?.iter().cloned() {
            if (!self.global || opts.restrict) && !self.contained_path(entry.path()) {
                continue;
            }

            let path = &entry.path();
            println!("PATH: {:?}", path);

            all.push(entry.clone());

            match entry.get_fs_metadata() {
                Ok(cm) =>
                    if entry.changed_since(&cm)? {
                        unmodified.push(entry);
                    } else {
                        modified.push(entry);
                    },
                Err(e) => {
                    match e.downcast::<io::Error>() {
                        Ok(inner) => match inner.kind() {
                            io::ErrorKind::NotFound => {
                                log::debug!("{}: not found on filesystem", path.display());
                                invalid.push(entry);
                            },
                            io::ErrorKind::PermissionDenied => {
                                wutag_error!(
                                    "{}: user does not have correct permissions",
                                    bold_entry!(path)
                                );
                            },
                            _ => {},
                        },
                        Err(err) => {
                            log::debug!("{}: failed to downcast error", bold_entry!(path));
                        },
                    }
                    continue;
                },
            }
        }

        // Remove extended attributes from a given [`PathBuf`]
        let handle_xattr = |path: &PathBuf, tag: &Tag| {
            if path.get_tag(tag).is_err() {
                wutag_error!(
                    "{}: found ({}) in database, though file has no xattrs",
                    bold_entry!(path),
                    self.fmt_tag(tag)
                );
            } else if let Err(e) = path.untag(tag) {
                wutag_error!("{}: {}", path.display(), e);
            }
        };

        // Remove tags or values that aren't connected to any File
        let clean_single_items = |entry: &File| -> Result<()> {
            let path = &entry.path();

            for tag in reg.unique_tags_by_file(entry.id())?.iter() {
                for value in reg.unique_values_by_tag(tag.id())?.iter() {
                    log::debug!("{}: deleting value {}", path.display(), value.name());
                    if let Err(e) = reg.delete_value(value.id()) {
                        wutag_error!(
                            "{}: failed to delete value {}: {}",
                            bold_entry!(path),
                            value.name().color(self.base_color).bold(),
                            e
                        );
                    }
                }

                log::debug!("{}: deleting tag {}", path.display(), tag.name());
                if let Err(e) = reg.delete_tag(tag.id()) {
                    wutag_error!(
                        "{}: failed to delete tag {}: {}",
                        bold_entry!(path),
                        self.fmt_tag(tag),
                        e
                    );
                }

                if path.get_tag(tag).is_err() {
                    wutag_error!(
                        "{}: found ({}) in database, though file has no xattrs",
                        bold_entry!(path),
                        self.fmt_tag(tag)
                    );
                } else if let Err(e) = path.untag(tag) {
                    wutag_error!("{}: {}", path.display(), e);
                }
            }

            match reg.delete_filetag_by_fileid(entry.id()) {
                Ok(_) =>
                    if !self.quiet {
                        println!("{}: removed", self.fmt_path(path));
                    },
                Err(e) => wutag_error!("{}: failed to delete FileTag: {}", bold_entry!(path), e),
            }

            Ok(())
        };

        for entry in &modified {
            let path = &entry.path();

            log::debug!("{}: updating file metadata (modified)", path.display());
            if !opts.dry_run {
                if let Err(e) = reg.update_file(entry.id(), path) {
                    wutag_error!("{}: failed to update file path: {}", bold_entry!(path), e);
                    continue;
                }
            }

            println!(
                "{}: updated fingerprint ({})",
                self.fmt_path(path),
                "M".red().bold()
            );
        }

        if opts.unmodified {
            for entry in &unmodified {
                let path = &entry.path();

                log::debug!("{}: updating file metadata (unmodified)", path.display());
                if !opts.dry_run {
                    if let Err(e) = reg.update_file(entry.id(), path) {
                        wutag_error!("{}: failed to update file path: {}", bold_entry!(path), e);
                        continue;
                    }
                }

                println!(
                    "{}: updated fingerprint ({})",
                    self.fmt_path(path),
                    "U".green().bold()
                );
            }
        }

        // Delete invalid Files from the Registry
        if opts.remove {
            log::debug!("deleting invalid Files");
            for entry in &invalid {
                let path = &entry.path();
                clean_single_items(entry)?;
            }
        }

        // Delete untagged files (there shouldn't be any)
        if let Ok(untagged) =
            reg.files_untagged_by_fileids(&(all.iter().map(File::id).collect::<FileIds>()))
        {
            log::debug!("deleting untagged Files");
            for entry in untagged.iter() {
                let path = &entry.path();
                clean_single_items(entry)?;
            }
        }

        // println!("MODIFIEDx: {:#?}", modified);
        // println!("UNMODIFIED: {:#?}", unmodified);
        // println!("INVALID: {:#?}", invalid);

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
