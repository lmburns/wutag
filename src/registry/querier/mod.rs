//! Parser to the query into the database

pub(crate) mod ast;
pub(crate) mod combinator;
pub(crate) mod parser;
#[cfg(test)]
pub(crate) mod parser_tests;
pub(crate) mod tracker;

use nom_locate::LocatedSpan;
use once_cell::sync::Lazy;
use std::ops::Range;

/// Reserved function names
pub(crate) static FUNC_NAMES: &[&str] = &[
    "hash", "print", "exec", "value", "tag", "atime", "ctime", "mtime",
];

/// Reserved comparison operator names
pub(crate) static COMPARISON_OPS: &[&str] =
    &["and", "or", "not", "eq", "ne", "lt", "gt", "le", "ge"];

/// All reserved words within `wutag`
pub(crate) static RESERVED_WORDS: Lazy<Vec<&'static str>> =
    Lazy::new(|| [FUNC_NAMES, COMPARISON_OPS].concat());

/// Type alias for the `Range<I>` over the concerned section of the `Query`
pub(crate) type QueryRange = Range<usize>;

/// Type alias for [`nom_supreme`]'s [`LocatedSpan`] with extra information
/// Allows for better tracking of a query and tracking its location
pub(crate) type Span<'a> = LocatedSpan<&'a str, &'a ast::query::Query>;
