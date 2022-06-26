// TODO: Confirm all options work
// TODO: Crawl directory, find items with tags not in directory
#![allow(unused)]

use super::App;
use crate::{
    bold_entry, fail,
    filesystem::contained_path,
    g, r,
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

    // XXX: Implement
    /// Crawl directory for files that have tags but aren't in the registry
    #[clap(name = "crawl", long = "crawl", short = 'c')]
    pub(crate) crawl: bool,

    // /// Repair files that have tags but are not in the registry
    // #[clap(name = "dangling", long = "dangling", short = 'd')]
    // pub(crate) dangling: bool,
    /// Remove files from the registry that no longer exist on the system
    #[clap(name = "remove", long = "remove", short = 'R')]
    pub(crate) remove: bool,

    /// Restrict the repairing to the CWD, or the path given with -d
    #[clap(name = "restrict", long = "restrict", short = 'r')]
    pub(crate) restrict: bool,

    /// Manually set a file's new location
    #[clap(
        name = "manual",
        long = "manual",
        short = 'm',
        takes_value = true,
        number_of_values = 2,
        conflicts_with_all = &["remove", "unmodified", "restrict"],
        value_names = &["from", "to"],
        value_hint = ValueHint::FilePath,
        // Would be nice to have a validator for one argument
        // validator = |t| parse_path(t)
    )]
    pub(crate) manual: Option<Vec<String>>,

    /// Manually update a directory's contents
    #[clap(
        name = "directory",
        long = "directory",
        short = 'D',
        requires = "manual",
        long_help = "Instead of updating a single file, all the files in a directory can be updated at \
                     once. Requires --manual"
    )]
    pub(crate) directory: bool,

    /// Update the hash sum of all files, including unmodified files
    #[clap(name = "unmodified", long = "unmodified", short = 'u')]
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
                            wutag_error!("{}: user does not have correct permissions", bold_entry!(to));
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

        if let Some(manual) = &opts.manual {
            // There is only two items in this vector guaranteed
            let from = manual
                .get(0)
                .and_then(|f| {
                    // Why did I do this part again?
                    let path = PathBuf::from(f);
                    let parent = path
                        .parent()
                        .unwrap_or_else(|| wutag_fatal!("failed to get parent of 'from' path"));
                    let fname = path
                        .file_name()
                        .unwrap_or_else(|| wutag_fatal!("failed to get file name of 'from' path"));
                    fs::canonicalize(parent).ok().map(|f| f.join(fname))
                })
                .context("failed to canonicalize 'from' path")?;
            let to = manual
                .get(1)
                .and_then(|f| fs::canonicalize(f).ok())
                .context("failed to canonicalize 'to' path")?;

            let mut manual_tbl = vec![];
            log::debug!("manual repair in {}", from.display());

            if opts.directory {
                if !from.is_dir() {
                    wutag_fatal!(
                        "{}: is not a directory. Don't use the {} option",
                        bold_entry!(from),
                        "--directory".green()
                    );
                }
                if !to.is_dir() {
                    wutag_fatal!(
                        "{}: is not a directory. Don't use the {} option",
                        bold_entry!(to),
                        "--directory".green()
                    );
                }

                match reg.files_by_directory(&from, false) {
                    Ok(dir_files) => {
                        if dir_files.is_empty() {
                            wutag_error!("No files are tagged within {}", from.display());
                            return Ok(());
                        }

                        for file in dir_files.iter() {
                            let file_path = &file.path();
                            let new = file.clone().set_directory(&to)?;
                            let new_path = &new.path();

                            if fs::metadata(new_path).is_ok() {
                                manual_tbl.push(vec![
                                    self.fmt_path(file_path).cell(),
                                    g!("=>").cell().justify(Justify::Center),
                                    self.fmt_path(new_path).cell(),
                                ]);

                                if !opts.dry_run {
                                    manual_repair(file, new_path)?;
                                }
                            }
                        }
                    },
                    Err(e) => wutag_fatal!("failed to retrieve files under {}", from.display()),
                }
            } else {
                match reg.file_by_path(&from) {
                    Ok(db_from) => {
                        if !opts.dry_run {
                            manual_repair(&db_from, &to)?;
                        }
                        manual_tbl.push(vec![
                            self.fmt_path(db_from.path()).cell(),
                            g!("=>").cell().justify(Justify::Center),
                            self.fmt_path(to).cell(),
                        ]);
                    },
                    Err(e) => wutag_fatal!("{}: not found in registry (from path)", bold_entry!(from)),
                }
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

            return Ok(());
        }

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

            println!("{}: updated fingerprint ({})", self.fmt_path(path), r!("M"));
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

                println!("{}: updated fingerprint ({})", self.fmt_path(path), g!("U"));
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

        Ok(())
    }
}
