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
    wutag_error, wutag_fatal,
    xattr::tag::DirEntryExt,
};
use anyhow::{anyhow, Context, Result};
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
        value_names = &["from", "to"],
        value_hint = ValueHint::FilePath,
        validator = |t| parse_path(t)
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
                            e => {
                                log::debug!("{}: metadata error: {}", path.display(), e);
                            },
                        },
                        Err(err) => {
                            log::debug!("{}: failed to downcast error", bold_entry!(path));
                        },
                    }
                    continue;
                },
            }
        }

        let manual_repair = |from: &File, to: &PathBuf| -> Result<()> {
            let from_path = &from.path();
            match fs::metadata(to) {
                Ok(cm) =>
                    if let Err(e) = reg.update_file(from.id(), &to) {
                        wutag_error!(
                            "{}: failed to update file {} => {}",
                            e,
                            bold_entry!(from_path),
                            bold_entry!(to)
                        );
                    },
                Err(e) => {
                    match e.kind() {
                        io::ErrorKind::NotFound => {
                            log::debug!("{}: not found on filesystem", to.display());
                        },
                        io::ErrorKind::PermissionDenied => {
                            wutag_error!(
                                "{}: user does not have correct permissions",
                                bold_entry!(to)
                            );
                        },
                        e => {
                            log::debug!("{}: metadata error: {}", to.display(), e);
                        },
                    }
                    return Err(anyhow!(""));
                },
            }
            Ok(())
        };

        if let Some(manual) = &opts.manual {
            // There is only two items in this vector
            let from = manual
                .get(0)
                .and_then(|f| fs::canonicalize(f).ok())
                .context("failed to canonicalize 'from' path")?;
            let to = manual
                .get(1)
                .and_then(|f| fs::canonicalize(f).ok())
                .context("failed to canonicalize 'to' path")?;

            // let db_to = reg.file_by_path(&to).unwrap_or_else(|_| {
            //     wutag_fatal!("{}: not found in registry (to path)", self.fmt_path(to))
            // });

            let mut manual_tbl = vec![];
            log::debug!("manual repair in {}", from.display());
            match reg.file_by_path(&from) {
                Ok(db_from) => {
                    if !opts.dry_run {
                        manual_repair(&db_from, &to)?;
                    }

                    match reg.files_by_directory(&from) {
                        Ok(dir_files) =>
                            for file in dir_files.iter() {
                                let file_path = &file.path();
                                let new = file.clone().set_directory(
                                    to.parent().context("failed to get parent of 'to' path")?,
                                )?;
                                let new_path = &new.path();

                                manual_tbl.push(vec![
                                    self.fmt_path(file_path).cell(),
                                    "=>".green().bold().cell().justify(Justify::Center),
                                    self.fmt_path(new_path).cell(),
                                ]);

                                match fs::metadata(new_path) {
                                    Ok(m) =>
                                        if !opts.dry_run {
                                            manual_repair(file, new_path)?;
                                        },
                                    Err(e) => wutag_fatal!(
                                        "{}: failed to get metadata: {}",
                                        new_path.display(),
                                        e
                                    ),
                                }
                            },
                        Err(e) =>
                            wutag_fatal!("failed to retrieve files under {}", db_from.directory()),
                    }
                },
                Err(e) => wutag_fatal!("{}: not found in registry (from path)", bold_entry!(from)),
            }

            if !self.quiet && !manual_tbl.is_empty() {
                println!("{}:", "Updated".purple().bold().underline());

                print_stdout(
                    manual_tbl
                        .table()
                        .border(Border::builder().build())
                        .separator(Separator::builder().build()),
                )
                .context("failed to print table")?;
            }

            // println!("From: {:#?}", db_from);
            // println!("To: {:#?}", db_to);
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

            if !opts.dry_run {
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

                if let Err(e) = reg.delete_filetag_by_fileid(entry.id()) {
                    wutag_error!("{}: failed to delete FileTag: {}", bold_entry!(path), e);
                    return Ok(());
                }
            }

            println!("{}: removed", self.fmt_path(path));

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
        log::debug!("deleting invalid Files");
        for entry in &invalid {
            let path = &entry.path();
            if opts.remove {
                clean_single_items(entry)?;
            } else {
                println!("{}: missing", self.fmt_path(path));
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

        Ok(())
    }
}
