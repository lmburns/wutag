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

pub fn generalize_exitcodes(results: &[ExitCode]) -> ExitCode {
    if results.iter().any(|&c| ExitCode::is_error(c)) {
        return ExitCode::GeneralError;
    }
    ExitCode::Success
}
