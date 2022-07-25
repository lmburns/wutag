//! Modules that allow for `xattr` manipulation of tags

pub(crate) mod core;
pub(crate) mod tag;
pub(crate) mod tag_old;
pub(crate) mod value;

use colored::{ColoredString, Colorize};
use std::{ffi, io, string};
use thiserror::Error;

/// Default error used throughout this `xattr` module
#[non_exhaustive]
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// Tag already exists within the registry
    #[error("Tag({0}) already exists")]
    TagExists(ColoredString),

    /// Tag is not found within the registry
    #[error("Tag({0}) doesn't exist")]
    TagNotFound(String),

    /// The key was invalid
    #[error("tag key was invalid - {0}")]
    InvalidTagKey(String),

    /// Value already exists within the registry
    #[error("Value({0}) already exists on Tag({1})")]
    ValueExists(ColoredString, ColoredString),

    /// Value is not found on a tag within the registry
    #[error("Value({0}) doesn't exist on Tag({1})")]
    TagValueNotFound(ColoredString, ColoredString),

    /// [`Value`] doesn't exist on a [`Tag`]
    #[error("value doesn't exist on Tag({1})")]
    ValueNotFoundOnTag(String, String),

    /// The value was invalid
    #[error("value was invalid - {0}")]
    InvalidValueVal(String),

    /// Value is not found within the registry
    #[error("Value({0}) doesn't exist")]
    ValueNotFound(ColoredString),

    /// General error
    #[error("error: {0}")]
    Other(String),

    /// OS Error 95
    #[error("setting xattrs on symlinks is unavailable: {0}")]
    SymlinkUnavailable95(String),

    /// OS Error 1. Two strings are used only for bolding the word `privileged`
    #[error(
        r#"to set an extended attribute on a symlink,
the 'trusted' namespace must be used instead of 'user'. Also, a {1} user must perform
this action: {0}.

For regular files the permission bits definite access to the file's contents, whereas for special
files they define access to the device described by the special file. File permissions of symbolic
links are not used in access checks.

For this reason, user extended attributes are allowed only for regular files
and directories, and access to user extended attributes is restricted to
the owner and to users with appropriate capabilities for directories with the
sticky bit set. See xattr(7), attr(1), setfattr(1), lsetxattr(2)"#
    )]
    SymlinkUnavailable1(String, String),

    /// Invalid string was given
    #[error("provided string was invalid - {0}")]
    InvalidString(#[from] ffi::NulError),

    /// Unable to convert into valid UTF-8
    #[error("provided string was not valid UTF-8")]
    Utf8ConversionFailed(#[from] string::FromUtf8Error),

    /// Unable to convert into valid UTF-8 (`str`)
    #[error("provided string was not valid UTF-8")]
    Utf8ConversionFailedStr(#[from] std::str::Utf8Error),

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
pub(crate) type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    #[inline]
    fn from(err: io::Error) -> Self {
        match err.kind() {
            // FIX: This displays 'Tag(File exists (os error 17)) already exists'
            io::ErrorKind::AlreadyExists => Self::TagExists(err.to_string().green().bold()),
            _ => match err.raw_os_error() {
                Some(61_i32) => Self::TagNotFound("".to_owned()),
                _ => Self::Other(err.to_string()),
            },
        }
    }
}
