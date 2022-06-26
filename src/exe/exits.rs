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
    /// Is the [`ExitCode`] an error?
    fn is_error(self) -> bool {
        self != Self::Success
    }
}

// TODO: Can implement Sized for this, or leave this lint disabled

/// If there are any errors in the vector of `ExitCode`s, return a
/// [`GeneralError`](ExitCode::GeneralError)
pub(crate) fn generalize_exitcodes<R: IntoIterator<Item = ExitCode>>(results: R) -> ExitCode {
    results
        .into_iter()
        .any(ExitCode::is_error)
        .then(|| ExitCode::GeneralError)
        .unwrap_or(ExitCode::Success)
}

#[cfg(test)]
mod test {
    use super::{generalize_exitcodes, ExitCode};

    #[test]
    fn success() {
        assert_eq!(generalize_exitcodes(vec![ExitCode::Success]), ExitCode::Success);
        assert_eq!(
            generalize_exitcodes(vec![ExitCode::Success, ExitCode::Success]),
            ExitCode::Success
        );
    }

    #[test]
    fn success_on_empty() {
        assert_eq!(generalize_exitcodes(vec![]), ExitCode::Success);
    }

    #[test]
    fn general_errors() {
        assert_eq!(
            generalize_exitcodes(vec![ExitCode::GeneralError]),
            ExitCode::GeneralError
        );
        assert_eq!(
            generalize_exitcodes(vec![ExitCode::Success, ExitCode::GeneralError]),
            ExitCode::GeneralError
        );
    }
}
