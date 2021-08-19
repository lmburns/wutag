use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub use crate::{
    app::App,
    util::{contained_path, fmt_err},
};

use super::{
    exits::{generalize_exitcodes, ExitCode},
    CommandTemplate,
};

use crossbeam_channel as channel;
use crossbeam_utils::thread;
use rayon::prelude::*;

// TODO: Get rid of this
pub enum WorkerResult {
    Entry(PathBuf),
    #[allow(dead_code)] // Never constructed
    Error(std::io::Error),
}

pub fn run_command(app: &App, command: &Arc<CommandTemplate>, tags: &Arc<Vec<String>>) {
    let threads = num_cpus::get();
    let app = Arc::new(&app);
    let (tx, rx) = channel::unbounded::<WorkerResult>();

    let command = Arc::clone(command);

    let receiver = std::thread::spawn(move || {
        if command.in_batch_mode() {
            // command.generate_and_execute_batch(rx.iter().collect());

            let paths = rx.iter().filter_map(|value| match value {
                WorkerResult::Entry(val) => Some(val),
                WorkerResult::Error(err) => {
                    eprintln!("{}", fmt_err(err.to_string()));
                    None
                },
            });
            // let paths = rx.iter().filter_map(|v| matches!(v, Ok(v) if Some(v)));

            command.generate_and_execute_batch(paths)
        } else {
            let shared_rx = Arc::new(Mutex::new(rx));
            let out_perm = Arc::new(Mutex::new(()));

            let exits = thread::scope(|s| {
                let mut results = Vec::new();
                for _ in 0..threads {
                    let command = Arc::clone(&command);
                    let out_perm = Arc::clone(&out_perm);
                    let rx = Arc::clone(&shared_rx);

                    results.push(s.spawn(move |_| {
                        let mut inner: Vec<ExitCode> = Vec::new();

                        loop {
                            let lock = rx.lock().unwrap();
                            let value: PathBuf = match lock.recv() {
                                Ok(WorkerResult::Entry(val)) => val,
                                Ok(WorkerResult::Error(err)) => {
                                    eprintln!("{}", fmt_err(err.to_string()));
                                    continue;
                                },
                                Err(_) => break,
                            };

                            inner.push(command.generate_and_execute(&value, Arc::clone(&out_perm)))
                        }
                        generalize_exitcodes(inner)
                    }));
                }
                results
                    .into_iter()
                    .map(|h| h.join())
                    .collect::<Result<_, _>>()
            })
            .unwrap()
            .unwrap();

            generalize_exitcodes(exits)
        }
    });

    thread::scope(move |s| {
        let tx_thread = tx.clone();
        s.spawn(move |_| {
            app.registry
                .list_entries_with_tags(tags.iter())
                .par_iter()
                .filter_map(|id| {
                    if let Some(entry) = app.registry.get_entry(*id) {
                        if !app.global && !contained_path(entry.path(), &app.base_dir) {
                            None
                        } else {
                            Some(entry)
                        }
                    } else {
                        None
                    }
                })
                .for_each(|entry| {
                    tx_thread
                        .send(WorkerResult::Entry(entry.path().to_owned()))
                        .unwrap();
                });
        });
    })
    .unwrap();

    receiver.join().unwrap();
}
