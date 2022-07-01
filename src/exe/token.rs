use std::fmt::{self, Display, Formatter};

/// Designates what should be written to a buffer
///
/// Each `Token` contains either text, or a placeholder variant, which will be
/// used to generate commands after all tokens for a given command template have
/// been collected.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    Placeholder,
    Basename,
    Parent,
    NoExt,
    BasenameNoExt,
    Wutag,
    WutagColored,
    WutagSet,
    WutagRemove,
    WutagClear,
    WutagCp,
    Text(String),
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Self::Placeholder => "{}",
            Self::Basename => "{/}",
            Self::Parent => "{//}",
            Self::NoExt => "{.}",
            Self::BasenameNoExt => "{/.}",
            Self::Wutag => "{..}",
            Self::WutagColored => "{@}",
            Self::WutagSet => "{@s}",
            Self::WutagRemove => "{@r}",
            Self::WutagClear => "{@x}",
            Self::WutagCp => "{@c}",
            Self::Text(ref s) => s,
        })
    }
}
