use std::{io, io::Write, process::Command, sync::Mutex};

use super::exits::ExitCode;
use crate::wutag_error;
use colored::Colorize;

pub(crate) fn execute_command(mut cmd: Command, out_perm: &Mutex<()>) -> ExitCode {
    // Spawn the supplied command.
    let output = cmd.output();

    // Then wait for the command to exit, if it was spawned.
    match output {
        Ok(output) => {
            // While this lock is active, this thread will be the only thread allowed
            // to write its outputs.
            let _lock = out_perm.lock().unwrap();

            let stdout = io::stdout();
            let stderr = io::stderr();

            let _ = stdout.lock().write_all(&output.stdout);
            let _ = stderr.lock().write_all(&output.stderr);

            if output.status.code() == Some(0) {
                ExitCode::Success
            } else {
                ExitCode::GeneralError
            }
        },
        Err(ref why) if why.kind() == io::ErrorKind::NotFound => {
            wutag_error!("Command not found: {:?}", cmd);
            ExitCode::GeneralError
        },
        Err(why) => {
            wutag_error!("Problem while executing command: {}", why);
            ExitCode::GeneralError
        },
    }
}
