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
            Token::Placeholder => "{}",
            Token::Basename => "{/}",
            Token::Parent => "{//}",
            Token::NoExt => "{.}",
            Token::BasenameNoExt => "{/.}",
            Token::Wutag => "{..}",
            Token::WutagColored => "{@}",
            Token::WutagSet => "{@s}",
            Token::WutagRemove => "{@r}",
            Token::WutagClear => "{@x}",
            Token::WutagCp => "{@c}",
            Token::Text(ref s) => s,
        })
    }
}
