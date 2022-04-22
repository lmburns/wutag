//! Modules that allow for `xattr` manipulation of tags

pub mod color;
pub mod tag;
pub mod xattr;

use colored::{ColoredString, Colorize};
use std::{ffi, io, string};
use thiserror::Error;

/// Prefix used to identify extra attributes on files that were added by `wutag`
pub const WUTAG_NAMESPACE: &str = "user.wutag";

/// Default error used throughout this `wutag_core`
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum Error {
    /// Tag already exists within the database
    #[error("tag {0} already exists")]
    TagExists(ColoredString),

    /// Tag is not found within the database
    #[error("tag `{0}` doesn't exist")]
    TagNotFound(String),

    /// The key was invalid
    #[error("tag key was invalid - {0}")]
    InvalidTagKey(String),

    /// General error
    #[error("error: {0}")]
    Other(String),

    /// Invalid string was given
    #[error("provided string was invalid - {0}")]
    InvalidString(#[from] ffi::NulError),

    /// Unable to convert into valid UTF-8
    #[error("provided string was not valid UTF-8")]
    Utf8ConversionFailed(#[from] string::FromUtf8Error),

    /// Extended attributes were modified when retrieving them
    #[error("xattrs changed while getting their size")]
    AttrsChanged,

    /// Invalid color was given
    #[error(
        "provided color `{0}` is not a valid hex color. Standard 'red', 'green', etc. are also \
         acceptable. See '--help' for the full list"
    )]
    InvalidColor(String),

    /// Unable to use `serde` on the `Tag`
    #[error("failed to serialize or deserialize tag - `{0}`")]
    TagSerDeError(#[from] serde_cbor::Error),

    /// Unable to convert to or from `yaml`
    #[error("failed to serialize or deserialize yaml - `{0}`")]
    YamlSerDeError(#[from] serde_yaml::Error),

    /// Unable to decode with `base64`
    #[error("failed to decode data with base64 - `{0}`")]
    Base64DecodeError(#[from] base64::DecodeError),
}

/// Shorter `Result`, used for ergonomics
pub type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    #[inline]
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::AlreadyExists => Self::TagExists(err.to_string().green().bold()),
            _ => match err.raw_os_error() {
                Some(61_i32) => Self::TagNotFound("".to_owned()),
                _ => Self::Other(err.to_string()),
            },
        }
    }
}
