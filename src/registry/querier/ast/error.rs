//! Errors used within the very small query language

use super::{super::QueryRange, query::Query};
use annotate_snippets::{
    display_list::{DisplayList, DisplayMark, FormatOptions},
    formatter::style,
    snippet::{Annotation, AnnotationType, Slice, Snippet, SourceAnnotation},
};
use std::env;

/// Build an error for a fancified display
#[derive(Debug)]
pub(crate) struct ErrorBuilder<'a> {
    /// The snippet to the error
    snippet: Snippet<'a>,
    /// The inner query that is passed around to parsing functions
    query:   &'a Query,
}

impl<'a> ErrorBuilder<'a> {
    /// Create a new [`ErrorBuilder`]
    pub(crate) fn new(err: &'a str, query: &'a Query) -> Self {
        Self {
            snippet: Snippet {
                title:  Some(Annotation {
                    label:           Some(err),
                    id:              None,
                    annotation_type: AnnotationType::Error,
                }),
                slices: vec![],
                footer: vec![],
                opt:    FormatOptions {
                    color: env::var_os("NO_COLOR").is_none() && atty::is(atty::Stream::Stderr),
                    ..FormatOptions::default()
                },
            },
            query,
        }
    }

    /// Add a range of characters to highlight
    pub(crate) fn add_range(mut self, range: QueryRange, text: &'a str) -> Self {
        self.snippet.slices.push(Slice {
            source:      self.query.query().as_str(),
            line_start:  1,
            origin:      None,
            fold:        false,
            annotations: vec![SourceAnnotation {
                range:           (range.start, range.end),
                label:           text,
                annotation_type: AnnotationType::Error,
            }],
        });
        self
    }

    /// Add a solution to print to the screen
    pub(crate) fn add_solution(mut self, solution: &'a str) -> Self {
        self.snippet.footer.push(Annotation {
            label:           Some(solution),
            id:              None,
            annotation_type: AnnotationType::Help,
        });
        self
    }

    /// Print the report to the screen
    pub(crate) fn print_err(mut self) {
        self.query.reporter.handle_error(self.snippet);
    }
}

/// An error reporter to implement a trait to print an error to the console
#[derive(Debug, Default)]
pub(crate) struct ErrorReport {}

/// A trait that allows for the fancy printing of an error message
///
/// The errors are similar to Rust's builtin compiler errors/warnings
pub(crate) trait ErrorReporter {
    fn handle_error(&self, mut s: Snippet) {}
}

/// Trait that allows for debug messages as well as error reporting
///
/// Mainly used for tests
pub(crate) trait ErrDebug: ErrorReporter + std::fmt::Debug {}

impl ErrorReporter for ErrorReport {
    fn handle_error(&self, mut s: Snippet) {
        let dl = DisplayList::from(s);
        eprintln!("{}", dl);
    }
}

impl ErrDebug for ErrorReport {}
