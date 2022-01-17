//! The parsed and unparsed query

use super::{
    super::parser::parse_query,
    error::{ErrDebug, ErrorBuilder, ErrorReport},
    Expr,
};
use nom::FindSubstring;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A query that has been parsed by `nom`
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParsedQuery {
    /// The parsed expression
    parsed: Expr,
    /// The raw string expression
    raw:    String,
}

/// A representation of a query into the database before being parsed
#[derive(Debug)]
pub(crate) struct Query {
    /// The inner query as a string
    inner:               String,
    /// The error reporter
    pub(crate) reporter: Box<dyn ErrDebug>,
    /// The number of errors that have occurred
    err_cnt:             AtomicUsize,
    /// The delimiter used for a regex or glob
    delim:               Option<char>,
}

impl ParsedQuery {
    /// Created a new [`ParsedQuery`]
    pub(crate) fn new<S: AsRef<str>>(parsed: Expr, raw: S) -> Self {
        Self {
            parsed,
            raw: raw.as_ref().to_owned(),
        }
    }

    /// Return the parsed `Expr`
    pub(crate) const fn parsed(&self) -> &Expr {
        &self.parsed
    }

    /// Return the raw query
    pub(crate) const fn raw(&self) -> &String {
        &self.raw
    }
}

impl Query {
    /// Create a new [`Query`]
    pub(crate) fn new<S: AsRef<str>>(query: S, reporter: Option<Box<dyn ErrDebug>>) -> Self {
        let query = query.as_ref();
        let delim = query
            .find_substring("%r")
            .or_else(|| query.find_substring("%g"))
            .and_then(|pos| query.chars().nth(pos + 2));

        Self {
            inner: query.to_owned(),
            reporter: reporter.unwrap_or_else(|| Box::new(ErrorReport {})),
            err_cnt: AtomicUsize::new(0),
            delim,
        }
    }

    /// Return the number of errors for the current [`Query`]
    pub(crate) fn error_count(&self) -> usize {
        self.err_cnt.load(Ordering::Relaxed)
    }

    /// Return the inner query
    pub(crate) const fn query(&self) -> &String {
        &self.inner
    }

    /// Return the opening delimiter
    pub(crate) const fn delim(&self) -> Option<char> {
        self.delim
    }

    /// Return the closing delimiter if one is used
    pub(crate) fn closing_delim(&self) -> Option<char> {
        self.delim.map(|delim| match delim {
            '(' => ')',
            '[' => ']',
            '{' => '}',
            '<' => '>',
            de => de,
        })
    }

    /// Log an error message. Consumes `self` and returns an [`ErrorBuilder`]
    pub(crate) fn log_error<'a>(&'a self, error: &'a str) -> ErrorBuilder<'a> {
        self.err_cnt.fetch_add(1, Ordering::Relaxed);

        ErrorBuilder::new(error, self)
    }

    /// Parse the [`Query`] by returning a [`ParsedQuery`]
    pub(crate) fn parse(&self) -> Result<ParsedQuery, ()> {
        parse_query(self)
    }
}
