//! Test the parsing of the miniature language

use super::{
    ast::{
        error::{ErrDebug, ErrorReporter},
        query::{ParsedQuery, Query},
        time::TimeFilter,
        Expr,
    },
    parser::parse_query,
};
use annotate_snippets::{display_list::DisplayList, snippet::Snippet};
use expect_test::{expect, Expect};
use std::{cell::RefCell, rc::Rc};

/// Check whether the [`ParsedQuery`] matches a printed structure evaluated
/// by the [`expect`](expect_test::expect) macro
fn check(actual: &(ParsedQuery, &Vec<String>), expect: &Expect) {
    let actual_errors = actual.1.join("\n");
    let actual_combined = format!("{:#?}\n{}", actual.0, actual_errors);
    expect.assert_eq(&actual_combined);
}

fn query_ok(query_in: &str, expect: &Expect) {
    let errors = Rc::new(RefCell::new(vec![]));
    let parsed = {
        let q = Query::new(
            query_in.to_string(),
            Some(Box::new(TestErrorReporter::new(errors.clone()))),
        );
        let pq = parse_query(&q);
        pq.unwrap_or_else(|_| ParsedQuery::new(Expr::Vec(vec![]), ""))
    };

    check(&(parsed, errors.borrow().as_ref()), expect);
}

/// Sample error reporting struct
#[derive(Debug)]
struct TestErrorReporter {
    errors: Rc<RefCell<Vec<String>>>,
}

/// Allow for debug messages to be printed
impl ErrDebug for TestErrorReporter {}

/// Allow the printing of errors to the console
impl ErrorReporter for TestErrorReporter {
    fn handle_error(&self, s: Snippet) {
        let dl = DisplayList::from(s);
        self.errors.borrow_mut().push(format!("{}", dl));
    }
}

impl TestErrorReporter {
    fn new(errors: Rc<RefCell<Vec<String>>>) -> Self {
        Self { errors }
    }
}

// ============================ Equivalency ===========================
// ====================================================================

// #[test]
fn utf8_is_ok() {
    let q = Query::new("֍؏😎", None);
    assert!(q.parse().is_ok());
}

// TODO: Do not need the regex flag for slashes
// #[test]
fn regex_flag_does_nothing_implicit() {
    let q1 = Query::new("/abc/-i-u", None);
    let q2 = Query::new("/abc/r-i-u", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("/abc/Im", None);
    let q2 = Query::new("/abc/Imr", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("/abc/Imr-ux", None);
    let q2 = Query::new("/abc/Im-ux", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn regex_flag_does_nothing_explicit() {
    let q1 = Query::new("%r/abc/-i-u", None);
    let q2 = Query::new("%r/abc/r-i-u", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("%r/abc/Im", None);
    let q2 = Query::new("%r/abc/Imr", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("%r/abc/Imr-ux", None);
    let q2 = Query::new("%r/abc/Im-ux", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn glob_flag_does_nothing_explicit() {
    let q1 = Query::new("%g/abc/-i-u", None);
    let q2 = Query::new("%g/abc/g-i-u", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("%g/abc/Im", None);
    let q2 = Query::new("%g/abc/Img", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("%g/abc/Img-ux", None);
    let q2 = Query::new("%g/abc/Im-ux", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_flags() {
    let q1 = Query::new("/abc/-i", None);
    let q2 = Query::new("/abc/I", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("/abc/-u", None);
    let q2 = Query::new("/abc/U", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

// TODO: Do not need the regex flag for slashes
// #[test]
fn equivalent_expressions_regex() {
    let q1 = Query::new("/abc/I", None);
    let q2 = Query::new("%r/abc/I", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("/abc/I", None);
    let q2 = Query::new("/abc/rI", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("%r/abc/I", None);
    let q2 = Query::new("/abc/rI", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_expressions_glob() {
    let q1 = Query::new("%g/abc/I", None);
    let q2 = Query::new("/abc/gI", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_logical() {
    let q1 = Query::new("a and b", None);
    let q2 = Query::new("a && b", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("a or b", None);
    let q2 = Query::new("a || b", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("a or b and c", None);
    let q2 = Query::new("a || b && c", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("a || b and c", None);
    let q2 = Query::new("a or b && c", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_logical_not() {
    let q1 = Query::new("!foo", None);
    let q2 = Query::new("not foo", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("a and !foo", None);
    let q2 = Query::new("a and not foo", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("a && !foo", None);
    let q2 = Query::new("a && not foo", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("a or !foo", None);
    let q2 = Query::new("a || not foo", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("a or not foo", None);
    let q2 = Query::new("a || !foo", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_cmp() {
    let q1 = Query::new("foo == bar", None);
    let q2 = Query::new("foo eq bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo != bar", None);
    let q2 = Query::new("foo ne bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo < bar", None);
    let q2 = Query::new("foo lt bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo > bar", None);
    let q2 = Query::new("foo gt bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo <= bar", None);
    let q2 = Query::new("foo le bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo >= bar", None);
    let q2 = Query::new("foo ge bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_cmp_spaces_do_nothing() {
    let q1 = Query::new("foo == bar", None);
    let q2 = Query::new("foo==bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo != bar", None);
    let q2 = Query::new("foo!=bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo < bar", None);
    let q2 = Query::new("foo<bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo > bar", None);
    let q2 = Query::new("foo>bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo <= bar", None);
    let q2 = Query::new("foo<=bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo >= bar", None);
    let q2 = Query::new("foo>=bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_logical_spaces_do_nothing() {
    let q1 = Query::new("foo && bar", None);
    let q2 = Query::new("foo&&bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo || bar", None);
    let q2 = Query::new("foo||bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_ternary_spaces_do_nothing() {
    let q1 = Query::new("foo ? bar", None);
    let q2 = Query::new("foo?bar", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("foo ? bar : fig", None);
    let q2 = Query::new("foo?bar:fig", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_if_spaces_do_nothing() {
    let q1 = Query::new("if foo { bar }", None);
    let q2 = Query::new("if foo {bar}", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("if foo { bar } else { fig }", None);
    let q2 = Query::new("if foo {bar}else {fig}", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_unless_spaces_do_nothing() {
    let q1 = Query::new("unless foo { bar }", None);
    let q2 = Query::new("unless foo {bar}", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("unless foo { bar } else { fig }", None);
    let q2 = Query::new("unless foo {bar}else {fig}", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());
}

#[test]
fn equivalent_tag_array() {
    let q1 = Query::new("@F", None);
    let q2 = Query::new("@F[..]", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("@F[..]", None);
    let q2 = Query::new("@F[0..9223372036854775807]", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("@F[..]", None);
    let q2 = Query::new("@F[  ..  ]", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("@F[1..]", None);
    let q2 = Query::new("@F[1..9223372036854775807]", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("@F[1..]", None);
    let q2 = Query::new("@F[  1.. ]", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("@F[..2]", None);
    let q2 = Query::new("@F[0..2]", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("@F[1,2,3]", None);
    let q2 = Query::new("@F[1,    2,           3]", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q1 = Query::new("@F[1,2,3]", None);
    let q2 = Query::new("@F[   1,    2, 3  ]", None);
    assert_eq!(q1.parse().unwrap().parsed(), q2.parse().unwrap().parsed());

    let q = Query::new("@F[=]", None);
    assert!(q.parse().is_err());

    let q = Query::new("@F[..=]", None);
    assert!(q.parse().is_err());

    let q = Query::new("@F[,]", None);
    assert!(q.parse().is_err());

    let q = Query::new("@F[", None);
    assert!(q.parse().is_err());

    let q = Query::new("@F[]", None);
    assert!(q.parse().is_err());
}

// ==========================Escape Sequences =========================
// ====================================================================

#[test]
fn escaped_chars() {
    let q = Query::new("a\\{\\}\\(\\)\\=\\<\\>\\!\\&\\|\\?\\:\\,\\ \\'", None);
    assert!(q.parse().is_ok());

    let q = Query::new("a\\ bcd", None);
    assert!(q.parse().is_ok());

    let q = Query::new("a\\/bcd", None);
    assert!(q.parse().is_ok());

    let q = Query::new("a/bcd", None);
    assert!(q.parse().is_ok());
}

#[test]
fn esc_test_non_escape() {
    query_ok("a\\{\\}\\(\\)\\=\\<\\>\\!\\&\\|\\?\\:\\,\\ \\'", &expect![
        [r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "a{}()\\=<>!&|?:, '",
                    t: Exact,
                },
            ),
            raw: "a\\{\\}\\(\\)\\=\\<\\>\\!\\&\\|\\?\\:\\,\\ \\'",
        }
        "#]
    ]);

    query_ok("aa\\ bb", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "aa bb",
                    t: Exact,
                },
            ),
            raw: "aa\\ bb",
        }
        "#]]);

    query_ok("\\/hi/r", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "/hi/r",
                    t: Exact,
                },
            ),
            raw: "\\/hi/r",
        }
        "#]]);

    query_ok("/hi/r", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "hi",
                    t: Regex,
                },
            ),
            raw: "/hi/r",
        }
        "#]]);
}

#[test]
fn glob_escaped_astrisk() {
    query_ok("aa\\*b", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "aa*b",
                    t: Exact,
                },
            ),
            raw: "aa\\*b",
        }
        "#]]);
}

#[test]
fn glob_non_escaped_astrisk() {
    query_ok("aa*b", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "^(?-i)aa([^/]*)(?-i)b$",
                    t: Glob,
                },
            ),
            raw: "aa*b",
        }
        "#]]);
}

// ============================= Tag Array ============================
// ====================================================================

#[test]
fn tag_index_range_complete() {
    query_ok("@F[3..8]", &expect![[r#"
        ParsedQuery {
            parsed: Tag(
                Range(
                    3..8,
                ),
            ),
            raw: "@F[3..8]",
        }
        "#]]);
}

#[test]
fn tag_index_range_no_start() {
    query_ok("@F[..8]", &expect![[r#"
        ParsedQuery {
            parsed: Tag(
                Range(
                    0..8,
                ),
            ),
            raw: "@F[..8]",
        }
        "#]]);
}

#[test]
fn tag_index_range_no_end() {
    query_ok("@F[8..]", &expect![[r#"
        ParsedQuery {
            parsed: Tag(
                Range(
                    8..9223372036854775807,
                ),
            ),
            raw: "@F[8..]",
        }
        "#]]);
}

#[test]
fn tag_index_range_no_start_no_end() {
    query_ok("@F[..]", &expect![[r#"
        ParsedQuery {
            parsed: Tag(
                Range(
                    0..9223372036854775807,
                ),
            ),
            raw: "@F[..]",
        }
        "#]]);
}

#[test]
fn tag_index_range_complete_inclusive() {
    query_ok("@F[3..=8]", &expect![[r#"
        ParsedQuery {
            parsed: Tag(
                RangeInclusive(
                    3..=8,
                ),
            ),
            raw: "@F[3..=8]",
        }
        "#]]);
}

#[test]
fn tag_index_range_no_start_inclusive() {
    query_ok("@F[..=8]", &expect![[r#"
        ParsedQuery {
            parsed: Tag(
                RangeInclusive(
                    0..=8,
                ),
            ),
            raw: "@F[..=8]",
        }
        "#]]);
}

#[test]
fn tag_index_range_no_end_inclusive() {
    let q = Query::new("@F[8..=]", None);
    assert!(q.parse().is_err());
}

#[test]
fn tag_index_range_no_start_no_end_inclusive() {
    let q = Query::new("@F[..=]", None);
    assert!(q.parse().is_err());
}

#[test]
fn tag_index_single() {
    query_ok("@F[1]", &expect![[r#"
        ParsedQuery {
            parsed: Tag(
                Index(
                    [
                        1,
                    ],
                ),
            ),
            raw: "@F[1]",
        }
        "#]]);
}

#[test]
fn tag_index_double() {
    query_ok("@F[1,5]", &expect![[r#"
        ParsedQuery {
            parsed: Tag(
                Index(
                    [
                        1,
                        5,
                    ],
                ),
            ),
            raw: "@F[1,5]",
        }
        "#]]);
}

#[test]
fn tag_index_many() {
    query_ok("@F[1,5,7,8,2]", &expect![[r#"
        ParsedQuery {
            parsed: Tag(
                Index(
                    [
                        1,
                        5,
                        7,
                        8,
                        2,
                    ],
                ),
            ),
            raw: "@F[1,5,7,8,2]",
        }
        "#]]);
}

// ============================== Flags ===============================
// ====================================================================

#[test]
fn regex_single_flag() {
    query_ok("/hi/ri", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "(?i)hi",
                    t: Regex,
                },
            ),
            raw: "/hi/ri",
        }
        "#]]);
}

#[test]
fn regex_single_negated_flag() {
    query_ok("/hi/r-i", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "(?-i)hi",
                    t: Regex,
                },
            ),
            raw: "/hi/r-i",
        }
        "#]]);
}

#[test]
fn exact_single() {
    query_ok("hi", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "hi",
                    t: Exact,
                },
            ),
            raw: "hi",
        }
        "#]]);
}

#[test]
fn negated_single() {
    query_ok("!foo", &expect![[r#"
        ParsedQuery {
            parsed: Unary {
                op: Not,
                operand: Pattern(
                    Search {
                        inner: "foo",
                        t: Exact,
                    },
                ),
            },
            raw: "!foo",
        }
        "#]]);
}

// ============================ Comparison ============================
// ====================================================================

#[test]
fn cmp_equal() {
    query_ok("a == b", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Comparison(
                    Equal,
                ),
                lhs: Pattern(
                    Search {
                        inner: "a",
                        t: Exact,
                    },
                ),
                rhs: Pattern(
                    Search {
                        inner: "b",
                        t: Exact,
                    },
                ),
            },
            raw: "a == b",
        }
        "#]]);
}

#[test]
fn cmp_not_equal() {
    query_ok("a != b", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Comparison(
                    NotEqual,
                ),
                lhs: Pattern(
                    Search {
                        inner: "a",
                        t: Exact,
                    },
                ),
                rhs: Pattern(
                    Search {
                        inner: "b",
                        t: Exact,
                    },
                ),
            },
            raw: "a != b",
        }
        "#]]);
}

// ============================== Quoted ==============================
// ====================================================================

#[test]
fn single_quoted_single() {
    query_ok("'hi*'", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "hi*",
                    t: Exact,
                },
            ),
            raw: "'hi*'",
        }
        "#]]);
}

#[test]
fn single_quoted_paren() {
    query_ok("a and 'b)'", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    And,
                ),
                lhs: Pattern(
                    Search {
                        inner: "a",
                        t: Exact,
                    },
                ),
                rhs: Pattern(
                    Search {
                        inner: "b)",
                        t: Exact,
                    },
                ),
            },
            raw: "a and 'b)'",
        }
        "#]]);
}

// =========================== Pattern Match ==========================
// ====================================================================

// TODO: Do not need the regex flag for slashes
// #[test]
fn regex_slash_single() {
    query_ok("/hi/", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "hi",
                    t: Regex,
                },
            ),
            raw: "/hi/",
        }
        "#]]);
}

#[test]
fn regex_symbol_single() {
    query_ok("%r{hi}", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "hi",
                    t: Regex,
                },
            ),
            raw: "%r{hi}",
        }
        "#]]);
}

#[test]
fn glob_symbol_single() {
    query_ok("%g{hi}", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "^(?-i)hi$",
                    t: Glob,
                },
            ),
            raw: "%g{hi}",
        }
        "#]]);
}

#[test]
fn glob_implicit_single() {
    query_ok("hi*", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "^(?-i)hi([^/]*)$",
                    t: Glob,
                },
            ),
            raw: "hi*",
        }
        "#]]);
}

#[test]
fn regex_paren() {
    query_ok("%r/reg(ex)?[es]+/", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "reg(ex)?[es]+",
                    t: Regex,
                },
            ),
            raw: "%r/reg(ex)?[es]+/",
        }
        "#]]);
}

#[test]
fn regex_allow_symbols() {
    query_ok("%r{rege(x(es)?|x&\\>ps?|(oog)+e)}", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "rege(x(es)?|x&\\>ps?|(oog)+e)",
                    t: Regex,
                },
            ),
            raw: "%r{rege(x(es)?|x&\\>ps?|(oog)+e)}",
        }
        "#]]);
}

#[test]
fn glob_expand() {
    query_ok("%g/{*.rs,*.py}/", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "^((?:(?:[^/]*)(?-i)\\.rs)|(?:(?:[^/]*)(?-i)\\.py))$",
                    t: Glob,
                },
            ),
            raw: "%g/{*.rs,*.py}/",
        }
        "#]]);
}

#[test]
fn glob_0n_times() {
    query_ok("%g/<*.rs:0,>/", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "^((?:(?:[^/]*)(?-i)\\.rs){0,})$",
                    t: Glob,
                },
            ),
            raw: "%g/<*.rs:0,>/",
        }
        "#]]);
}

#[test]
fn glob_0n_times_alt() {
    query_ok("/<*.rs:0,>/g", &expect![[r#"
        ParsedQuery {
            parsed: Pattern(
                Search {
                    inner: "^((?:(?:[^/]*)(?-i)\\.rs){0,})$",
                    t: Glob,
                },
            ),
            raw: "/<*.rs:0,>/g",
        }
        "#]]);
}

// ============================= Logical ==============================
// ====================================================================

#[test]
fn and_literal_single() {
    query_ok("abc and def", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    And,
                ),
                lhs: Pattern(
                    Search {
                        inner: "abc",
                        t: Exact,
                    },
                ),
                rhs: Pattern(
                    Search {
                        inner: "def",
                        t: Exact,
                    },
                ),
            },
            raw: "abc and def",
        }
        "#]]);
}

#[test]
fn and_symbol_single() {
    query_ok("abc && def", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    And,
                ),
                lhs: Pattern(
                    Search {
                        inner: "abc",
                        t: Exact,
                    },
                ),
                rhs: Pattern(
                    Search {
                        inner: "def",
                        t: Exact,
                    },
                ),
            },
            raw: "abc && def",
        }
        "#]]);
}

#[test]
fn and_literal_double() {
    query_ok("a and b and c", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    And,
                ),
                lhs: Binary {
                    op: Logical(
                        And,
                    ),
                    lhs: Pattern(
                        Search {
                            inner: "a",
                            t: Exact,
                        },
                    ),
                    rhs: Pattern(
                        Search {
                            inner: "b",
                            t: Exact,
                        },
                    ),
                },
                rhs: Pattern(
                    Search {
                        inner: "c",
                        t: Exact,
                    },
                ),
            },
            raw: "a and b and c",
        }
        "#]]);
}

#[test]
fn or_literal_single() {
    query_ok("abc or def", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    Or,
                ),
                lhs: Pattern(
                    Search {
                        inner: "abc",
                        t: Exact,
                    },
                ),
                rhs: Pattern(
                    Search {
                        inner: "def",
                        t: Exact,
                    },
                ),
            },
            raw: "abc or def",
        }
        "#]]);
}

#[test]
fn or_symbol_single() {
    query_ok("abc || def", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    Or,
                ),
                lhs: Pattern(
                    Search {
                        inner: "abc",
                        t: Exact,
                    },
                ),
                rhs: Pattern(
                    Search {
                        inner: "def",
                        t: Exact,
                    },
                ),
            },
            raw: "abc || def",
        }
        "#]]);
}

#[test]
fn and_or_literal_double() {
    query_ok("a or b and c", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    Or,
                ),
                lhs: Pattern(
                    Search {
                        inner: "a",
                        t: Exact,
                    },
                ),
                rhs: Binary {
                    op: Logical(
                        And,
                    ),
                    lhs: Pattern(
                        Search {
                            inner: "b",
                            t: Exact,
                        },
                    ),
                    rhs: Pattern(
                        Search {
                            inner: "c",
                            t: Exact,
                        },
                    ),
                },
            },
            raw: "a or b and c",
        }
        "#]]);
}

#[test]
fn and_regex() {
    query_ok("/abc/r and /def/r", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    And,
                ),
                lhs: Pattern(
                    Search {
                        inner: "abc",
                        t: Regex,
                    },
                ),
                rhs: Pattern(
                    Search {
                        inner: "def",
                        t: Regex,
                    },
                ),
            },
            raw: "/abc/r and /def/r",
        }
        "#]]);
}

#[test]
fn and_glob() {
    query_ok("%g{abc} && %g{def}", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    And,
                ),
                lhs: Pattern(
                    Search {
                        inner: "^(?-i)abc$",
                        t: Glob,
                    },
                ),
                rhs: Pattern(
                    Search {
                        inner: "^(?-i)def$",
                        t: Glob,
                    },
                ),
            },
            raw: "%g{abc} && %g{def}",
        }
        "#]]);
}

#[test]
fn or_tag_array() {
    query_ok("@F[..2] || @F[42]", &expect![[r#"
        ParsedQuery {
            parsed: Binary {
                op: Logical(
                    Or,
                ),
                lhs: Tag(
                    Range(
                        0..2,
                    ),
                ),
                rhs: Tag(
                    Index(
                        [
                            42,
                        ],
                    ),
                ),
            },
            raw: "@F[..2] || @F[42]",
        }
        "#]]);
}

// =========================== Conditional ============================
// ====================================================================

#[test]
fn ternary_no_else() {
    query_ok("abc ? def", &expect![[r#"
        ParsedQuery {
            parsed: Conditional {
                kind: Ternary,
                cond: Pattern(
                    Search {
                        inner: "abc",
                        t: Exact,
                    },
                ),
                if_true: Pattern(
                    Search {
                        inner: "def",
                        t: Exact,
                    },
                ),
                if_false: Empty,
            },
            raw: "abc ? def",
        }
        "#]]);
}

#[test]
fn ternary_no_with_else() {
    query_ok("abc ? def : ghi", &expect![[r#"
        ParsedQuery {
            parsed: Conditional {
                kind: Ternary,
                cond: Pattern(
                    Search {
                        inner: "abc",
                        t: Exact,
                    },
                ),
                if_true: Pattern(
                    Search {
                        inner: "def",
                        t: Exact,
                    },
                ),
                if_false: Pattern(
                    Search {
                        inner: "ghi",
                        t: Exact,
                    },
                ),
            },
            raw: "abc ? def : ghi",
        }
        "#]]);
}

#[test]
fn ternary_nested() {
    query_ok("a ? b ? c : d : e", &expect![[r#"
        ParsedQuery {
            parsed: Conditional {
                kind: Ternary,
                cond: Conditional {
                    kind: Ternary,
                    cond: Pattern(
                        Search {
                            inner: "a",
                            t: Exact,
                        },
                    ),
                    if_true: Pattern(
                        Search {
                            inner: "b",
                            t: Exact,
                        },
                    ),
                    if_false: Empty,
                },
                if_true: Pattern(
                    Search {
                        inner: "c",
                        t: Exact,
                    },
                ),
                if_false: Pattern(
                    Search {
                        inner: "d",
                        t: Exact,
                    },
                ),
            },
            raw: "a ? b ? c : d : e",
        }
        "#]]);
}

#[test]
    #[rustfmt::skip]
    fn ternary_stupid_complex() {
        query_ok(
            "\
            %r{a(b)?c+[a-z]} ? zz \
                : %r{bb+} ? @F \
                : %g{*.rs} ? @F[1,4] \
                : @F[3..6]",
            &expect![[r#"
            ParsedQuery {
                parsed: Conditional {
                    kind: Ternary,
                    cond: Conditional {
                        kind: Ternary,
                        cond: Conditional {
                            kind: Ternary,
                            cond: Pattern(
                                Search {
                                    inner: "a(b)?c+[a-z]",
                                    t: Regex,
                                },
                            ),
                            if_true: Pattern(
                                Search {
                                    inner: "zz",
                                    t: Exact,
                                },
                            ),
                            if_false: Pattern(
                                Search {
                                    inner: "bb+",
                                    t: Regex,
                                },
                            ),
                        },
                        if_true: Tag(
                            Range(
                                0..9223372036854775807,
                            ),
                        ),
                        if_false: Pattern(
                            Search {
                                inner: "^([^/]*)(?-i)\\.rs$",
                                t: Glob,
                            },
                        ),
                    },
                    if_true: Tag(
                        Index(
                            [
                                1,
                                4,
                            ],
                        ),
                    ),
                    if_false: Tag(
                        Range(
                            3..6,
                        ),
                    ),
                },
                raw: "%r{a(b)?c+[a-z]} ? zz : %r{bb+} ? @F : %g{*.rs} ? @F[1,4] : @F[3..6]",
            }
            "#]],
        );
    }

#[test]
fn if_no_else() {
    query_ok("if a { b }", &expect![[r#"
        ParsedQuery {
            parsed: Conditional {
                kind: If,
                cond: Pattern(
                    Search {
                        inner: "a",
                        t: Exact,
                    },
                ),
                if_true: Pattern(
                    Search {
                        inner: "b",
                        t: Exact,
                    },
                ),
                if_false: Empty,
            },
            raw: "if a { b }",
        }
        "#]]);
}

// ========================== Function Calls ==========================
// ====================================================================

#[test]
fn tag_func_exact() {
    query_ok("tag(exact)", &expect![[r#"
        ParsedQuery {
            parsed: FunctionCall(
                Tag {
                    term: Pattern(
                        Search {
                            inner: "exact",
                            t: Exact,
                        },
                    ),
                },
            ),
            raw: "tag(exact)",
        }
        "#]]);

    query_ok("t(exact)", &expect![[r#"
        ParsedQuery {
            parsed: FunctionCall(
                Tag {
                    term: Pattern(
                        Search {
                            inner: "exact",
                            t: Exact,
                        },
                    ),
                },
            ),
            raw: "t(exact)",
        }
        "#]]);
}

#[test]
fn tag_func_regex() {
    query_ok("tag(%r{regex?})", &expect![[r#"
        ParsedQuery {
            parsed: FunctionCall(
                Tag {
                    term: Pattern(
                        Search {
                            inner: "regex?",
                            t: Regex,
                        },
                    ),
                },
            ),
            raw: "tag(%r{regex?})",
        }
        "#]]);
}

#[test]
fn hash_func() {
    query_ok("hash(af1349b9f5f9a1)", &expect![[r#"
        ParsedQuery {
            parsed: FunctionCall(
                Hash {
                    term: Hash(
                        "af1349b9f5f9a1",
                    ),
                },
            ),
            raw: "hash(af1349b9f5f9a1)",
        }
        "#]]);
}

#[test]
fn value_func() {
    query_ok("value(@F[1,4])", &expect![[r#"
        ParsedQuery {
            parsed: FunctionCall(
                Value {
                    term: Tag(
                        Index(
                            [
                                1,
                                4,
                            ],
                        ),
                    ),
                },
            ),
            raw: "value(@F[1,4])",
        }
        "#]]);
}

// ============================ TimeFilter ============================
// ====================================================================

#[test]
fn timefilter_matches() {
    use chrono::offset::TimeZone;
    use std::time::Duration;

    let t = chrono::Local
        .datetime_from_str("2022-01-01 02:00:00", "%F %T")
        .unwrap()
        .into();

    // t > t - 1min
    assert!(TimeFilter::after(&t, "1min").unwrap().does_match(&t));
    // t !< t - 1min
    assert!(!TimeFilter::before(&t, "1min").unwrap().does_match(&t));

    let t1 = t - Duration::from_secs(300);
    // t - 5min < t - 4min
    assert!(TimeFilter::before(&t, "4 min").unwrap().does_match(&t1));
    //  t - 5min > t - 6min
    assert!(TimeFilter::after(&t, "6 min").unwrap().does_match(&t1));
    //  t - 5min !> t - 5min
    assert!(!TimeFilter::after(&t, "5 min").unwrap().does_match(&t1));
    //  t - 5min !< t - 5min
    assert!(!TimeFilter::before(&t, "5 min").unwrap().does_match(&t1));
}
