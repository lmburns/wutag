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
        match *self {
            Token::Placeholder => f.write_str("{}")?,
            Token::Basename => f.write_str("{/}")?,
            Token::Parent => f.write_str("{//}")?,
            Token::NoExt => f.write_str("{.}")?,
            Token::BasenameNoExt => f.write_str("{/.}")?,
            Token::Wutag => f.write_str("{..}")?,
            Token::WutagColored => f.write_str("{@}")?,
            Token::WutagSet => f.write_str("{@s}")?,
            Token::WutagRemove => f.write_str("{@r}")?,
            Token::WutagClear => f.write_str("{@x}")?,
            Token::WutagCp => f.write_str("{@c}")?,
            Token::Text(ref string) => f.write_str(string)?,
        }
        Ok(())
    }
}
