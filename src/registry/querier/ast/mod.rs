//! Structures to hold the parsed data

pub(crate) mod error;
pub(crate) mod query;
pub(crate) mod search;
pub(crate) mod time;
pub(crate) use query::Query;

use search::Search;
use std::{
    fmt,
    ops::{Range, RangeInclusive},
};

/// A comparison operator that requires more than one operand
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ComparisonOp {
    /// Are the operands equivalent?
    Equal,
    /// Are the operands not equivalent?
    NotEqual,
    /// Is operand A less than operand B?
    LessThan,
    /// Is operand A greater than operand B?
    GreaterThan,
    /// Is operand A less than or equal to operand B?
    LessThanOrEqual,
    /// Is operand A greater than or equal to operand B?
    GreaterThanOrEqual,
}

/// Logical operators that require more than one operand
#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum LogicalOp {
    /// Logically `and` combine operands
    And,
    /// Logically `or` combine operands
    Or,
}

/// An operator requiring a single operand
///
/// Only `not` is supported at this time
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    /// Negate the proceeding item
    Not,
}

/// A conditional operator kind
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConditionalKind {
    /// An if-statement
    If,
    /// An unless-statement
    Unless,
    /// A ternary-statement
    Ternary,
    // TODO: An if else-if statement
    // ElseIf,
}

/// A literal representation of an object
#[allow(unused)]
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Literal {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

/// An index into an array
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Idx<I> {
    /// A range of indexes
    ///
    /// Supported syntaxes: `[1..2]` | `[1..]` | `[..1]` | `[..]`
    Range(Range<I>),
    /// A range of indexes with an inclusive end
    ///
    /// Supported syntaxes: `[1..=2]` | `[..=1]`
    RangeInclusive(RangeInclusive<I>),
    /// A single or multiple indexes
    ///
    /// Supported syntaxes: `[1]` | `[1,2]` | `[1,2,4]`
    Index(Vec<I>),
}

/// Functions that are available for a query
#[allow(variant_size_differences)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Function {
    /// Function that matches the tag's values
    Value { term: Box<Expr> },
    /// Function that matches the tag name
    Tag { term: Box<Expr> },
    /// Function that matches the file's hash
    Hash { term: Box<Expr> },

    // TODO: ===

    // --- THESE: Function or tag as a result?
    /// Function that matches files that have an implied tag
    Implied { term: Box<Expr> },
    /// Function that matches files that have tags that imply another
    Implies { term: Box<Expr> },

    /// Function that matches a given time before
    Before { term: Box<Expr> },
    /// Function that matches a given time after
    After { term: Box<Expr> },

    /// Function that matches the file's size
    Size { term: Box<Expr> },
    /// Function that matches the file's `UID`
    Uid { term: Box<Expr> },
    /// Function that matches the file's `GID`
    Gid { term: Box<Expr> },
    /// Function that matches the file's modification time
    Mtime { term: Box<Expr> },
    /// Function that matches the file's creation time
    Ctime { term: Box<Expr> },
    /// Function that matches directories
    Dir,

    /// Function to print a search term
    Print { term: Box<Expr> },
}

// ╓                                                          ╖
// ║  These were created as their own structs to be able to   ║
// ║  implement `Display`, which translates to an SQL query   ║
// ╙                                                          ╜

/// A unary expression with a given operator
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UnaryExpr<T: fmt::Display> {
    /// The unary operator
    pub(crate) operator: T,
    /// The single operand
    pub(crate) operand:  Box<Expr>,
}

/// A binary expression with a given operator
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BinaryExpr<T> {
    /// The binary operator
    pub(crate) operator: T,
    /// Left-hand-side operand
    pub(crate) lhs:      Box<Expr>,
    /// Right-hand-side operand
    pub(crate) rhs:      Box<Expr>,
}

/// A conditonal expression with a given operator
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConditionalExpr<T: fmt::Display> {
    /// The kind of conditional statement
    kind:     T,
    /// The condition to test
    cond:     Box<Expr>,
    /// Value to return if `cond` is true
    if_true:  Box<Expr>,
    /// Value to return if `cond` is false
    if_false: Box<Expr>,
}

/// A parsed expression
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expr {
    /// An expression surrounded by parenthesis
    Paren(Box<Self>),
    /// A unary expression
    Unary(UnaryExpr<UnaryOp>),
    /// A comparison expression
    Comparison(BinaryExpr<ComparisonOp>),
    /// A logical expression
    Logical(BinaryExpr<LogicalOp>),
    /// A conditional expression
    Conditional(ConditionalExpr<ConditionalKind>),
    /// A function call
    FunctionCall(Function),

    /// A wrapper value where it falls into nothing else
    Value(String),
    /// A pattern search into the database
    Pattern(Search),
    /// The overall [`Tag`] array
    Tag(Idx<i64>),
    /// A file hash
    Hash(String),
    /// A literal object
    Literal(Literal),
    /// A vector of expressions
    Vec(Vec<Self>),

    /// An empty expression
    Empty,
    /// An error occurred during the search
    Error,
}

impl ComparisonOp {
    /// Negate the given operator
    pub(crate) const fn negate(&self) -> Self {
        match *self {
            Self::Equal => Self::NotEqual,
            Self::NotEqual => Self::Equal,
            Self::LessThan => Self::GreaterThan,
            Self::GreaterThan => Self::LessThan,
            Self::LessThanOrEqual => Self::GreaterThanOrEqual,
            Self::GreaterThanOrEqual => Self::LessThanOrEqual,
        }
    }
}

impl<I> Idx<I> {
    /// Create a new [`Idx::Range`]
    pub(crate) const fn new_range(start: I, end: I) -> Self {
        Self::Range(Range { start, end })
    }

    /// Create a new [`Idx::RangeInclusive`]
    pub(crate) const fn new_range_inclusive(start: I, end: I) -> Self {
        Self::RangeInclusive(RangeInclusive::new(start, end))
    }

    /// Create a new [`Idx::Index`]
    pub(crate) fn new_index(idx: Vec<I>) -> Self {
        Self::Index(idx)
    }
}

impl Expr {
    /// Create a `Literal` boolean value
    pub(crate) const fn boolean(val: bool) -> Self {
        Self::Literal(Literal::Boolean(val))
    }

    /// Create a `Literal` integer value
    pub(crate) const fn int(val: i64) -> Self {
        Self::Literal(Literal::Integer(val))
    }

    /// Create a `Literal` float value
    pub(crate) const fn float(val: f64) -> Self {
        Self::Literal(Literal::Float(val))
    }

    /// Create a `Literal` string value
    pub(crate) fn literal_string(val: &str) -> Self {
        Self::Literal(Literal::String(val.to_owned()))
    }

    /// Create a `Unary` operator expression
    pub(crate) fn unary_op(operator: UnaryOp, operand: Self) -> Self {
        Self::Unary(UnaryExpr {
            operator,
            operand: Box::new(operand),
        })
    }

    /// Create a `Binary` expression with a `Comparison` operator
    pub(crate) fn comparison_op(operator: ComparisonOp, lhs: Self, rhs: Self) -> Self {
        Self::Comparison(BinaryExpr {
            operator,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        })
    }

    /// Create a `Binary` expression with a `Logical` operator
    pub(crate) fn logical_op(operator: LogicalOp, lhs: Self, rhs: Self) -> Self {
        Self::Logical(BinaryExpr {
            operator,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        })
    }

    /// Create a `Conditional` expression
    pub(crate) fn conditional(
        kind: ConditionalKind,
        cond: Self,
        if_true: Self,
        if_false: Self,
    ) -> Self {
        Self::Conditional(ConditionalExpr {
            kind,
            cond: Box::new(cond),
            if_true: Box::new(if_true),
            if_false: Box::new(if_false),
        })
    }

    /// Create a `value` `FunctionCall` expression
    pub(crate) fn value_func(arg: Self) -> Self {
        Self::FunctionCall(Function::Value {
            term: Box::new(arg),
        })
    }

    /// Create a `tag` `FunctionCall` expression
    pub(crate) fn tag_func(arg: Self) -> Self {
        Self::FunctionCall(Function::Tag {
            term: Box::new(arg),
        })
    }

    /// Create a `hash` `FunctionCall` expression
    pub(crate) fn hash_func(arg: Self) -> Self {
        Self::FunctionCall(Function::Hash {
            term: Box::new(arg),
        })
    }

    /// Create an `mtime` `FunctionCall` expression
    pub(crate) fn mtime_func(arg: Self) -> Self {
        Self::FunctionCall(Function::Mtime {
            term: Box::new(arg),
        })
    }

    /// Create an `ctime` `FunctionCall` expression
    pub(crate) fn ctime_func(arg: Self) -> Self {
        Self::FunctionCall(Function::Mtime {
            term: Box::new(arg),
        })
    }

    /// Create a `uid` [`FunctionCall`] expression
    pub(crate) fn uid_func(arg: Self) -> Self {
        Self::FunctionCall(Function::Uid {
            term: Box::new(arg),
        })
    }

    /// Create a `gid` [`FunctionCall`] expression
    pub(crate) fn gid_func(arg: Self) -> Self {
        Self::FunctionCall(Function::Gid {
            term: Box::new(arg),
        })
    }

    /// Create a `print` `FunctionCall` expression
    pub(crate) fn print_func(arg: Self) -> Self {
        Self::FunctionCall(Function::Print {
            term: Box::new(arg),
        })
    }

    /// Create a [`Search`] object to search for exact keyword(s)
    pub(crate) fn search_raw<S: AsRef<str>>(str: S) -> Option<Self> {
        let s = str.as_ref();
        if s.is_empty() {
            None
        } else {
            Some(Self::Pattern(Search::new_exact(s, false)))
        }
    }

    /// Create a [`Search`] object to search for a glob
    pub(crate) fn search_glob<S: AsRef<str>>(str: S) -> Option<Self> {
        let s = str.as_ref();
        if s.is_empty() {
            None
        } else {
            Some(Self::Pattern(Search::new_glob(s, &[])))
        }
    }

    /// Create a [`Search`] object to search for a regex
    pub(crate) fn search_regex<S: AsRef<str>>(str: S) -> Option<Self> {
        let s = str.as_ref();
        if s.is_empty() {
            None
        } else {
            Some(Self::Pattern(Search::new_regex(s, &[])))
        }
    }
}

// === `Display` implementations ===================================

impl fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::LessThan => "<",
            Self::GreaterThan => ">",
            Self::LessThanOrEqual => "<=",
            Self::GreaterThanOrEqual => ">=",
        })
    }
}

impl fmt::Display for LogicalOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Self::And => " AND ",
            Self::Or => " OR ",
        })
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Self::Not => " NOT ",
        })
    }
}

impl fmt::Display for ConditionalKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Self::If | Self::Ternary => "CASE WHEN ",
            Self::Unless => "CASE WHEN NOT ",
        })
    }
}

// impl<T: fmt::Display> fmt::Display for ConditionalExpr<T> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         let mut s = format!(
//             "{} {} THEN {}",
//             self.kind.to_string(),
//             self.cond.to_string(),
//             self.if_true.to_string()
//         );
//
//         if self.if_false != Box::new(Expr::Empty) {
//             s.push_str(&format!(" ELSE {}", self.if_false.to_string()));
//         }
//
//         write!(f, "{}", s)
//     }
// }

// impl<T: fmt::Display> fmt::Display for UnaryExpr<T> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{} {}", self.operator.to_string(), self.operand)
//     }
// }
