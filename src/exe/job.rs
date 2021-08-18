use std::{
    path::PathBuf,
    sync::{
        // mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

use crate::{app::App, util::{fmt_err, contained_path}};

use super::{
    exits::{generalize_exitcodes, ExitCode},
    CommandTemplate,
};

use thiserror::Error;
use rayon::prelude::*;
use crossbeam_channel as channel;
use crossbeam_channel::{Sender, Receiver, drop};

#[derive(Debug, Error)]
pub enum Error {
    #[error("path `{0}` caused error")]
    PathError(String),
}

pub enum WorkerResult {
    Entry(PathBuf),
    Error(Error),
}

pub fn run_cmd(
    app: Arc<&App>,
    command: &Arc<CommandTemplate>,
    tags: &Arc<Vec<String>>,
) {
    let threads = num_cpus::get();

    rayon::scope(|scope| {
        let (tx, rx) = channel::unbounded::<PathBuf>();
        scope.spawn(move |_| {
            if command.in_batch_mode() {
                command.generate_and_execute_batch(rx.iter().collect());
            } else {
                let mut handles = Vec::with_capacity(threads);
                let out_perm = Arc::new(Mutex::new(()));
                for _ in 0..threads {
                    let out_perm = Arc::clone(&out_perm);
                    let handle = scope.spawn(move |_| {
                        let mut results: Vec<ExitCode> = Vec::new();
                        loop {
                            let value: PathBuf = match rx.recv() {
                                Ok(val) => val,
                                Err(_) => break,
                            };

                            results.push(command.generate_and_execute(&value, Arc::clone(&out_perm)))
                        }
                        generalize_exitcodes(&results);
                    });
                    handles.push(handle);
                }
                let mut results: Vec<ExitCode> = Vec::new();
                for h in handles {
                    results.push(h.join().unwrap());
                }
                generalize_exitcodes(&results);
            }
        });

        // let app = Arc::clone(&app);
        // let tags = Arc::clone(tags);
        // let tx_thread = tx.clone();
        //
        // Box::new(move |app: Arc<App>| {
        //     for id in app.registry.list_entries_with_tags(tags.iter()) {
        //         match app.registry.get_entry(id) {
        //             Some(entry) =>
        //                 if !app.global && !contained_path(entry.path(), &app.base_dir) {
        //                     continue;
        //                 } else {
        //                     tx_thread.send(entry.to_owned());
        //                 },
        //             None => continue,
        //         }
        //     }
        // }
    });
}

fn spawn_senders(app: Arc<&App>, tags: &Arc<Vec<String>>, tx: Sender<WorkerResult>) {
    let app = Arc::clone(&app);
    let tags = Arc::clone(tags);
    let tx_thread = tx.clone();

    Box::new(move |app: Arc<App>| {
        for id in app.registry.list_entries_with_tags(tags.iter()) {
            match app.registry.get_entry(id) {
                Some(entry) =>
                    if !app.global && !contained_path(entry.path(), &app.base_dir) {
                        continue;
                    } else {
                        tx_thread.send(WorkerResult::Entry(entry.to_owned()));
                    },
                None => continue,
            }
        }
    });
}
