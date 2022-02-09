//! Execute the given command passed to `-x` or `-X`

use std::{io, io::Write, process::Command, sync::Mutex};

use super::exits::ExitCode;
use crate::wutag_error;
use colored::Colorize;

/// Execute the given command, writing the output to [`stdout`] and [`stderr`]
///
/// [`stdout`]: io::stdout
/// [`stderr`]: io::stderr
pub(crate) fn execute_command(mut cmd: Command, out_perm: &Mutex<()>) -> ExitCode {
    let output = cmd.output();

    // Wait for the command to exit
    match output {
        Ok(output) => {
            // While this lock is active, this thread will be the only thread allowed
            // to write its outputs.
            let _lock = out_perm.lock().expect("failed to lock `out_perm`");

            let stdout = io::stdout();
            let stderr = io::stderr();

            let _drop = stdout.lock().write_all(&output.stdout);
            let _drop = stderr.lock().write_all(&output.stderr);

            if output.status.code() == Some(0_i32) {
                ExitCode::Success
            } else {
                ExitCode::GeneralError
            }
        },
        Err(ref why) if why.kind() == io::ErrorKind::NotFound => {
            wutag_error!("command not found: {:?}", cmd);
            ExitCode::GeneralError
        },
        Err(why) => {
            wutag_error!("problem while executing command: {}", why);
            ExitCode::GeneralError
        },
    }
}
