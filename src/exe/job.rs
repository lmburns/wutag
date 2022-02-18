//! Execute a search for tags asynchronously. Optionally execute a
//! command on each result. Outline came from [fd](https://github.com/sharkdp/fd)
use std::{
    borrow::Cow,
    ffi::OsStr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(crate) use crate::{
    filesystem::{contained_path, osstr_to_bytes},
    global_opts,
    subcommand::{search::SearchOpts, App},
    util::{fmt_local_path, fmt_path, fmt_tag, raw_local_path, regex_builder},
    wutag_error,
};

use super::{
    exits::{generalize_exitcodes, ExitCode},
    CommandTemplate,
};

use crossbeam_channel::{Receiver, Sender};
use crossbeam_utils::thread;
// use rayon::prelude::*;
use colored::Colorize;
use regex::bytes::Regex;

/// Result from a multi-threaded command
#[allow(variant_size_differences)]
pub(crate) enum WorkerResult {
    /// Entry and its' id
    Entry((PathBuf, usize)),

    /// An error
    #[allow(dead_code)] // Never constructed
    Error(std::io::Error),
}

/// Spawn a receiver channel that prints the result by default, but will execute
/// a command on the result if `-x|--exec` or `-X|--exec-batch` if passed using
/// `generate_and_execute` or `generate_and_execute_batch` from
/// [CommandTemplate](crate::exe::CommandTemplate)
pub(crate) fn receiver(
    app: &Arc<App>,
    opts: &Arc<SearchOpts>,
    cmd: Option<Arc<CommandTemplate>>,
    rx: Receiver<WorkerResult>,
) -> std::thread::JoinHandle<ExitCode> {
    let app = Arc::clone(app);
    let opts = Arc::clone(opts);

    let threads = num_cpus::get();

    std::thread::spawn(move || {
        if let Some(ref command) = cmd {
            if command.in_batch_mode() {
                let paths = rx.iter().filter_map(|value| match value {
                    WorkerResult::Entry((entry, _id)) => Some(entry),
                    WorkerResult::Error(err) => {
                        wutag_error!("{}", err.to_string());
                        None
                    },
                });

                command.generate_and_execute_batch(paths)
            } else {
                let shared_rx = Arc::new(Mutex::new(rx));
                let out_perm = Arc::new(Mutex::new(()));

                let exits = thread::scope(|s| {
                    let mut results = Vec::new();
                    for _ in 0..threads {
                        let command = Arc::clone(command);
                        let out_perm = Arc::clone(&out_perm);
                        let rx = Arc::clone(&shared_rx);

                        results.push(s.spawn(move |_| {
                            let mut inner: Vec<ExitCode> = Vec::new();

                            loop {
                                let lock = rx.lock().expect("failed to lock receiver");
                                let value: PathBuf = match lock.recv() {
                                    Ok(WorkerResult::Entry((entry, _id))) => entry,
                                    Ok(WorkerResult::Error(err)) => {
                                        wutag_error!("{}", err.to_string());
                                        continue;
                                    },
                                    Err(_) => break,
                                };

                                inner.push(
                                    command.generate_and_execute(&value, &Arc::clone(&out_perm)),
                                );
                            }
                            generalize_exitcodes(inner)
                        }));
                    }
                    results
                        .into_iter()
                        .map(thread::ScopedJoinHandle::join)
                        .collect::<Result<_, _>>()
                })
                .expect("failed to unwrap scope thread")
                .expect("failed to unwrap scope thread");

                generalize_exitcodes(exits)
            }
        } else {
            for result in rx {
                match result {
                    WorkerResult::Entry((entry, id)) => {
                        if opts.raw {
                            global_opts!(
                                raw_local_path(
                                    entry.display().to_string(),
                                    app.base_dir.display().to_string(),
                                ),
                                entry.display().to_string(),
                                app,
                                opts.garrulous
                            );
                        } else {
                            global_opts!(
                                fmt_local_path(
                                    &entry,
                                    &app.base_dir,
                                    app.base_color,
                                    app.ls_colors,
                                ),
                                fmt_path(&entry, app.base_color, app.ls_colors),
                                app,
                                opts.garrulous
                            );
                        }

                        if opts.only_files && !app.quiet {
                            println!();
                        } else {
                            let tags = app
                                .registry
                                .list_entry_tags(id)
                                .unwrap_or_default()
                                .iter()
                                .map(|t| {
                                    if opts.raw {
                                        t.name().to_owned()
                                    } else {
                                        fmt_tag(t).to_string()
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join(" ");

                            if opts.garrulous && !app.quiet {
                                println!("\t{}", tags);
                            } else if !app.quiet {
                                println!(": {}", tags);
                            }
                        }
                    },
                    WorkerResult::Error(err) => {
                        wutag_error!("{}", err.to_string());
                    },
                }
            }
            ExitCode::Success
        }
    })
}

/// Spawn a sender channel that filters results and `sends` them to
/// [receiver](self::receiver)
pub(crate) fn sender(
    app: &Arc<App>,
    opts: &Arc<SearchOpts>,
    re: &Arc<Regex>,
    tx: Sender<WorkerResult>,
) {
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
            // Repeated code from calling function to run on multiple threads
            for (&id, entry) in app.registry.list_entries_and_ids() {
                if !app.global && !contained_path(entry.path(), &app.base_dir) {
                    continue;
                }

                let search_str: Cow<OsStr> = Cow::Owned(entry.path().as_os_str().to_os_string());
                let search_bytes = osstr_to_bytes(search_str.as_ref());

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
                    if !opts.tags.is_empty()
                        && ((opts.only_all
                            && !opts.all
                            && !app.registry.entry_has_only_all_tags(id, &opts.tags))
                            || (!opts.only_all
                                && opts.all
                                && !app.registry.entry_has_all_tags(id, &opts.tags))
                            || (!opts.only_all
                                && !opts.all
                                && !app.registry.entry_has_any_tags(id, &opts.tags)))
                    {
                        continue;
                    }

                    tx_thread
                        .send(WorkerResult::Entry((entry.path().to_owned(), id)))
                        .expect("failed to send result across threads");
                }
            }
        });
    })
    .expect("failed to unwrap scope thread");
}
