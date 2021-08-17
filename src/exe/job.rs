use std::{
    path::PathBuf,
    sync::{
        mpsc::{channel, Receiver, Sender},
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

#[derive(Debug, Error)]
pub enum Error {
    #[error("path `{0}` caused error")]
    PathError(String),
}

pub enum WorkerResult {
    Entry(PathBuf),
    Error(Error),
}

pub fn run_command(
    app: Arc<&App>,
    command: &Arc<CommandTemplate>,
    tags: &Arc<Vec<String>>,
) -> Result<ExitCode, ()> {
    let (tx, rx) = channel();
    // Spawn the thread that receives all results through the channel.
    let receiver_thread = spawn_receiver(&command, rx);
    // Spawn the sender threads.
    spawn_senders(app, &tags, tx);
    // Wait for the receiver thread to print out all results.
    let exit_code = receiver_thread.join().unwrap();
    Ok(exit_code)
}

fn spawn_receiver(
    command: &Arc<CommandTemplate>,
    rx: Receiver<WorkerResult>,
) -> thread::JoinHandle<ExitCode> {
    let command = Arc::clone(command);
    let show_filesystem_errors = true;
    let threads = num_cpus::get();

    thread::spawn(move || {
        if command.in_batch_mode() {
            batch(rx, command.as_ref(), show_filesystem_errors)
        } else {
            let shared_rx = Arc::new(Mutex::new(rx));
            let out_perm = Arc::new(Mutex::new(()));

            // Each spawned job will store it's thread handle in here.
            let mut handles = Vec::with_capacity(threads);
            for _ in 0..threads {
                let rx = Arc::clone(&shared_rx);
                let cmd = Arc::clone(&command);
                let out_perm = Arc::clone(&out_perm);

                // Spawn a job thread that will listen for and execute inputs.
                let handle = thread::spawn(move || job(rx, cmd, out_perm, show_filesystem_errors));

                // Push the handle of the spawned thread into the vector for later joining.
                handles.push(handle);
            }

            // Wait for all threads to exit before exiting the program.
            let mut results: Vec<ExitCode> = Vec::new();
            for h in handles {
                results.push(h.join().unwrap());
            }

            generalize_exitcodes(&results)
        }
        // ExitCode::Success
    })
}

fn spawn_senders(app: Arc<&App>, tags: &Arc<Vec<String>>, tx: Sender<WorkerResult>) {
    let app = Arc::clone(&app);
    let tags = Arc::clone(tags);
    let tx_thread = tx.clone();

    Box::new(move |app: Arc<App>| {
        for id in app.registry.list_entries_with_tags(tags.iter()) {
            let path = match app.registry.get_entry(id) {
                Some(entry) =>
                    if !app.global && !contained_path(entry.path(), &app.base_dir) {
                        continue;
                    } else {
                        entry.path()
                    },
                None => continue,
            };
            tx_thread.send(WorkerResult::Entry(path.to_owned()));
        }

        // if send_result.is_err() {
        //     return WorkerResult::Error;
        // }
    });
}

/// An event loop that listens for inputs from the `rx` receiver. Each received
/// input will generate a command with the supplied command template. The
/// generated command will then be executed, and this process will continue
/// until the receiver's sender has closed.
pub fn job(
    rx: Arc<Mutex<Receiver<WorkerResult>>>,
    cmd: Arc<CommandTemplate>,
    out_perm: Arc<Mutex<()>>,
    show_filesystem_errors: bool,
) -> ExitCode {
    let mut results: Vec<ExitCode> = Vec::new();
    loop {
        // Create a lock on the shared receiver for this thread.
        let lock = rx.lock().unwrap();

        // Obtain the next result from the receiver, else if the channel
        // has closed, exit from the loop
        let value: PathBuf = match lock.recv() {
            Ok(WorkerResult::Entry(val)) => val,
            Ok(WorkerResult::Error(err)) => {
                if show_filesystem_errors {
                    eprintln!("{}", fmt_err(err.to_string()));
                }
                continue;
            },
            Err(_) => break,
        };

        // Drop the lock so that other threads can read from the receiver.
        drop(lock);
        // Generate a command, execute it and store its exit code.
        results.push(cmd.generate_and_execute(&value, Arc::clone(&out_perm)))
    }
    // Returns error in case of any error.
    generalize_exitcodes(&results)
}

pub fn batch(
    rx: Receiver<WorkerResult>,
    cmd: &CommandTemplate,
    show_filesystem_errors: bool,
) -> ExitCode {
    let paths = rx.iter().filter_map(|value| match value {
        WorkerResult::Entry(val) => Some(val),
        WorkerResult::Error(err) => {
            if show_filesystem_errors {
                eprintln!("{}", fmt_err(err.to_string()));
            }
            None
        },
    });
    cmd.generate_and_execute_batch(paths)
}
