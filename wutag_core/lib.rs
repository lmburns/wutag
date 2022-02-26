//! Modules that allow for `xattr` manipulation of tags

#![deny(
    clippy::all,
    clippy::complexity,
    clippy::correctness,
    clippy::nursery,
    clippy::pedantic,
    clippy::perf,
    clippy::restriction,
    clippy::style,
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    bad_style,
    const_err,
    dead_code,
    ellipsis_inclusive_range_patterns,
    exported_private_dependencies,
    ill_formed_attribute_input,
    improper_ctypes,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    missing_debug_implementations,
    missing_docs,
    no_mangle_generic_items,
    non_shorthand_field_patterns,
    noop_method_call,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    pointer_structural_match,
    private_in_public,
    pub_use_of_private_extern_crate,
    semicolon_in_expressions_from_macros,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unaligned_references,
    unconditional_recursion,
    unreachable_pub,
    variant_size_differences,
    while_true
)]
#![allow(
    // Fill out documentation
    clippy::missing_docs_in_private_items,

    // clippy::pattern_type_mismatch,
    clippy::module_name_repetitions,
    clippy::redundant_pub_crate,
    clippy::implicit_return,
    clippy::wildcard_enum_match_arm,
    clippy::separated_literal_suffix,
    clippy::blanket_clippy_restriction_lints,
    clippy::shadow_reuse,
    clippy::shadow_same,
    clippy::shadow_unrelated,
    clippy::same_name_method,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::integer_arithmetic,
    clippy::single_char_lifetime_names,
    clippy::missing_inline_in_public_items,
    clippy::undocumented_unsafe_blocks,
    clippy::indexing_slicing,

    // clippy::as_conversions,
    // clippy::cast_possible_truncation,
    // clippy::cast_sign_loss,
    // clippy::cognitive_complexity,
    // clippy::create_dir,
    // clippy::doc_markdown,
    // clippy::else_if_without_else,
    // clippy::expect_used,
    // clippy::exit,
    // clippy::integer_division,
    // clippy::mod_module_files,
    // clippy::multiple_inherent_impl,
    // clippy::similar_names,
    // clippy::string_add,
    // clippy::string_slice,
    // clippy::struct_excessive_bools,
    // clippy::too_many_lines,
    // clippy::upper_case_acronyms,
    // clippy::unreachable,
    // clippy::unwrap_in_result
    // clippy::single_match_else,
)]
#![cfg_attr(
    any(test),
    allow(
        clippy::expect_fun_call,
        clippy::expect_used,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::unwrap_in_result,
        clippy::unwrap_used,
        clippy::wildcard_enum_match_arm,
    )
)]

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
    #[error("provided color `{0}` is not a valid hex color")]
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
