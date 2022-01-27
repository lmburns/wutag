//! Structure that holds an `SQL` query and its parameters

use super::querier::ast::{
    query::ParsedQuery,
    search::{Search, SearchKind},
    BinaryExpr, ComparisonOp, Expr, LogicalOp, UnaryExpr, UnaryOp,
};
use anyhow::{anyhow, Context, Result};
use bytes::{Bytes, BytesMut};
use itertools::Itertools;
use rusqlite::{
    self as rsq, params,
    types::{ToSql, ToSqlOutput},
    Params,
};
use std::{
    fmt::{self, Write},
    ops::Deref,
    path::Path,
};

// ============================ SqlBuilder ============================

/// Builder for an `SQL` query
pub(crate) struct SqlBuilder {
    /// The `SQL` query as bytes
    query:  BytesMut,
    /// Parameters used for the `SQL` query
    params: Vec<Box<dyn ToSql>>,
    /// The index of the parameters
    pidx:   usize,
    /// Does the query need a comma? (i.e., there's more than one param)
    comma:  bool,
}

impl SqlBuilder {
    /// Create a new `SqlBuilder
    pub(crate) fn new() -> Self {
        Self {
            query:  BytesMut::new(),
            params: vec![],
            pidx:   1,
            comma:  false,
        }
    }

    // /// Return the `query` as a `Bytes` buf
    // pub(crate) fn as_buf(&self) -> &Bytes {
    //     &*self.query.freeze()
    // }

    /// Return the `query` as bytes
    pub(crate) fn as_bytes(&self) -> Vec<u8> {
        self.query.to_vec()
    }

    /// Return the `query` as a `String`
    pub(crate) fn utf(&self) -> Result<String> {
        String::from_utf8(self.query.to_vec()).context("failed to convert query to String")
    }

    /// Return the `params` as a vector of [`ToSqlOutput`]
    pub(crate) fn params_as_output(&self) -> Result<Vec<ToSqlOutput<'_>>> {
        self.params
            .iter()
            .map(|s| ToSql::to_sql(s).map_err(|e| anyhow!(e)))
            .into_iter()
            .collect()
    }

    /// Return the `params` as a slice where each element implements [`ToSql`]
    pub(crate) fn params_as_slice(&self) -> Vec<&dyn ToSql> {
        self.params.iter().map(Deref::deref).collect::<Vec<_>>()
    }

    /// Append a string to the query with a starting newline
    pub(crate) fn appendln<S: AsRef<str>>(&mut self, s: S) {
        // let chars = s.chars().collect::<Vec<_>>();
        // if chars[0] == ' ' || chars[0] == '\n' {}

        self.query.write_str("\n");
        self.query.write_str(s.as_ref());
        self.comma = false;
    }

    /// Append a string to the query
    pub(crate) fn append<S: AsRef<str>>(&mut self, s: S) {
        self.query.write_str(s.as_ref());
        self.comma = false;
    }

    /// Append a parameter to the vector of `params`
    pub(crate) fn append_param<S: ToSql + 'static>(&mut self, param: S) {
        if self.comma {
            self.query.write_str(",");
        }

        self.query.write_str(&format!("?{}", self.pidx));
        self.pidx += 1;

        self.params.push(Box::new(param));
        self.comma = true;
    }

    /// Append `COLLATE NOCASE` to ignore case when searching
    pub(crate) fn nocase_collation(&mut self, ignore: bool) {
        if ignore {
            self.query.write_str(" COLLATE NOCASE ");
        }
    }

    /// Instead of appending `COLLATE NOCASE` to the query, just return `COLLATE
    /// NOCASE` as a string if case is to be ignored for the query
    pub(crate) fn return_nocase_collation(ignore: bool) -> &'static str {
        ignore.then(|| " COLLATE NOCASE ").unwrap_or("")
    }

    /// Start a query for files, returning the count
    pub(crate) fn file_count_query<P: AsRef<Path>>(
        expr: &ParsedQuery,
        path: P,
        cwd: bool,
        explicit: bool,
        ignore_case: bool,
    ) -> Self {
        let mut builder = Self::new();
        builder.append(
            "SELECT count(id)
            FROM file
            WHERE",
        );

        builder
    }

    /// Handle query branch statements by appending the corresponding SQL
    pub(crate) fn file_handle_branch(&mut self, expr: &Expr, explicit: bool, ignore_case: bool) {
        match expr {
            Expr::Pattern(ref search) => match search.inner_t() {
                SearchKind::Exact => self.build_pattern_branch(search, explicit, ignore_case),
                SearchKind::Regex => println!("Regex query: {:#?}", expr.clone()),
                SearchKind::Glob => println!("Glob query: {:#?}", expr.clone()),
            },
            _ => println!("other"),
        }
    }

    /// Handle a comparison expression for a file query
    pub(crate) fn build_comparison_branch(&mut self, cmp: Expr, explicit: bool, ignore_case: bool) {
        let case = Self::return_nocase_collation(ignore_case);

        if let Expr::Comparison(BinaryExpr {
            mut operator,
            lhs,
            rhs,
        }) = cmp
        {
            // TODO: If a number: 'CAST(v.name as float)'
            let value = "v.name";

            if operator == ComparisonOp::NotEqual {
                operator = operator.negate();
                self.append(" not ");
            }

            if explicit {
                self.append(format!(
                    "id IN (
                        SELECT file_id
                        FROM file_tag
                        WHERE tag_id = (
                            SELECT id
                            FROM tag
                            WHERE name {} = ",
                    case
                ));

                // FIX: Finish
            }
        }
    }

    /// Handle a [`UnaryExpr`]. The only operator is `not`
    pub(crate) fn build_not_branch(
        &mut self,
        expr: UnaryExpr<UnaryOp>,
        explicit: bool,
        ignore_case: bool,
    ) {
        self.append(" NOT ");

        let UnaryExpr { operand, .. } = expr;
        self.file_handle_branch(&*operand, explicit, ignore_case);
    }

    /// Handle a [`Search`] pattern for files
    pub(crate) fn build_pattern_branch(
        &mut self,
        patt: &Search,
        explicit: bool,
        ignore_case: bool,
    ) {
        let case = Self::return_nocase_collation(ignore_case);

        if explicit {
            self.appendln(format!(
                "id IN (
                    SELECT file_id
                    FROM file_tag
                    WHERE tag_id = (
                        SELECT id
                        FROM tag
                        WHERE name {} = ",
                case
            ));
            self.append_param(patt.inner().clone());
            self.appendln("))");
        } else {
            self.appendln(format!(
                "id IN (
                    SELECT file_id
                    FROM file_tag
                    INNER JOIN (
                        WITH RECURSIVE working (tag_id, value_id) AS
                            (
                                SELECT id, 0
                                FROM tag
                                WHERE name {} = ",
                case
            ));
            self.append_param(patt.inner().clone());
            self.appendln(
                "UNION ALL
                    SELECT i.tag_id, i.value_id
                    FROM impl i, working
                    WHERE i.implied_tag_id = working.tag_id
                    AND
                    (
                        i.implied_value_id = working.value_id
                        OR
                        working.value_id = 0
                    )
                )
                SELECT tag_id, value_id
                FROM working
                ) imps
                ON file_tag.tag_id = imps.tag_id
                AND
                (
                    file_tag.value_id = imps.value_id
                    OR
                    imps.value_id = 0
                )
                )",
            );
        }
    }
}

impl fmt::Debug for SqlBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqlBuilder")
            .field("query", &self.query.to_vec())
            .field(
                "params",
                &self.params.iter().fold(String::new(), |mut acc, f| {
                    acc.push_str(&format!(" {:?}", f.to_sql()));
                    acc
                }),
            )
            .field("pidx", &self.pidx)
            .field("comma", &self.comma)
            .finish()
    }
}

// ============================== Sort ===============================

/// The method in which the files should be sorted in the database
#[derive(Debug, Copy, Clone)]
pub(crate) enum Sort {
    /// Sort by the `File` id
    Id,
    /// Sort by the `File` name
    Name,
    /// Sort by the `File` `mtime`
    ModificationTime,
    /// Sort by the `File` `ctime`
    CreationTime,
    /// Sort by the `File` `size`
    FileSize,
    /// Do not sort the `File`s
    None,
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Sort::Id => f.write_str("ORDER BY id"),
            Sort::Name => f.write_str("ORDER BY directory || '/' || name"),
            Sort::ModificationTime => f.write_str("ORDER BY mtime, directory || '/' || name"),
            Sort::CreationTime => f.write_str("ORDER BY ctime, directory || '/' || name"),
            Sort::FileSize => f.write_str("ORDER BY size, directory || '/' || name"),
            Sort::None => f.write_str(""),
        }
    }
}
