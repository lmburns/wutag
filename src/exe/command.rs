//! Execute the given command passed to `-x` or `-X`

use super::exits::ExitCode;
use crate::wutag_error;
use anyhow::Result;
use argmax::Command;
use std::{
    io::{self, Write},
    sync::Mutex,
};

/// Streams in which output can be written to
struct OutputStreams {
    /// Standard output stream
    stdout: Vec<u8>,
    /// Standard error stream
    stderr: Vec<u8>,
}

/// Represents the buffer where output is written
struct OutputBuffer<'a> {
    output_permission: &'a Mutex<()>,
    output_streams:    Vec<OutputStreams>,
}

impl<'a> OutputBuffer<'a> {
    /// Create a new [`OutputBuffer`]
    fn new(out_perm: &'a Mutex<()>) -> Self {
        Self {
            output_permission: out_perm,
            output_streams:    Vec::new(),
        }
    }

    /// Add a new item to the `output_streams`
    fn push(&mut self, stdout: Vec<u8>, stderr: Vec<u8>) {
        self.output_streams.push(OutputStreams { stdout, stderr });
    }

    /// Write `stdout` and `stderr`
    fn write(self) {
        // If there are no output streams, don't do anything
        if self.output_streams.is_empty() {
            return;
        }

        // When locked, only this thread can write to OutputStreams
        let _lock = self.output_permission.lock().unwrap();

        let stdout = io::stdout();
        let stderr = io::stderr();

        let mut stdout = stdout.lock();
        let mut stderr = stderr.lock();

        for output in &self.output_streams {
            let _stdout = stdout.write_all(&output.stdout);
            let _stderr = stderr.write_all(&output.stderr);
        }
    }
}

// /// Execute the given command, writing the output to [`stdout`] and [`stderr`]
// ///
// /// [`stdout`]: io::stdout
// /// [`stderr`]: io::stderr
// pub(crate) fn execute_command(mut cmd: Command, out_perm: &Mutex<()>) -> ExitCode {
//     let output = cmd.output();
//
//     // Wait for the command to exit
//     match output {
//         Ok(output) => {
//             // While this lock is active, this thread will be the only thread allowed
//             // to write its outputs.
//             let _lock = out_perm.lock().expect("failed to lock `out_perm`");
//
//             let stdout = io::stdout();
//             let stderr = io::stderr();
//
//             let _drop = stdout.lock().write_all(&output.stdout);
//             let _drop = stderr.lock().write_all(&output.stderr);
//
//             if output.status.code() == Some(0_i32) {
//                 ExitCode::Success
//             } else {
//                 ExitCode::GeneralError
//             }
//         },
//         Err(ref why) if why.kind() == io::ErrorKind::NotFound => {
//             wutag_error!("command not found: {:?}", cmd);
//             ExitCode::GeneralError
//         },
//         Err(why) => {
//             wutag_error!("problem while executing command: {}", why);
//             ExitCode::GeneralError
//         },
//     }
// }

/// Execute the given command, writing the output to [`stdout`] and [`stderr`]
pub(crate) fn execute_commands<I: Iterator<Item = Result<Command>>>(
    cmds: I,
    out_perm: &Mutex<()>,
    output_buffering: bool,
) -> ExitCode {
    let mut output_buffer = OutputBuffer::new(out_perm);

    for res in cmds {
        let mut cmd = match res {
            Ok(cmd) => cmd,
            Err(err) => return handle_cmd_error(None, err),
        };

        let output = if output_buffering {
            cmd.output()
        } else {
            // Don't buffer the output on a single thread
            // Allows for viewing and interacting with intermediate command output
            cmd.spawn().and_then(|c| c.wait_with_output())
        };

        // If spawned, wait for the command to exit
        match output {
            Ok(output) => {
                if output_buffering {
                    output_buffer.push(output.stdout, output.stderr);
                }
                if output.status.code() != Some(0) {
                    output_buffer.write();
                    return ExitCode::GeneralError;
                }
            },
            Err(why) => {
                output_buffer.write();
                return handle_cmd_error(Some(&cmd), why.into());
            },
        }
    }

    output_buffer.write();
    ExitCode::Success
}

/// Print an error to `stderr`
pub(crate) fn handle_cmd_error(cmd: Option<&Command>, err: anyhow::Error) -> ExitCode {
    match err.downcast::<io::Error>() {
        Ok(err) => match (cmd, err) {
            (Some(cmd), err) if err.kind() == io::ErrorKind::NotFound => {
                wutag_error!("command not found: {:?}", cmd);
            },
            (_, err) => {
                wutag_error!("problem while executing command: {}", err);
            },
        },
        Err(e) => {
            wutag_error!("failed to downcast anyhow error");
        },
    }

    ExitCode::GeneralError
}
