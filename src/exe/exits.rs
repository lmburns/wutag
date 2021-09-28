#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExitCode {
    Success,
    GeneralError,
    Sigint,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> Self {
        match code {
            ExitCode::Success => 0,
            ExitCode::GeneralError => 1,
            ExitCode::Sigint => 130,
        }
    }
}

impl ExitCode {
    fn is_error(self) -> bool {
        self != ExitCode::Success
    }
}

// TODO: Can implement Sized for this, or leave this lint disabled
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn generalize_exitcodes(results: Vec<ExitCode>) -> ExitCode {
    if results.iter().any(|&c| ExitCode::is_error(c)) {
        return ExitCode::GeneralError;
    }
    ExitCode::Success
}
