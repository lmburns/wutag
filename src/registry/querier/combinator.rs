//! Extra functions (combinators) to help with parsing
//!
//! Taken from `rcoh/angle-grinder`

use super::{
    ast::query::Query,
    tracker::{Position, ToRange},
    QueryRange, Span,
};
use nom::{
    error::{Error as NomError, ParseError},
    Err as NomErr, IResult, Parser, Slice as NomSlice,
};
use nom_locate::position;
use std::ops::Range;

/// Combinator that captures the position of the current `fragment` of the
/// [`Span`], and returns a `Result` of <[`Span`], [`Position`]>
pub(super) fn with_position<'a, O, E, F>(
    mut f: F,
) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, Position<O>, E>
where
    E: ParseError<Span<'a>>,
    F: Parser<Span<'a>, O, E>,
{
    move |input: Span<'a>| {
        let (input, start) = position(input)?;
        match f.parse(input) {
            Ok((i, o)) => {
                let (i, end) = position(i)?;
                Ok((i, Position {
                    range: Range {
                        start: start.location_offset(),
                        end:   end.location_offset(),
                    },
                    value: o,
                }))
            },
            Err(e) => Err(e),
        }
    }
}

/// Combinator that expects the passed parser to succeed
///
/// # Error
///   1. The error message is printed to the console
///   2. Input is consumed to the `sync_point` (i.e., `|` | `)` | `}` | `]`)
///   3. `None` is returned
pub(super) fn expect<'a, F, E, T>(
    mut parser: F,
    error_msg: E,
) -> impl FnMut(Span<'a>) -> IResult<Span, Option<T>>
where
    F: FnMut(Span<'a>) -> IResult<Span, T>,
    E: AsRef<str>,
{
    move |input: Span<'a>| match parser(input) {
        Ok((rest, out)) => Ok((rest, Some(out))),
        Err(NomErr::Error(NomError { input, .. }) | NomErr::Failure(NomError { input, .. })) => {
            let r = input.to_sync_point();
            let end = r.end - input.location_offset();
            input
                .extra
                .log_error(error_msg.as_ref())
                .add_range(r, "-")
                .print_err();
            Ok((input.slice(end..), None))
        },
        Err(err) => Err(err),
    }
}

/// Combinator that expects the passed parser to succeed
///
/// # Error
///   1. The error function is called on input [`Range`]
///   2. Input is consumed to the `sync_point` (i.e., `|` | `)` | `}` | `]`)
///   3. `None` is returned
pub(super) fn expect_fn<'a, F, O, EF>(
    mut parser: F,
    mut error_fn: EF,
) -> impl FnMut(Span<'a>) -> IResult<Span, Option<O>>
where
    F: Parser<Span<'a>, O, NomError<Span<'a>>>,
    EF: FnMut(&Query, QueryRange),
{
    move |input: Span<'a>| match parser.parse(input) {
        Ok((remaining, out)) => Ok((remaining, Some(out))),
        Err(NomErr::Error(NomError { input, .. }) | NomErr::Failure(NomError { input, .. })) => {
            let r = input.to_sync_point();
            let end = r.end - input.location_offset();
            error_fn(input.extra, r);
            let next = input.slice(end..);
            Ok((next, None))
        },
        Err(err) => Err(err),
    }
}

/// Combinator that acts the same as [`delimited`](nom::sequence::delimited).
/// The only difference is that an error function is called when the given
/// parser fails
///
/// # Error
///   1. The error function is called on input [`Range`]
pub(super) fn expect_delimited<'a, O1, O2, O3, F, G, H, EF>(
    mut first: F,
    mut second: G,
    mut third: H,
    mut error_fn: EF,
) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, O2, NomError<Span<'a>>>
where
    F: Parser<Span<'a>, O1, NomError<Span<'a>>>,
    G: Parser<Span<'a>, O2, NomError<Span<'a>>>,
    H: Parser<Span<'a>, O3, NomError<Span<'a>>>,
    EF: FnMut(&Query, QueryRange),
{
    move |input: Span<'a>| {
        let full_r = input.to_sync_point();
        let (input, _) = first.parse(input)?;
        let (input, o2) = second.parse(input)?;
        if let Ok((input, _)) = third.parse(input) {
            Ok((input, o2))
        } else {
            let start = input.location_offset();
            let mut remaining = input;

            loop {
                if remaining.is_empty() {
                    // Run the error function on the unparsed
                    error_fn(remaining.extra, Range {
                        start: full_r.start,
                        end:   remaining.location_offset(),
                    });
                    return Ok((remaining, o2));
                }

                remaining = remaining.slice(1..);
                let end = remaining.location_offset();
                let res = third.parse(remaining);

                if let Ok((remaining, _)) = res {
                    remaining
                        .extra
                        .log_error("unhandled input")
                        .add_range(Range { start, end }, "Possibly used an invalid character")
                        .add_solution("The following chars need to be escaped: ")
                        .add_solution("\'\"\\(){}=<>!&|?:,")
                        .add_solution("The forward slash (/) needs to be escaped if query starts with it")
                        .print_err();
                    return Ok((remaining, o2));
                }
            }
        }
    }
}
