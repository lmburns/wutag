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
    self as rsq, named_params, params,
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
pub(crate) struct SqlBuilder<'a> {
    /// The `SQL` query as bytes
    query:        BytesMut,
    /// Parameters used for the `SQL` query
    params:       Vec<Box<dyn ToSql>>,
    /// Named parameters used for the `SQL` query
    named_params: Vec<(&'a str, &'a dyn ToSql)>,
    /// The index of the parameters
    pidx:         usize,
    /// Does the query need a comma? (i.e., there's more than one param)
    comma:        bool,
}

impl<'a> SqlBuilder<'a> {
    /// Create a new [`SqlBuilder`]
    pub(crate) fn new() -> Self {
        Self {
            query:        BytesMut::new(),
            params:       vec![],
            named_params: vec![],
            pidx:         1,
            comma:        false,
        }
    }

    /// Create a new [`SqlBuilder`] with an initial `query`
    pub(crate) fn new_initial(query: &str) -> Self {
        let mut s = Self::new();
        s.query = BytesMut::from(query);
        s
    }

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

    /// Return the `params` to be used with [`named_params`]
    pub(crate) fn named_params_as_slice(&self) -> &[(&str, &dyn ToSql)] {
        self.named_params.as_slice()
    }

    /// Append a string to the query with a starting newline
    pub(crate) fn appendln<S: AsRef<str>>(&mut self, s: S) {
        self.query.write_str("\n");
        self.query.write_str(s.as_ref());
        self.comma = false;
    }

    /// Append a string to the `query`
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

    /// Append a named parameter to the vector of `named_params`
    pub(crate) fn append_named_param<S: ToSql>(&mut self, named: &'a str, param: &'a S) {
        if self.comma {
            self.query.write_str(",");
        }

        self.query.write_str(&format!(":{}", named));
        self.pidx += 1;

        self.named_params.push((named, param));
        self.comma = true;
    }

    /// Return a string with `LIKE` wildcards surrounding the argument
    pub(crate) fn likenize(a: &str) -> String {
        format!("%{}%", a)
    }

    // TODO: Test unicase vs nocase

    /// Append `COLLATE NOCASE` to ignore case when searching
    pub(crate) fn nocase_collation(&mut self, ignore: bool) {
        if ignore {
            self.query.write_str(" COLLATE unicase ");
        }
    }

    /// Instead of appending `COLLATE NOCASE` to the query, just return `COLLATE
    /// NOCASE` as a string if case is to be ignored for the query
    pub(crate) fn return_nocase_collation(ignore: bool) -> &'static str {
        ignore.then(|| " COLLATE unicase ").unwrap_or("")
    }

    // ========================== Query Language ==========================

    /// Start a query for files, returning the count
    pub(crate) fn file_count_query<P: AsRef<Path>>(
        expr: &ParsedQuery,
        path: P,
        cwd: bool,
        ignore_case: bool,
    ) -> Self {
        let mut builder = Self::new();
        builder.append(
            "SELECT count(id)
            FROM file
            WHERE",
        );

        builder.file_handle_branch(expr.parsed(), ignore_case);
        // build_path_clause

        builder
    }

    /// Build a query for a file in the database
    pub(crate) fn build_query<P: AsRef<Path>>(
        expr: &ParsedQuery,
        path: P,
        cwd: bool,
        ignore_case: bool,
        sort: Sort,
    ) -> Self {
        let mut builder = Self::new_initial(&format!(
            "SELECT
                id,
                directory,
                name,
                hash,
                mime,
                mtime,
                ctime,
                mode,
                inode,
                links,
                uid,
                gid,
                size,
                is_dir
                {}
            FROM file
            WHERE",
            super::file::e2p_feature_comma()
        ));

        builder.file_handle_branch(expr.parsed(), ignore_case);
        // build_path_clause
        // build_sort

        builder
    }

    /// Handle query branch statements by appending the corresponding SQL
    pub(crate) fn file_handle_branch(&mut self, expr: &Expr, ignore_case: bool) {
        match expr {
            Expr::Pattern(ref search) => match search.inner_t() {
                SearchKind::Exact => self.build_tag_pattern_branch(search, ignore_case),
                SearchKind::Regex => println!("Regex query: {:#?}", expr.clone()),
                SearchKind::Glob => println!("Glob query: {:#?}", expr.clone()),
            },
            _ => println!("other"),
        }
    }

    /// Handle a comparison expression for a file query
    pub(crate) fn build_comparison_branch(&mut self, cmp: Expr, ignore_case: bool) {
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
            // value
            // self.append_param();
            self.appendln("))");
        }
    }

    /// Handle a [`Search`] pattern for files
    pub(crate) fn build_tag_pattern_branch(&mut self, patt: &Search, ignore_case: bool) {
        let case = Self::return_nocase_collation(ignore_case);

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
    }

    /// Handle a [`UnaryExpr`]. The only operator is `not`
    pub(crate) fn build_not_branch(&mut self, expr: UnaryExpr<UnaryOp>, ignore_case: bool) {
        self.append(" NOT ");

        let UnaryExpr { operand, .. } = expr;
        self.file_handle_branch(&*operand, ignore_case);
    }

    /// Handle a [`BinaryExpr`] with a [`LogicalOp`]
    pub(crate) fn build_and_branch(&mut self, expr: BinaryExpr<LogicalOp>, ignore_case: bool) {
        let BinaryExpr { operator, lhs, rhs } = expr;

        if operator == LogicalOp::And {
            self.file_handle_branch(&*lhs, ignore_case);
            self.append(" AND ");
            self.file_handle_branch(&*rhs, ignore_case);
        }

        if operator == LogicalOp::Or {
            self.append("(");
            self.file_handle_branch(&*lhs, ignore_case);
            self.append(" OR ");
            self.file_handle_branch(&*rhs, ignore_case);
            self.append(")");
        }
    }

    /// Append a sort to the end of the SQL query given a [`Sort`] variant
    pub(crate) fn build_sort(&mut self, sort: Option<Sort>) {
        // self.appendln(sort.unwrap_or(Sort::None).to_string());
        if let Some(s) = sort {
            self.appendln(s.to_string());
        }
    }
}

impl fmt::Debug for SqlBuilder<'_> {
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
            .field(
                "named_params",
                &self.named_params.iter().fold(String::new(), |mut acc, f| {
                    acc.push_str(&format!(" {} {:?}", f.0, f.1.to_sql()));
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
            Sort::Id => f.write_str(" ORDER BY id"),
            Sort::Name => f.write_str(" ORDER BY directory || '/' || name"),
            Sort::ModificationTime => f.write_str(" ORDER BY mtime, directory || '/' || name"),
            Sort::CreationTime => f.write_str(" ORDER BY ctime, directory || '/' || name"),
            Sort::FileSize => f.write_str(" ORDER BY size, directory || '/' || name"),
            Sort::None => f.write_str(""),
        }
    }
}
