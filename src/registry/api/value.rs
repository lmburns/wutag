//! Interactions with the [`Value`] object

use super::super::{
    types::{
        file::FileId,
        tag::TagId,
        value::{Value, ValueId, Values},
    },
    Registry,
};
use anyhow::{Context, Result};

impl Registry {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the number of [`Value`]s in the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn value_count(&self) -> Result<u32> {
        self.txn_wrap(|txn| txn.value_count())
    }

    /// Retrieve the number of [`Tag`]s a given [`Value`] is associated with
    pub(crate) fn value_count_by_id(&self, id: ValueId) -> Result<u32> {
        self.txn_wrap(|txn| txn.value_count_by_id(id))
    }

    /// Retrieve all [`Value`]s in the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn values(&self) -> Result<Values> {
        self.txn_wrap(|txn| txn.values())
    }

    /// Retrieve the [`Value`] matching the [`ValueId`] in the database
    pub(crate) fn value(&self, id: ValueId) -> Result<Value> {
        self.txn_wrap(|txn| txn.value(id))
    }

    /// Retrieve all [`Values`] matching the vector of [`ValueId`]s
    pub(crate) fn values_by_valueids(&self, ids: &[ValueId]) -> Result<Values> {
        self.txn_wrap(|txn| txn.values_by_valueids(ids.to_vec()).map_err(Into::into))
    }

    /// Retrieve all unused [`Value`]s within the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn values_unused(&self) -> Result<Values> {
        self.txn_wrap(|txn| txn.values_unused())
    }

    /// Retrieve a [`Value`] by its string name
    ///   - **Exact match** searching
    pub(crate) fn value_by_name<S: AsRef<str>>(&self, name: S, ignore_case: bool) -> Result<Value> {
        self.txn_wrap(|txn| txn.value_by_name(name, ignore_case))
    }

    /// Retrieve all [`Value`]s matching a vector of names
    ///   - **Exact match** searching
    pub(crate) fn values_by_names<S: AsRef<str>>(
        &self,
        names: &[S],
        ignore_case: bool,
    ) -> Result<Values> {
        self.txn_wrap(|txn| txn.values_by_names(names, ignore_case).map_err(Into::into))
    }

    /// Retrieve all [`Value`]s matching a `TagId`
    pub(crate) fn values_by_tagid(&self, tid: TagId) -> Result<Values> {
        self.txn_wrap(|txn| txn.values_by_tagid(tid))
    }

    // ============================== Pattern =============================
    // ====================================================================

    /// Query for [`Values`] using the `pcre` regex custom function
    pub(crate) fn values_by_pcre(&self, reg: &str) -> Result<Values> {
        self.txn_wrap(|txn| txn.select_values_by_pcre(reg))
    }

    /// Query for [`Values`] using the `regex` custom function
    pub(crate) fn values_by_regex(&self, reg: &str) -> Result<Values> {
        self.txn_wrap(|txn| txn.select_values_by_regex(reg))
    }

    /// Query for [`Values`] using the `regex` custom function
    pub(crate) fn values_by_iregex(&self, reg: &str) -> Result<Values> {
        self.txn_wrap(|txn| txn.select_values_by_iregex(reg))
    }

    /// Query for [`Values`] using the `glob` custom function
    pub(crate) fn values_by_glob(&self, glob: &str) -> Result<Values> {
        self.txn_wrap(|txn| txn.select_values_by_glob(glob))
    }

    /// Query for [`Values`] using the `iglob` custom function
    pub(crate) fn values_by_iglob(&self, glob: &str) -> Result<Values> {
        self.txn_wrap(|txn| txn.select_values_by_iglob(glob))
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a [`Value`] into the database
    pub(crate) fn insert_value<S: AsRef<str>>(&self, name: S) -> Result<Value> {
        Value::validate_name(&name)?;
        self.wrap_commit(|txn| txn.insert_value(name))
    }

    /// Update the [`Value`] by changing its' name
    pub(crate) fn update_value<S: AsRef<str>>(&self, id: ValueId, name: S) -> Result<Value> {
        Value::validate_name(&name)?;
        self.wrap_commit(|txn| txn.update_value(id, name).map_err(Into::into))
    }

    /// Remove a [`Value`] from the database
    pub(crate) fn delete_value(&self, id: ValueId) -> Result<()> {
        self.wrap_commit(|txn| {
            txn.delete_filetag_by_valueid(id)?;
            txn.delete_implication_by_valueid(id)?;
            txn.delete_value(id).map_err(Into::into)
        })
    }
}
