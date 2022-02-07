//! Using `nom` to parse the query

// NOTE: A lot of this can be credited to `rcoh/angle-grinder`
// I wanted to learn how to use `nom` and used some of their functions

// TODO: Allow special marker for searching files or database separately
// TODO: Tag vs File searching
// TODO: Regex/Glob patterns can't match symbol delimiter
// TODO: Lookaround crate feature
// TODO: Allow character if escaped
// TODO: Possible allow 'abc def ghi 'to be 'abc and def and ghi'

// TODO: Set file search
// TODO: is_dir, is_socket, ... functions

use nom::{
    branch::alt,
    bytes::complete::{escaped, is_a, is_not, take, take_while},
    character::complete::{
        char, digit1, hex_digit1, line_ending, multispace0, multispace1, none_of, one_of, satisfy,
        space0,
    },
    combinator::{eof, map, map_res, not, opt, peek, recognize, value, verify},
    multi::{fold_many0, many0, many1, many_till, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult, InputIter, Parser, Slice,
};
use nom_supreme::{
    multi::collect_separated_terminated,
    tag::complete::{tag, tag_no_case},
    ParserExt,
};

use super::{
    ast::{
        query::{ParsedQuery, Query},
        search::{Search, SearchFlags},
        ComparisonOp, ConditionalKind, Expr, Idx, LogicalOp, UnaryOp,
    },
    combinator::{expect, expect_delimited, expect_fn, with_position},
    tracker::ToRange,
    Span,
};

// =========================== Base Parsing ===========================
// ====================================================================

/// Recognize a new line.
///
/// Even though this is not a file and is instead a single query into a
/// database, some shells (`zsh` at least) allow for comments within the shell.
/// This is to parse the comments starting with a hashtag `#` or an empty line
fn new_line(i: Span) -> IResult<Span, Span> {
    recognize(pair(
        opt(preceded(char('#'), many0(none_of("\r\n")))),
        one_of("\r\n"),
    ))(i)
}

/// Recognize a line continued by a backslash
fn line_continuation(i: Span) -> IResult<Span, Span> {
    recognize(tuple((char('\\'), line_ending)))(i)
}

/// Recognize whitespace consisting of:
///
/// `space` | `tab` | `\r\n` | `\n` | `# ...` | `... \`
#[inline]
fn ws(i: Span) -> IResult<Span, Span> {
    alt((recognize(one_of(" \t")), new_line, line_continuation))(i)
}

/// Recognize **0 or more** whitespace consisting of:
///
/// `space` | `tab` | `\r\n` | `\n` | `# ...` | `... \`
fn ws0(i: Span) -> IResult<Span, Span> {
    recognize(many0(alt((ws, line_ending))))(i)
}

/// Recognize **1 or more** whitespace consisting of:
///
/// `space` | `tab` | `\r\n` | `\n` | `# ...` | `... \`
fn ws1(i: Span) -> IResult<Span, Span> {
    recognize(many1(alt((ws, line_ending))))(i)
}

/// Recognize **0 or more** whitespace consisting of:
///
/// `space` | `tab` | `... \`
fn ws_no_lt(i: Span) -> IResult<Span, Span> {
    recognize(many0(alt((recognize(one_of(" \t")), line_continuation))))(i)
}

/// Recognize a word
///
/// The characters were slowly added as new functions were introduced
/// All of the symbols below after `(` would be allowed without having to be
/// escaped if they were used in a place that required whitespace separation.
/// However, a phrase like `aa&&bb` is allowed and is equivalent to `aa && bb`.
/// It is either they require an escape or spaces must be required
#[inline]
fn word(i: Span) -> IResult<Span, Span> {
    recognize(many0(
        is_not(" \t\r\n\'\\\"(){}=<>!&|?:,")
            .or(is_a(" \"\'/(){}=<>!&|?:,*").preceded_by(tag("\\"))),
    ))(i)
}

/// Characters not allowed to start an identifier
///
/// Matches: `/` | `\`
#[inline]
const fn starts_ident(c: char) -> bool {
    !matches!(c, '/' | ')')
}

/// Base identifier function
///
/// Identifier must not start with `/` or `\`
/// Consumes **all input** until a space of some kind
#[inline]
fn ident(input: Span) -> IResult<Span, String> {
    terminated(recognize(pair(satisfy(starts_ident), word)), multispace0)
        .map(transform_escaped_non_expanded)
        .parse(input)
}

/// Base identifier function for a regular expression between custom delimiters
/// Consumes **all input** until a space of some kind
///
/// See [`not_closing`] for the delimiter specification
#[inline]
fn ident_custom_delim(input: Span) -> IResult<Span, String> {
    terminated(recognize(not_closing), multispace0)
        .map(transform_escaped_non_expanded)
        .parse(input)
}

/// Base identifier function for a regular expression between forward slashes
/// (`/`)
///
/// Consumes **all input** until a space of some kind
#[inline]
fn ident_slash(input: Span) -> IResult<Span, String> {
    terminated(recognize(not_slash), multispace0)
        .map(transform_escaped_non_expanded)
        .parse(input)
}

/// Base identifier function for an implicit glob
/// The word must contain an asterisk (`*`) but not a backslash before it
///
/// Consumes **all input** until a space of some kind
#[inline]
fn ident_detect_glob(input: Span) -> IResult<Span, String> {
    terminated(
        recognize(verify(word, |w| {
            for item in w.match_indices('*').collect::<Vec<_>>() {
                if let Some(u) = w.fragment().chars().nth(item.0 - 1) {
                    if u != '\\' {
                        return true;
                    }
                }
            }

            false
        })),
        multispace0,
    )
    .map(|s: Span| (*s.fragment()).to_string())
    .parse(input)
}

/// Parse a file hash string
fn file_hash(i: Span) -> IResult<Span, Expr> {
    hex_digit1
        .map(|s: Span| Expr::Hash((*s.fragment()).to_string()))
        .parse(i)
}

/// Consumes **all input** until a matching closing delimiter
///
/// The user can use `/regex/` or `%r{regex}` as a marker for a regular
/// expression. The curly braces are an example. Any character can be used as a
/// delimiter, and the character directly following the `%r` is used.
///
/// The only characters where the starting delimiter *differs* from the closing
/// delimiter are:
///   - `{` => `}`
///   - `(` => `)`
///   - `<` => `>`
///   - `[` => `]`
#[inline]
fn not_closing(i: Span) -> IResult<Span, Span> {
    let delim = i.extra.closing_delim().unwrap_or('}');
    recognize(many0(
        is_not(&*format!("\\{}", delim.to_owned()))
            .or(is_a(&*delim.to_string()).preceded_by(tag("\\"))),
    ))(i)
}

/// Consumes **all input** until a matching closing delimiter
///
/// The closing delimiter must be a forward slash (`/`)
#[inline]
fn not_slash(i: Span) -> IResult<Span, Span> {
    recognize(many0(is_not("\\/").or(is_a("/").preceded_by(tag("\\")))))(i)
}

// ========================== Escape Sequence =========================
// ====================================================================

/// Parse characters that will be escaped
fn escaped_chars(i: Span) -> IResult<Span, Span> {
    alt((
        tag("\\"),
        tag("r"),
        tag("n"),
        tag("t"),
        tag("0"),
        tag("'"),
        tag("\""),
        take(1_usize),
    ))
    .parse(i)
}

/// Transform all illegal characters that are escaped into an unescaped version
fn transform_escaped_non_expanded(input: Span) -> String {
    let mut in_escape = false;

    input
        .fragment()
        .iter_elements()
        .fold(String::new(), |mut acc, ch| {
            let was_in_escape = in_escape;
            match ch {
                '\\' if !in_escape => {
                    in_escape = true;
                },
                '\\' if in_escape => acc.push('\\'),
                '/' if in_escape => acc.push('/'),
                '(' if in_escape => acc.push('('),
                ')' if in_escape => acc.push(')'),
                '{' if in_escape => acc.push('{'),
                '}' if in_escape => acc.push('}'),
                '<' if in_escape => acc.push('<'),
                '>' if in_escape => acc.push('>'),
                '!' if in_escape => acc.push('!'),
                '&' if in_escape => acc.push('&'),
                '|' if in_escape => acc.push('|'),
                '?' if in_escape => acc.push('?'),
                ':' if in_escape => acc.push(':'),
                ',' if in_escape => acc.push(','),
                '*' if in_escape => acc.push('*'),
                ' ' if in_escape => acc.push(' '),
                '\'' if in_escape => acc.push('\''),
                '"' if in_escape => acc.push('"'),
                unhandled_escape if in_escape => {
                    acc.push('\\');
                    acc.push(unhandled_escape);
                },
                other => acc.push(other),
            };
            if was_in_escape {
                in_escape = false;
            }

            acc
        })
}

/// Transform an expanded escaped character properly if it is within quotation
/// marks
fn transform_escaped(input: Span) -> String {
    let mut in_escape = false;

    input
        .fragment()
        .iter_elements()
        .fold(String::new(), |mut acc, ch| {
            let was_in_escape = in_escape;
            match ch {
                '\\' if !in_escape => {
                    in_escape = true;
                },
                '\\' if in_escape => acc.push('\\'),
                't' if in_escape => acc.push('\t'),
                'r' if in_escape => acc.push('\r'),
                'n' if in_escape => acc.push('\n'),
                '0' if in_escape => acc.push('\0'),
                'f' if in_escape => acc.push('\x0c'),
                'v' if in_escape => acc.push('\x0b'),
                '\'' if in_escape => acc.push('\''),
                '"' if in_escape => acc.push('"'),
                unhandled_escape if in_escape => {
                    acc.push('\\');
                    acc.push(unhandled_escape);
                },
                other => acc.push(other),
            };
            if was_in_escape {
                in_escape = false;
            }

            acc
        })
}

/// Properly parse a single or double quoted string
///
/// Returns a `String`
fn quoted_string(input: Span) -> IResult<Span, String> {
    let sq_esc = escaped(none_of("\\\'"), '\\', escaped_chars).or(tag(""));
    let single_quoted = expect_delimited(tag("'"), sq_esc, tag("'"), |q, r| {
        q.log_error("missing a terminating single quotation mark")
            .add_range(r, "")
            .add_solution("Add a final single quote (')")
            .print_err();
    });

    let dq_esc = escaped(none_of("\\\""), '\\', escaped_chars).or(tag(""));
    let double_quoted = expect_delimited(tag("\""), dq_esc, tag("\""), |q, r| {
        q.log_error("missing a terminating double quotation mark")
            .add_range(r, "")
            .add_solution("Add a final double quote (\")")
            .print_err();
    });

    alt((single_quoted, double_quoted))
        .map(transform_escaped)
        .parse(input)
}

// ============================= Operators ============================
// ====================================================================

/// Parse a comparison operator. A comparison operator requires at least two
/// operands
///
/// Matches: `==` | `!=` | `<`  | `>`  | `<=` | `>=`
/// Matches: `eq` | `ne` | `lt` | `gt` | `le` | `ge`
///
/// Returns a [`ComparisonOp`] object
#[rustfmt::skip]
fn comparison_operator(i: Span) -> IResult<Span, ComparisonOp> {
    alt((
        tag("==").or(tag_no_case("eq")).map(|_| ComparisonOp::Equal),
        tag("!=").or(tag_no_case("ne")).map(|_| ComparisonOp::NotEqual),
        tag("<=").or(tag_no_case("le")).map(|_| ComparisonOp::LessThanOrEqual),
        tag(">=").or(tag_no_case("ge")).map(|_| ComparisonOp::GreaterThanOrEqual),
        tag("<").or(tag_no_case("lt")).map(|_| ComparisonOp::LessThan),
        tag(">").or(tag_no_case("gt")).map(|_| ComparisonOp::GreaterThan),
    ))(i)
}

/// Parse an initial conditional operator
fn conditional_op(i: Span) -> IResult<Span, ConditionalKind> {
    alt((
        tag_no_case("if").map(|_| ConditionalKind::If),
        tag_no_case("unless").map(|_| ConditionalKind::Unless),
    ))(i)
}

/// Parse a unary operator and returns a [`UnaryOp`] object
///
/// Matches: `!` | `not`
fn unary_operator(i: Span) -> IResult<Span, UnaryOp> {
    preceded(tag_no_case("not"), multispace1)
        .or(tag("!").precedes(multispace0))
        .map(|_| UnaryOp::Not)
        .parse(i)
}

// ============================= Arguments ============================
// ====================================================================

/// Parse a *required* single argument to a function
/// Returns a function for a continuation of parsing
fn arg_single(desc: &'static str) -> impl Clone + Fn(Span) -> IResult<Span, Expr> {
    move |input: Span| {
        expect_delimited(
            tag("(").and(ws0),
            expect_fn(opt_expr, |q, r| {
                q.log_error("this function requires 1 argument, but 0 were supplied")
                    .add_range(r, "- supplied 0 arguments")
                    .add_solution(&*format!("the argument should be {}", desc))
                    .print_err();
            })
            .map(|e| e.unwrap_or(Expr::Error)),
            tag(")"),
            |q, r| {
                q.log_error("unterminated function call")
                    .add_range(r, "unterminated function call")
                    .add_solution("Insert a right parenthesis to terminate this call")
                    .print_err();
            },
        )
        .parse(input)
    }
}

/// Parse an *optional* single argument to a function
fn opt_arg_single(i: Span) -> IResult<Span, Expr> {
    expect_delimited(
        tag("(").and(ws0),
        opt(opt_expr.delimited_by(ws0)),
        tag(")"),
        |q, r| {
            q.log_error("unterminated function call")
                .add_range(r, "unterminated function call")
                .add_solution("Insert a right parenthesis to terminate this call")
                .print_err();
        },
    )
    .map(|e| e.unwrap_or(Expr::Empty))
    .parse(i)
}

/// Parses an argument array to be used for a function
/// Allows for an empty call where no arguments are passed
#[allow(dead_code)]
fn opt_arg_arr(i: Span) -> IResult<Span, Vec<Expr>> {
    collect_separated_terminated(
        opt_expr.terminated(space0),
        tag(",").terminated(space0),
        tag(")"),
    )
    .or(tag(")").value(vec![]))
    .preceded_by(tag("(").terminated(space0))
    .parse(i)
}

/// Parses an argument array to be used for a function
#[allow(dead_code)]
fn req_arg_arr(i: Span) -> IResult<Span, Vec<Expr>> {
    expect_delimited(
        tag("(").and(multispace0),
        separated_list0(tag(",").and(ws0), opt_expr.delimited_by(ws0)),
        tag(")"),
        |q, r| {
            q.log_error("unterminated function call")
                .add_range(r, "unterminated function call")
                .add_solution("Insert a closing parenthesis `)` to call the function")
                .print_err();
        },
    )
    .parse(i)
}

/// Parse a hash string to the `hash` function. Another function is required to
/// parse the arguments so that it is not placed within the `atomic` function to
/// parse all base types. If this were to be placed there then many things that
/// were not intended to be used as hashes would be wrongly parse
fn arg_hash(i: Span) -> IResult<Span, Expr> {
    expect_delimited(
        tag("(").and(ws0),
        expect_fn(file_hash, |q, r| {
            q.log_error("this function requires 1 argument, but 0 were supplied")
                .add_range(r, "- supplied 0 arguments")
                .add_solution("the argument should be a file hash")
                .print_err();
        })
        .map(|e| e.unwrap_or(Expr::Error)),
        tag(")"),
        |q, r| {
            q.log_error("unterminated function call")
                .add_range(r, "unterminated function call")
                .add_solution("Insert a right parenthesis to terminate this call")
                .print_err();
        },
    )
    .parse(i)
}

// ============================= Functions ============================
// ====================================================================

/// Creates the `value` function to search for tag values
fn func_value(i: Span) -> IResult<Span, Expr> {
    tag("value")
        .or(tag("v"))
        .terminated(ws0)
        .precedes(arg_single("the value parameter to search for"))
        .map(Expr::value_func)
        .parse(i)
}

/// Creates the `tag` function to search for tag names
fn func_tag(i: Span) -> IResult<Span, Expr> {
    tag("tag")
        .or(tag("t"))
        .terminated(ws0)
        .precedes(arg_single("the tag name to search for"))
        .map(Expr::tag_func)
        .parse(i)
}

/// Creates the `hash` function to search for file hashes
fn func_hash(i: Span) -> IResult<Span, Expr> {
    tag("hash")
        .or(tag("h"))
        .terminated(ws0)
        .precedes(arg_hash)
        .map(Expr::hash_func)
        .parse(i)
}

// =========================== Conditional ============================
// ====================================================================

/// Parse a conditional `if` or `unless` expression
fn conditional_expr(i: Span) -> IResult<Span, Expr> {
    tuple((
        conditional_op,
        opt_expr.delimited_by(ws0),
        delimited(tag("{"), opt_expr.delimited_by(ws0), tag("}")),
        opt(preceded(
            tag_no_case("else").delimited_by(ws0),
            delimited(tag("{"), opt_expr.delimited_by(ws0), tag("}")),
        )),
    ))
    .map(|(kind, cond, tr, fa)| {
        if let Some(fals) = fa {
            Expr::conditional(kind, cond, tr, fals)
        } else {
            Expr::conditional(kind, cond, tr, Expr::Empty)
        }
    })
    .parse(i)
}

// ============================= Tag Array ============================
// ====================================================================

/// Parse a digit (`i64`)
fn parse_digit(i: Span) -> IResult<Span, i64> {
    recognize(opt(tag("-")).precedes(digit1))
        .map_res(|s: Span| s.fragment().parse::<i64>())
        .parse(i)
}

// Parse a multiple digit index `[...,...]`
//
//  - `[1,2]` | `[1,2,4]`
fn parse_index_multiple(i: Span) -> IResult<Span, Idx<i64>> {
    delimited(
        ws0,
        separated_list1(tag(",").terminated(space0), parse_digit),
        ws0,
    )
    .map(Idx::new_index)
    .parse(i)
}

/// Parse an indexed range `[...]`
///
///  - `[1..3]` | `[1..]` | `[..3]` | `[..]` | `[1..=2]` | `[..=2]`
fn parse_index_range(i: Span) -> IResult<Span, Idx<i64>> {
    tuple((
        opt(parse_digit),
        opt(tag("..").delimited_by(multispace0)),
        opt(pair(opt(tag("=").delimited_by(space0)), parse_digit)),
    ))
    .delimited_by(ws0)
    .map(|(start, sep, stop)| {
        match (start, sep, stop) {
            // (Some(start), None, None) => Idx::new_range(start, start),
            (Some(start), Some(sep), Some(end)) => end
                .0
                .is_some()
                .then(|| Idx::new_range_inclusive(start, end.1))
                .unwrap_or_else(|| Idx::new_range(start, end.1)),
            (Some(start), Some(sep), None) => Idx::new_range(start, i64::MAX),
            (None, Some(sep), Some(end)) => end
                .0
                .is_some()
                .then(|| Idx::new_range_inclusive(0, end.1))
                .unwrap_or_else(|| Idx::new_range(0, end.1)),
            (None, Some(sep), None) => Idx::new_range(0, i64::MAX),
            (None, None, None) => {
                i.extra
                    .log_error("missing an index")
                    .add_solution("Remove the brackets (`[]`)")
                    .add_solution("Add a single integer or range as an index")
                    .add_solution("Can be:\n  * [N]\n  * [N..]\n  * [..M]\n  * [N..M]")
                    .print_err();

                // This is never reached
                Idx::new_range(0, i64::MAX)
            },
            _ => unreachable!(),
        }
    })
    .parse(i)
}

/// Parse the overall tag array
///
/// Is is specified using syntax similar to Perl or Ruby
///  - `@F[0]`, `@F[-1]`, `@F[..]`, `@F[1..]`, `@F[..3]`, `@F[2,4]`, `@F[2,4,6]`
fn tag_array(i: Span) -> IResult<Span, Expr> {
    preceded(
        tag("@F").or(tag("$F")),
        opt(alt((
            delimited(tag("["), parse_index_multiple, tag("]")),
            expect_delimited(tag("["), parse_index_range, tag("]"), |q, r| {
                q.log_error("missing a closing bracket")
                    .add_range(r, "")
                    .add_solution("Add a closing bracket to complete the index")
                    .print_err();
            }),
        ))),
    )
    .map(|t| Expr::Tag(t.unwrap_or_else(|| Idx::new_range(0, i64::MAX))))
    .parse(i)
}

/// Parse the overall tag array using exact Perl/Ruby syntax
///
///  - `$F[0]`, `$F[-1]`, `$F[..]`, `$F[2,4]` | `@F`
#[allow(dead_code)]
fn tag_array_perl_like(i: Span) -> IResult<Span, Expr> {
    alt((
        preceded(
            tag("$F"),
            opt(alt((
                delimited(tag("["), parse_index_multiple, tag("]")),
                expect_delimited(tag("["), parse_index_range, tag("]"), |q, r| {
                    q.log_error("missing a closing bracket")
                        .add_range(r, "")
                        .add_solution("Add a closing bracket to complete the index")
                        .print_err();
                }),
            ))),
        )
        .map(|t| Expr::Tag(t.unwrap_or_else(|| Idx::new_range(0, i64::MAX)))),
        tag("@F").map(|_| Expr::Tag(Idx::new_range(0, i64::MAX))),
    ))
    .parse(i)
}

// ========================== Pattern Matching ========================
// ====================================================================

/// Parse a regular expression query
///
/// A regular expression can be used with the following syntaxes:
///  - `/regex/`
///  - `%r{regex}` (any delimiter)
///
/// See [`not_closing`]
fn parse_regex(i: Span) -> IResult<Span, Search> {
    alt((
        pair(
            expect_delimited(
                tag(&*format!("%r{}", i.extra.delim().unwrap_or('{'))).and(multispace0),
                ident_custom_delim.delimited_by(ws0),
                tag(&*format!("{}", i.extra.closing_delim().unwrap_or('}'))),
                |q, r| {
                    q.log_error("missing a terminating delimiter for regex")
                        .add_range(r, "")
                        .add_solution(&format!(
                            "Add a final terminating char (`{}`)",
                            i.extra.closing_delim().unwrap_or('}')
                        ))
                        .print_err();
                },
            ),
            many0(
                recognize(preceded(tag("-"), one_of("iu")).or(one_of("riImuUxl")))
                    .map(|s: Span| (*s.fragment()).to_string()),
            ),
        ),
        pair(
            delimited(
                tag("/").and(multispace0),
                ident_slash.delimited_by(ws0),
                tag("/"),
            ),
            many1(
                recognize(preceded(tag("-"), one_of("iu")).or(one_of("riImuUxl")))
                    .map(|s: Span| (*s.fragment()).to_string()),
            ),
        ),
    ))
    .map(|(patt, tobe_flags)| {
        let flags = SearchFlags::from_vec(&tobe_flags);
        Search::new_regex(patt, &flags)
    })
    .parse(i)
}

/// Parse an **explicit** glob query
///
/// A glob expression can be used with the following syntaxes:
///  - `%g{regex}` (any delimiter)
///
/// See [`not_closing`]
fn parse_glob(i: Span) -> IResult<Span, Search> {
    // Only `glob` needs an `expect_fn` since it is parsed second
    alt((
        pair(
            expect_delimited(
                tag(&*format!("%g{}", i.extra.delim().unwrap_or('{'))).and(multispace0),
                ident_custom_delim.delimited_by(ws0),
                tag(&*format!("{}", i.extra.closing_delim().unwrap_or('}'))),
                |q, r| {
                    q.log_error("missing a terminating delimiter for regex")
                        .add_range(r, "")
                        .add_solution(&format!(
                            "Add a final terminating char (`{}`)",
                            i.extra.closing_delim().unwrap_or('}')
                        ))
                        .print_err();
                },
            ),
            expect_fn(
                many0(
                    recognize(preceded(tag("-"), one_of("iu")).or(one_of("giImuUxl")))
                        .map(|s: Span| (*s.fragment()).to_string()),
                ),
                |q, r| {
                    q.log_error("missing a flag")
                        .add_range(r, "")
                        .add_solution(&SearchFlags::error_message())
                        .print_err();
                },
            ),
        ),
        pair(
            expect_delimited(
                tag("/").and(multispace0),
                ident_slash.delimited_by(ws0),
                tag("/"),
                |q, r| {
                    q.log_error("missing a terminating slash")
                        .add_range(r, "")
                        .add_solution("Add a final forward-slash (`/`)")
                        .print_err();
                },
            ),
            expect_fn(
                many1(
                    recognize(preceded(tag("-"), one_of("iu")).or(one_of("giImuUxl")))
                        .map(|s: Span| (*s.fragment()).to_string()),
                ),
                |q, r| {
                    q.log_error("missing a flag")
                        .add_range(r, "^ Flags: griImuUxl")
                        .add_solution(&SearchFlags::error_message())
                        .print_err();
                },
            ),
        ),
    ))
    .map(|(patt, tobe_flags)| {
        let flags = SearchFlags::from_vec(&tobe_flags.unwrap_or_else(Vec::new));
        Search::new_glob(patt, &flags)
    })
    .parse(i)
}

/// Parse an **implicit** glob query
///
/// That is: a pattern that contains an asterisk that is not escaped
fn parse_glob_implicit(i: Span) -> IResult<Span, Search> {
    ident_detect_glob
        .delimited_by(ws0)
        .map(|f| Search::new_glob(f, &[]))
        .parse(i)
}

// =========================== Higher-Level ===========================
// ====================================================================

/// Parses the lowest level single [`Expr`]
fn atomic(i: Span) -> IResult<Span, Expr> {
    let value = alt((quoted_string, ident)).map(|p| Expr::Pattern(Search::new_exact(p, false)));
    let pattern = alt((parse_regex, parse_glob)).map(Expr::Pattern);
    let glob_implicit = parse_glob_implicit.map(Expr::Pattern);
    let funcs = alt((func_value, func_tag, func_hash));
    let paren = expect_delimited(tag("("), parse_expr.delimited_by(ws0), tag(")"), |q, r| {
        q.log_error("missing a terminating parenthesis")
            .add_range(r, "")
            .add_solution("Add a final parenthesis")
            .add_solution("Remove the first parenthesis")
            .print_err();
    })
    .map(|f| Expr::Paren(Box::new(f)));

    alt((
        conditional_expr,
        funcs,
        pattern,
        paren,
        tag_array,
        glob_implicit,
        value,
    ))
    .parse(i)
}

/// Parses an optional unary [`Expr`], else uses `atomic`
fn unary(i: Span) -> IResult<Span, Expr> {
    let (rest, opt) = opt(unary_operator)(i)?;

    match opt {
        None => atomic(rest),
        Some(op) => expect_fn(atomic, |e, r| {
            e.log_error("expected a unary operator expression")
                .add_range(r, "")
                .add_solution("Add an operand to the unary operator")
                .add_solution("Remove the unary operator")
                .print_err();
        })
        .map(|o| Expr::unary_op(op.clone(), o.unwrap_or(Expr::Error)))
        .parse(rest),
    }
}

/// Parses a comparison [`Expr`] built from the `unary` function
fn cmp_expr(i: Span) -> IResult<Span, Expr> {
    let cmp = map(
        unary.and(opt(pair(
            comparison_operator.delimited_by(ws0),
            expect(unary, "expected a right-hand-side of query"),
        ))),
        |(l, r)| match r {
            None => l,
            Some((op, right)) => Expr::comparison_op(op, l, right.unwrap_or(Expr::Error)),
        },
    );

    cmp.preceded_by(multispace0).parse(i)
}

/// Parses a ternary [`Expr`] built from the `cmp_expr` function (`a ? b : c`)
///
/// The 'else' expression (`:`) is optional
fn tern_expr(i: Span) -> IResult<Span, Expr> {
    let (rest, parsed) = cmp_expr(i)?;
    let query = rest.extra;

    let res = fold_many0(
        pair(
            ws0.precedes(with_position(tag("?"))),
            opt(ws0.precedes(cmp_expr)),
        )
        .map(|(p_tern, opt)| {
            opt.unwrap_or_else(|| {
                query
                    .log_error("expected an operand for the `?` operator (true clause)")
                    .add_range(p_tern.range, "- leftover '?'")
                    .add_solution("Remove the `?` operator")
                    .add_solution("Add a second operand")
                    .print_err();

                Expr::Error
            })
        })
        .and(opt(pair(
            ws0.precedes(with_position(tag(":"))),
            opt(ws0.precedes(cmp_expr)),
        )
        .map(|(p_tern, opt)| {
            opt.unwrap_or_else(|| {
                query
                    .log_error("expected an operand for the `:` operator (false clause)")
                    .add_range(p_tern.range, "- leftover ':'")
                    .add_solution("Remove the `:` operator")
                    .add_solution("Add a second operand")
                    .print_err();

                Expr::Error
            })
        }))),
        || parsed.clone(),
        |cond, (tr, fa)| {
            if let Some(fals) = fa {
                Expr::conditional(ConditionalKind::Ternary, cond, tr, fals)
            } else {
                Expr::conditional(ConditionalKind::Ternary, cond, tr, Expr::Empty)
            }
        },
    )
    .parse(rest);

    res
}

/// Parses a logical and [`Expr`] built from the `tern_expr` function
fn logical_and(i: Span) -> IResult<Span, Expr> {
    let (rest, parsed) = tern_expr(i)?;
    let query = rest.extra;

    // Assignment is needed
    let res = fold_many0(
        alt((
            pair(
                ws0.precedes(with_position(tag_no_case("and"))),
                opt(ws0.precedes(cmp_expr)),
            ),
            pair(with_position(tag("&&").delimited_by(ws0)), opt(cmp_expr)),
        ))
        .map(|(p_and, opt)| {
            opt.unwrap_or_else(|| {
                query
                    .log_error("expected an operand for the `and` operator")
                    .add_range(p_and.range, "- leftover 'and'")
                    .add_solution("Remove the `&&` or `and` operator")
                    .add_solution("Add a second operand")
                    .print_err();

                Expr::Error
            })
        }),
        || parsed.clone(),
        |lhs, rhs| Expr::logical_op(LogicalOp::And, lhs, rhs),
    )
    .parse(rest);

    res
}

/// Parses a logical or [`Expr`] built from the `logical_and` function
fn logical_or(i: Span) -> IResult<Span, Expr> {
    let (rest, parsed) = logical_and(i)?;
    let query = rest.extra;

    // Assignment is needed
    let res = fold_many0(
        alt((
            pair(
                ws0.precedes(with_position(tag("or"))),
                opt(ws0.precedes(logical_and)),
            ),
            pair(with_position(tag("||").delimited_by(ws0)), opt(logical_and)),
        ))
        .map(|(p_or, opt)| {
            opt.unwrap_or_else(|| {
                query
                    .log_error("expected an operand for the `or` operator")
                    .add_range(p_or.range, "- leftover 'or'")
                    .add_solution("Remove the `||` or `or` operator")
                    .add_solution("Add a second operand")
                    .print_err();

                Expr::Error
            })
        }),
        || parsed.clone(),
        |lhs, rhs| Expr::logical_op(LogicalOp::Or, lhs, rhs),
    )
    .parse(rest);

    res
}

/// Starts the recursive parsing of an [`Expr`]
/// Begins with the `logical_or` function
fn opt_expr(i: Span) -> IResult<Span, Expr> {
    logical_or.preceded_by(ws0).parse(i)
}

// ============================== Query ===============================
// ====================================================================

// TODO: Possibly make it all consuming

/// If the `expr` function fails, then
///   - consume the input up to the `sync_point`
///   - log the error to the console
///   - and return a tuple of the input sliced to the end, with an `Expr::Error`
///
/// An unnecessary wrap is needed here to pass this function to
/// [`expect_delimited`]
#[allow(clippy::unnecessary_wraps)]
fn parse_expr(input: Span) -> IResult<Span, Expr> {
    Ok(opt_expr.parse(input).unwrap_or_else(|_| {
        let mut pnt = input.to_sync_point();

        if pnt.is_empty() && pnt.start > 0 {
            pnt.start -= 1;
        }

        input
            .extra
            .log_error("expected an expression to be all consuming")
            .add_range(pnt.clone(), "Possibly used an invalid character")
            .add_solution("The following chars need to be escaped: ")
            .add_solution("\'\"\\(){}=<>!&|?:,")
            .add_solution("The forward slash (/) needs to be escaped if query starts with it")
            .print_err();

        let end = pnt.end - input.location_offset();
        (input.slice(end..), Expr::Error)
    }))
}

/// Convert a [`Query`] to a [`ParsedQuery`]
///
/// There is no need to return any kind of error object because the error is
/// logged to the console to provide hints to the user on how to correct
/// their query
///
/// # Errors
/// Returns `()` because all errors are printed to the screen during the parsing
/// phase
pub(super) fn parse_query(input: &Query) -> Result<ParsedQuery, ()> {
    let s = Span::new_extra(input.query(), input);
    let (rest, parsed) = parse_expr(s).map_err(|_| ())?;

    if rest.extra.error_count() > 0 {
        Err(())
    } else {
        Ok(ParsedQuery::new(parsed, input.query()))
    }
}
