#![allow(unused)]
//! Execute a search for tags asynchronously. Optionally execute a
//! command on each result. Outline came from [fd](https://github.com/sharkdp/fd)

use std::{
    borrow::Cow,
    ffi::OsStr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use super::{
    exits::{generalize_exitcodes, ExitCode},
    CommandTemplate,
};
pub(crate) use crate::{
    filesystem as wfs, global_opts,
    registry::{
        types::{tag::Tag, ID},
        Registry,
    },
    subcommand::{search::SearchOpts, App},
    utils::{fmt, regex_builder},
    wutag_error,
};

use anyhow::Result;
use colored::Colorize;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_utils::thread;
use itertools::Itertools;
// use rayon::prelude::*;
use regex::bytes::Regex;

/// Result from a multi-threaded command
#[derive(Debug)]
#[allow(variant_size_differences)]
pub(crate) enum ChannelResult {
    /// Entry and its' id
    Entry((PathBuf, ID)),

    /// An error
    Error(std::io::Error),
}

/// Generate and execute a command that is not in batch mode
pub(crate) fn process_single(
    rx: &Arc<Mutex<Receiver<ChannelResult>>>,
    command: &Arc<CommandTemplate>,
    out_perm: &Arc<Mutex<()>>,
) -> ExitCode {
    let mut inner = Vec::<ExitCode>::new();

    loop {
        // Lock the shared receiver for the current thread
        let lock = rx.lock().expect("failed to lock receiver");

        // Get the next item from the receiver
        let value: PathBuf = match lock.recv() {
            Ok(ChannelResult::Entry((entry, _id))) => entry,
            Ok(ChannelResult::Error(err)) => {
                wutag_error!("{}", err.to_string());
                continue;
            },
            Err(_) => break,
        };

        drop(lock);

        // Generate the command, return the `ExitCode`
        inner.push(command.generate_and_execute(&value, &Arc::clone(out_perm)));
    }

    generalize_exitcodes(inner)
}

/// Generate and execute a command that is in batch mode
pub(crate) fn process_batch(
    rx: &Receiver<ChannelResult>,
    command: &CommandTemplate,
    limit: usize,
) -> ExitCode {
    let paths = rx.iter().filter_map(|value| match value {
        ChannelResult::Entry((entry, _id)) => Some(entry),
        ChannelResult::Error(err) => {
            wutag_error!("{}", err.to_string());
            None
        },
    });

    command.generate_and_execute_batch(paths)
}

/// Spawn a receiver channel that prints the result by default, but will execute
/// a command on the result if `-x|--exec` or `-X|--exec-batch` if passed using
/// `generate_and_execute` or `generate_and_execute_batch` from
/// [CommandTemplate](crate::exe::CommandTemplate)
pub(crate) fn receiver(
    app: &Arc<App>,
    opts: &Arc<SearchOpts>,
    cmd: Option<Arc<CommandTemplate>>,
    rx: Receiver<ChannelResult>,
) -> std::thread::JoinHandle<ExitCode> {
    let app = Arc::clone(app);
    let opts = Arc::clone(opts);

    let threads = num_cpus::get();

    std::thread::spawn(move || {
        let reg = app.registry.lock().expect("poisoned registry lock");

        if let Some(ref command) = cmd {
            if command.in_batch_mode() {
                self::process_batch(&rx, command, 10)
            } else {
                let shared_rx = Arc::new(Mutex::new(rx));
                let out_perm = Arc::new(Mutex::new(()));

                let exits: Vec<ExitCode> = thread::scope(|s| {
                    let mut results = Vec::new();
                    for _ in 0..threads {
                        let command = Arc::clone(command);
                        let out_perm = Arc::clone(&out_perm);
                        let rx = Arc::clone(&shared_rx);

                        results.push(s.spawn(move |_| self::process_single(&rx, &command, &out_perm)));
                    }

                    results
                        .into_iter()
                        .map(thread::ScopedJoinHandle::join)
                        .collect::<Result<_, _>>()
                })
                .expect("failed to unwrap ScopedJoinHandle")
                .expect("failed to unwrap scope thread");

                generalize_exitcodes(exits)
            }
        } else {
            let raw = |t: &Tag, with_values: bool| {
                let tag = if opts.raw {
                    t.name().clone()
                } else {
                    app.fmt_tag(t).to_string()
                };

                if with_values {
                    // FIX: As of now, only one value per tag because of xattr ???
                    let values = reg.values_by_tagid(t.id()).map_or_else(
                        |_| String::from(""),
                        |values| {
                            format!(
                                "={}",
                                values
                                    .iter()
                                    .map(|value| {
                                        let v = value.name();
                                        tern::t!(
                                            opts.raw
                                                ? v.clone()
                                                : v.bold().to_string()
                                        )
                                    })
                                    .join(",")
                            )
                        },
                    );
                    format!("{}{}", tag, values)
                } else {
                    tag
                }
            };

            for result in rx {
                match result {
                    ChannelResult::Entry((entry, id)) => {
                        if opts.raw {
                            global_opts!(
                                app.fmt_raw_local_path(entry.display().to_string()),
                                entry.display().to_string(),
                                app.global,
                                opts.garrulous
                            );
                        } else {
                            global_opts!(
                                app.fmt_local_path(&entry),
                                app.fmt_path(&entry),
                                app.global,
                                opts.garrulous
                            );
                        }

                        if opts.only_files && !app.quiet {
                            println!();
                        } else {
                            let tags = reg
                                .tags_by_fileid(id)
                                .expect("failed to get Tags by FileId")
                                .iter()
                                .map(|t| raw(t, opts.with_values))
                                .collect::<Vec<_>>()
                                .join(" ");

                            if !app.quiet {
                                if opts.garrulous {
                                    println!("\t{}", tags);
                                } else {
                                    println!(": {}", tags);
                                }
                            }
                        }
                    },
                    ChannelResult::Error(err) => {
                        wutag_error!("{}", err.to_string());
                    },
                }
            }
            ExitCode::Success
        }
    })
}

/// Spawn a sender channel that filters results and `sends` them to
/// [`receiver`]
pub(crate) fn sender(app: &Arc<App>, opts: &Arc<SearchOpts>, re: &Arc<Regex>, tx: Sender<ChannelResult>) {
    let app = Arc::clone(app);
    let opts = Arc::clone(opts);
    let re = Arc::clone(re);

    let exclude_pattern = regex_builder(
        app.exclude.join("|").as_str(),
        app.case_insensitive,
        app.case_sensitive,
    );

    thread::scope(move |s| {
        let tx_thread = tx.clone();

        s.spawn(move |_| {
            // A new connection to the database has to be opened on each thread
            let reg = Registry::from(&app);

            // Repeated code from calling function to run on multiple threads
            for entry in reg.files(None).expect("failed to get Files").iter() {
                if !app.global && !app.contained_path(entry.path()) {
                    continue;
                }

                let search_str: Cow<OsStr> = Cow::Owned(entry.path().as_os_str().to_os_string());
                let search_bytes = wfs::osstr_to_bytes(search_str.as_ref());

                if !app.exclude.is_empty() && exclude_pattern.is_match(&search_bytes) {
                    continue;
                }

                if let Some(ref ext) = app.extension {
                    if !ext.is_match(&search_bytes) {
                        continue;
                    }
                }

                if let Some(ref file_types) = app.file_type {
                    if file_types.should_ignore(&entry.path()) {
                        log::debug!("Ignoring: {}", entry.path().display());
                        continue;
                    }
                }

                if re.is_match(&search_bytes) {
                    // Additional tag search
                    // if !opts.tags.is_empty()
                    //     && ((opts.only_all
                    //         && !opts.all
                    //         && !app.oregistry.entry_has_only_all_tags(id, &opts.tags))
                    //         || (!opts.only_all
                    //             && opts.all
                    //             && !app.oregistry.entry_has_all_tags(id, &opts.tags))
                    //         || (!opts.only_all
                    //             && !opts.all
                    //             && !app.oregistry.entry_has_any_tags(id, &opts.tags)))
                    // {
                    //     continue;
                    // }

                    // TODO: How to prune here?
                    if app.prune && app.contained_path(entry.path()) {
                        println!("PRUNING");
                        continue;
                    }

                    if tx_thread
                        .send(ChannelResult::Entry((entry.path().clone(), entry.id())))
                        .is_err()
                    {
                        wutag_error!("failed to send result across threads: {}", entry.path().display());
                        return;
                    }
                }
            }
        });
    })
    .expect("failed to unwrap scope thread");
}
