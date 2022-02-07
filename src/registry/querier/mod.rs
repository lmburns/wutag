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
#[rustfmt::skip]
pub(crate) static FUNC_NAMES: &[&str] = &[
    // Objects
    "value", "tag", "implied", "implies",
    // File operations
    "hash", "atime", "ctime", "mtime", "before", "after",
    // Other
    "print", "exec",
];

/// Reserved comparison operator names
pub(crate) static COMPARISON_OPS: &[&str] =
    &["and", "or", "not", "eq", "ne", "lt", "gt", "le", "ge"];

/// Other reserved symbols that are [`Regex`](regex::Regex)es
#[rustfmt::skip]
pub(crate) static OTHER_RES: &[&str] =
    &["^[@\\$]F(\\[\\s*((\\.{2}|((\\d+\\s*)?\\.{2}\\s*=?\\s*)?\\d+|\\d+\\s*\\.{2})|(\\d+(\\s*,\\s*\\d+)*))\\s*\\])?$"];

/// All reserved words within `wutag`
pub(crate) static RESERVED_WORDS: Lazy<Vec<&'static str>> =
    Lazy::new(|| [FUNC_NAMES, COMPARISON_OPS].concat());

/// Type alias for the `Range<I>` over the concerned section of the `Query`
pub(crate) type QueryRange = Range<usize>;

/// Type alias for [`nom_supreme`]'s [`LocatedSpan`] with extra information
/// Allows for better tracking of a query and tracking its location
pub(crate) type Span<'a> = LocatedSpan<&'a str, &'a ast::query::Query>;

/// Tests used to determine if the name of the `Tag` or `Value` will be allowed
/// due to the parsing rules implemented by [`nom`]
mod tests {
    use super::{OTHER_RES, RESERVED_WORDS};
    use crate::regex;
    use regex::Regex;

    #[test]
    fn match_legal_tag_array() {
        let mut names = vec![
            "@F",
            "@F[1]",
            "@F[33333]",
            "@F[..]",
            "@F[11..23]",
            "@F[11..=279]",
            "@F[1..4]",
            "@F[1..=4]",
            "@F[4..]",
            "@F[44..]",
            "@F[..5]",
            "@F[..55556]",
            "@F[..=2]",
            "@F[..=374]",
            "@F[1,2]",
            "@F[1,2,3]",
            // Space
            "@F[  1 ]",
            "@F[ 33333 ]",
            "@F[    ..  ]",
            "@F[11 ..23  ]",
            "@F[  11..  =  279 ]",
            "@F[ 1  .. 4  ]",
            "@F[1 ..  =4   ]",
            "@F[  4 .. ]",
            "@F[44..   ]",
            "@F[  .. 5  ]",
            "@F[ ..  55556 ]",
            "@F[  .. = 2 ]",
            "@F[..= 374 ]",
            "@F[ 1 ,  2 ]",
            "@F[ 1, 2  ,3]",
        ]
        .iter()
        .map(|v| (*v).to_string())
        .collect::<Vec<_>>();

        let mut dollar = vec![];
        for v in &names {
            dollar.push(v.replace("@", "$"));
        }

        dollar.extend_from_slice(&names[..]);

        let reg = regex!(OTHER_RES[0]);

        for totest in &dollar {
            assert!(reg.is_match(totest));
        }
    }

    #[test]
    fn match_illegal_tag_array() {
        let names = vec![
            "@F[]",
            "@F[..=]",
            "@F[,]",
            "@F[=]",
            "@F[",
            // Space
            "@F[  ]",
            "@F[ .. = ]",
            "@F[ , ]",
            "@F[ = ]",
            "@F[",
        ]
        .iter()
        .map(|v| (*v).to_string())
        .collect::<Vec<_>>();

        let mut dollar = vec![];
        for v in &names {
            dollar.push(v.replace("@", "$"));
        }

        dollar.extend_from_slice(&names[..]);

        let reg = regex!(OTHER_RES[0]);

        for totest in &dollar {
            assert!(!reg.is_match(totest));
        }
    }
}
