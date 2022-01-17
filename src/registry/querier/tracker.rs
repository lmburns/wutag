//! Trait and struct used to track the position of the parser

use super::{QueryRange, Span};
use itertools::Itertools;
use std::ops::Range;

/// Provide the ability to track the position of the parser in the query when
/// performing the tokenization
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Position<T> {
    /// Range of the characters
    pub(crate) range: QueryRange,
    /// Value being tracked
    pub(crate) value: T,
}

impl<T> Position<T> {
    /// Convert into the value `&T`
    pub(crate) const fn into(&self) -> &T {
        &self.value
    }
}

/// Converting a [`Span`] to a [`Range`] of some sort.
///
/// A trait must be implemented to use these values, because [`LocatedSpan`] is
/// defined within another crate
pub(crate) trait ToRange {
    /// Return a [`Range`] of the entire [`Span`]
    fn to_range(&self) -> Range<usize>;

    /// Return a `Range` from the offset to the closing parenthesis, brace, or
    /// bracket of the `Span`. (`|` | `)` | `]` | `}` | `>`)
    ///
    /// `Span`:  LocatedSpan<&'a str, &'a Query>
    /// `Range`: std::ops::Range
    fn to_sync_point(&self) -> Range<usize>;

    /// Return a `Range` from the offset to the next whitespace in the
    /// `Span`. (` ` | `\t` | `\n`)
    fn to_whitespace(&self) -> Range<usize>;
}

/// Macro to prevent writing basically the same function twice
macro_rules! to_point {
    ($name:ident, $matches:tt) => {
        fn $name(&self) -> Range<usize> {
            let frag = self.fragment();
            let end = frag
                .chars()
                .find_position($matches)
                .map_or(frag.len(), |pair| pair.0);

            Range {
                start: self.location_offset(),
                end:   self.location_offset() + end,
            }
        }
    };
}

impl ToRange for Span<'_> {
    to_point!(
        to_sync_point,
        (|ch| matches!(ch, '|' | ')' | ']' | '}' | '>'))
    );

    to_point!(to_whitespace, (|ch| matches!(ch, ' ' | '\t' | '\n')));

    fn to_range(&self) -> Range<usize> {
        let start = self.location_offset();
        let end = start + self.fragment().len();
        start..end
    }
}
