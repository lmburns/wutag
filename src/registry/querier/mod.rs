//! Parser to the query into the database
//!
//! This module is really overkill for what `wutag` actually is. The query
//! module was written for exploration purposes. I wanted to learn more about
//! how parsers work.

pub(crate) mod ast;
pub(crate) mod combinator;
pub(crate) mod parser;
#[cfg(test)]
pub(crate) mod parser_tests;
pub(crate) mod tracker;

pub(crate) use ast::Query;

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
#[rustfmt::skip]
pub(crate) static COMPARISON_OPS: &[&str] = &[
    "and", "or", "not", "eq", "ne", "lt", "gt", "le", "ge",
    "&&",  "||", "!",   "==", "!=", "<",  ">",  "<=", ">=",
];

/// Reserved conditional operator names
pub(crate) static CONDITIONAL_RES: &[&str] = &["if", "unless"];

/// Other reserved symbols that are [`Regex`](regex::Regex)es
#[rustfmt::skip]
pub(crate) static OTHER_RES: &[&str] = &[
        "^[@\\$]F(\\[\\s*((\\.{2}|((\\d+\\s*)?\\.{2}\\s*=?\\s*)?\\d+|\\d+\\s*\\.{2})|(\\d+(\\s*,\\s*\\d+)*))\\s*\\])?$",
        r"^%r([^<{(\[])((\\\1|[^\\1]+)*[^\\1]*)(\1)(-?[iu]|[IlUmxr])*$",
        r"^%g([^<{(\[])((\\\1|[^\\1]+)*[^\\1]*)(\1)(-?[iu]|[IlUmxg])*$",
        r"^%r(<((\\<|[^<]+)*[^<])>|\{((\\\{|[^{]+)*[^{])}|\(((\\\(|[^(]+)*[^(])\)|\[((\\\[|[^\[]+)*[^\[])\])(-?[iu]|[IlUmxr])*$",
        r"^%g(<((\\<|[^<]+)*[^<])>|\{((\\\{|[^{]+)*[^{])}|\(((\\\(|[^(]+)*[^(])\)|\[((\\\[|[^\[]+)*[^\[])\])(-?[iu]|[IlUmxg])*$",
];

/// All reserved words within [`wutag`](crate)
pub(crate) static RESERVED_WORDS: Lazy<Vec<&'static str>> =
    Lazy::new(|| [FUNC_NAMES, COMPARISON_OPS, CONDITIONAL_RES].concat());

/// Type alias for the [`Range<I>`] over the concerned section of the
/// [`Query`](ast::query::Query)
pub(self) type QueryRange = Range<usize>;

/// Type alias for [`nom_supreme`]'s [`LocatedSpan`] with extra information
/// Allows for better tracking of a query and tracking its location
pub(self) type Span<'a> = LocatedSpan<&'a str, &'a ast::query::Query>;

/// Tests used to determine if the name of the [`Tag`] or [`Value`] will be
/// allowed due to the parsing rules implemented by [`nom`]
///
/// [`Tag`]: super::super::types::tag::Tag
/// [`Value`]: super::super::types::value::Value
mod tests {
    use super::{FUNC_NAMES, OTHER_RES, RESERVED_WORDS};
    use crate::regex;
    use regex::Regex;

    #[test]
    fn match_func_names() {
        let names = FUNC_NAMES.iter().map(|f| format!("{}()", f)).collect::<Vec<_>>();

        let mut names_w_args = FUNC_NAMES
            .iter()
            .map(|f| format!("{}(hello)", f))
            .collect::<Vec<_>>();

        names_w_args.extend_from_slice(&names);

        let reg = regex!(&format!("({})\\([^(]*\\)", FUNC_NAMES.join("|")));

        for name in &names_w_args {
            assert!(reg.is_match(name));
        }
    }

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
            dollar.push(v.replace('@', "$"));
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
            dollar.push(v.replace('@', "$"));
        }

        dollar.extend_from_slice(&names[..]);

        let reg = regex!(OTHER_RES[0]);

        for totest in &dollar {
            assert!(!reg.is_match(totest));
        }
    }

    #[test]
    fn match_legal_pattern() {
        let names = &[
            "%r/hisir/",
            "%g/hisir/",
            "%g/h\\/isir/",
            "%g/hisir/",
            "%g|hi\\|sir|",
            "%r|hisir|",
            "%r:hisir:",
            "%r:hi\\:sir:",
            "%r:hi\\:s\\:i\\:r:",
            "%r;hisir;",
            "%r;hi\\;sir;",
            "%r/hi\\/sir/",
            "%g/hi\\/saa\\/ir/",
            // Flags
            "%g|hi\\|sir|-i",
            "%r|hisir|mxU",
            "%r:hisir:r-u",
            "%r:hi\\:sir:Url",
            "%r:hi\\:s\\:i\\:r:-i-uU",
        ];

        let reg1 = fancy_regex::Regex::new(OTHER_RES[1]).expect("failed to build regex");
        let reg2 = fancy_regex::Regex::new(OTHER_RES[2]).expect("failed to build regex");

        for totest in names.iter() {
            assert!(
                reg1.is_match(totest).expect("failed to unwrap match")
                    | reg2.is_match(totest).expect("failed to unwrap match")
            );
        }
    }

    #[test]
    fn match_legal_pattern_diff_delim() {
        let names = &[
            "%r<hisir>",
            "%r<h\\<isir>",
            "%r{hisir}",
            "%r{hi\\{s\\{ir}",
            "%r(hisir)",
            "%r(h\\(isir)",
            "%r[hisir]",
            "%r[h\\[isir]",
            "%g<hisir>",
            "%g{hisir}",
            "%g{h\\{isir}",
            "%g(hisir)",
            "%g[hisir]",
            // Flags
            "%r<hisir>i",
            "%r<h\\<isir>r-i",
            "%r{hisir}Ux",
            "%r{hi\\{s\\{ir}m",
            "%r(hisir)-u",
            "%r[h\\[isir]m",
        ];

        let reg1 = regex!(OTHER_RES[3]);
        let reg2 = regex!(OTHER_RES[4]);

        for totest in names.iter() {
            assert!(reg1.is_match(totest) | reg2.is_match(totest));
        }
    }
}
