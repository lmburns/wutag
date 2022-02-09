//! Pattern or exact searches into the database

use crate::util::contains_upperchar;
use anyhow::{anyhow, Result};
use colored::Colorize;
use regex::bytes::Regex;

/// Regular expression flags that can be used within the expression, rather than
/// using command line flags
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum SearchFlags {
    /// Case insensitively search
    CaseInsensitive,
    /// Case sensitively search
    CaseSensitive,
    /// Multi-line mod: `^` and `$` match beginning/end of line
    Multiline,
    /// Swap the meaning of `x*` and `x*?`
    SwapGreed,
    /// Unicode support
    Unicode,
    /// Disable unicode support
    UnicodeDisable,
    /// Ignore whitespace, and allow for comments (for whatever reason)
    IgnoreWhitespace,

    /// Regular expression mode
    Regex,
    /// Glob mode
    Glob,
    /// Unknown flag
    Unknown,
}

impl SearchFlags {
    /// Create a vector of `SearchFlags` from a vector of `char`s
    pub(crate) fn from_vec(v: &[String]) -> Vec<Self> {
        v.iter()
            .map(|flag| match flag.as_str() {
                "i" => Self::CaseInsensitive,
                "-i" | "I" => Self::CaseSensitive,
                "l" => Self::SwapGreed,
                "u" => Self::Unicode,
                "U" | "-u" => Self::UnicodeDisable,
                "m" => Self::Multiline,
                "x" => Self::IgnoreWhitespace,
                "r" => Self::Regex,
                "g" => Self::Glob,
                _ => Self::Unknown,
            })
            .collect::<Vec<_>>()
    }

    /// Display a longer and colorized error message
    pub(crate) fn error_message() -> String {
        macro_rules! g {
            ($s:tt) => {
                $s.green().bold()
            };
        };

        format!(
            r#"Add a flag(s):
- {}: regex
- {}: glob
- {} | {}: case insensitive
- {}: case sensitive
- {}: swap the meaning of {} and {}
- {}: unicode support (default)
- {} | {}: disable unicode support
- {}: multiline
- {}: ignore whitespace"#,
            g!("r"),
            g!("g"),
            g!("i"),
            g!("I"),
            g!("-i"),
            g!("l"),
            "x*".yellow(),
            "x*?".yellow(),
            g!("u"),
            g!("U"),
            g!("-u"),
            g!("m"),
            g!("x"),
        )
    }
}

/// The type of search into the database
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum SearchKind {
    /// An exact search. No pattern matching
    Exact,
    /// A glob search
    Glob,
    /// A regular expression search
    Regex,
}

/// A search into the database
#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct Search {
    /// The raw string or a string compiled as a [`Regex`]
    inner: String,
    /// The type of search
    t:     SearchKind,
}

impl Search {
    /// Return the inner `String`
    pub(crate) const fn inner(&self) -> &String {
        &self.inner
    }

    /// Return the inner [`Search`] type
    pub(crate) const fn inner_t(&self) -> SearchKind {
        self.t
    }

    /// Create a new [`Search`] for exact keyword(s)
    pub(crate) fn new_exact<S: AsRef<str>>(pattern: S, esc: bool) -> Self {
        let s = pattern.as_ref();
        Self {
            inner: esc
                .then(|| regex::escape(&s.replace("\\\"", "\"")).as_str().to_owned())
                .unwrap_or_else(|| s.to_owned()),
            t:     SearchKind::Exact,
        }
    }

    /// Create a new [`Search`] with a glob.
    ///
    /// This converts the glob pattern into a regex. This will allow the glob to
    /// be used with the [`Regex`] parser later on
    ///
    /// [`wax`] provides the following patterns:
    ///   - `(?i)`    = Turns on case insensitivity
    ///   - `(?-i)`   = Turns off case insensitivity
    ///   - `?`       = Matches 0 or more of any character except `/` (shortest)
    ///   - `*`       = Matches 0 or more of any character except `/` (longest)
    ///   - `**`      = Matches 0 or more of any character recursively
    ///   - `[a-z]`   = Matches `a` through `z` character class
    ///   - `[!a-z]`  = Matches a negated character class
    ///   - `{a,b}`   = Matches `a` or `b`
    ///   - `<a*:0,>` = Matches a repitition of `a` + any character 0 or more
    ///     times
    ///   - `<a*:1,2>` = Matches a repitition of `a` + any character 1 to 2
    ///     times
    pub(crate) fn new_glob<S: AsRef<str>>(pattern: S, flags: &[SearchFlags]) -> Self {
        let builder = wax::Glob::new(pattern.as_ref()).expect("failed to build glob");
        let mut new = Self::combine_flags(flags);
        new.push_str(&builder.regex().to_string());

        Self {
            inner: new,
            t:     SearchKind::Glob,
        }
    }

    /// Create a new [`Search`] with a [`Regex`]
    pub(crate) fn new_regex<S: AsRef<str>>(str: S, flags: &[SearchFlags]) -> Self {
        let mut new = Self::combine_flags(flags);
        new.push_str(str.as_ref());

        Self {
            inner: new,
            t:     SearchKind::Regex,
        }
    }

    fn combine_flags(flags: &[SearchFlags]) -> String {
        let mut s = String::new();
        for flag in flags {
            match flag {
                SearchFlags::Multiline => s.push_str("(?m)"),
                SearchFlags::CaseInsensitive => s.push_str("(?i)"),
                SearchFlags::CaseSensitive => s.push_str("(?-i)"),
                SearchFlags::Unicode => s.push_str("(?u)"),
                SearchFlags::UnicodeDisable => s.push_str("(?-u)"),
                SearchFlags::SwapGreed => s.push_str("(?U)"),
                SearchFlags::IgnoreWhitespace => s.push_str("(?x)"),
                _ => {},
            }
        }

        if contains_upperchar(&s) && !flags.contains(&SearchFlags::CaseSensitive) {
            s.push_str("(?i)");
        }

        s
    }

    /// Check whether the string to the [`Search`] is empty
    pub(crate) fn is_empty(&self) -> bool {
        self.inner().is_empty()
    }

    /// Convert this keyword to a `regex::Regex` object
    pub(crate) fn to_regex(&self) -> Result<regex::Regex> {
        regex::Regex::new(self.inner())
            .map_err(|e| anyhow!("{}\n\n{}", Self::error_message(), e.to_string()))
    }

    /// Convert this keyword to a `regex::Regex` object.
    pub(crate) fn to_regex_builder(
        &self,
        case_insensitive: bool,
        case_sensitive: bool,
    ) -> Result<Regex> {
        use regex::bytes::RegexBuilder;

        let sensitive = !case_insensitive && (case_sensitive || contains_upperchar(self.inner()));

        RegexBuilder::new(self.inner())
            .case_insensitive(!sensitive)
            .build()
            .map_err(|e| anyhow!("{}\n\n{}", Self::error_message(), e.to_string()))
    }

    // TODO:
    /// Use look-around capable regular expressions
    // #[cfg(feature = "lookaround")]
    // pub(crate) fn to_lookaround_regex(&self) -> Result<fancy_regex::Regex> {
    //     fancy_regex::Regex::new(self.inner())
    //         .map_err(|e| anyhow!("{}\n\n{}", Self::error_message(), e.to_string()))
    // }

    /// Display a longer and colorized error message
    pub(crate) fn error_message() -> String {
        String::from(
            r#"Invalid pattern.
The following patterns can be used:
Regex:
    (1) `--regex` flag
    (2) `/pattern/r`
    (3) `%r{{pattern}}` (any delimiter)

Note: For PCRE like regular expressions (lookarounds), the crate feature `lookaround` must be used

Glob:
    (1) no flag
    (2) `/pattern/g`
    (3)`%g{{pattern}}` (any delimiter)
"#,
        )
    }
}
