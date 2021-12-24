//! Exit codes to be used for the `exe` module of this crate. Allows for sending
//! the correct exit code to the calling program when executing commands across
//! multiple threads

/// The exit code
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ExitCode {
    /// Successful exit (0)
    Success,
    /// General error exit (!0)
    GeneralError,
    /// Interrupted exit (130)
    #[allow(dead_code)]
    Sigint,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> Self {
        match code {
            ExitCode::Success => 0_i32,
            ExitCode::GeneralError => 1_i32,
            ExitCode::Sigint => 130_i32,
        }
    }
}

impl ExitCode {
    /// Is the `ExitCode` an error?
    fn is_error(self) -> bool {
        self != Self::Success
    }
}

// TODO: Can implement Sized for this, or leave this lint disabled

/// If there are any errors in the vector of `ExitCode`s, return a
/// [`GeneralError`](ExitCode::GeneralError)
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn generalize_exitcodes(results: Vec<ExitCode>) -> ExitCode {
    if results.iter().any(|&c| ExitCode::is_error(c)) {
        return ExitCode::GeneralError;
    }
    ExitCode::Success
}
