use std::{io, io::Write, process::Command, sync::Mutex};

use super::exits::ExitCode;
use crate::util::fmt_err;

pub fn execute_command(mut cmd: Command, out_perm: &Mutex<()>) -> ExitCode {
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
            eprintln!("{}", fmt_err(format!("Command not found: {:?}", cmd)));
            ExitCode::GeneralError
        },
        Err(why) => {
            eprintln!(
                "{}",
                fmt_err(format!("Problem while executing command: {}", why))
            );
            ExitCode::GeneralError
        },
    }
}
