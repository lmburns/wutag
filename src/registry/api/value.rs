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
    pub(crate) fn value_count(&self) -> Result<u32> {
        let txn = self.txn()?;
        txn.value_count()
    }

    /// Retrieve all [`Value`]s in the database
    pub(crate) fn values(&self) -> Result<Values> {
        let txn = self.txn()?;
        txn.values()
    }

    /// Retrieve the [`Value`] matching the [`ValueId`] in the database
    pub(crate) fn value(&self, id: ValueId) -> Result<Value> {
        let txn = self.txn()?;
        txn.value(id)
    }

    /// Retrieve all `Value`s matching the vector of `ValueId`s
    pub(crate) fn values_by_valueids(&self, ids: Vec<ValueId>) -> Result<Values> {
        let txn = self.txn()?;
        txn.values_by_valueids(ids).map_err(Into::into)
    }

    /// Retrieve all unused [`Value`]s within the database
    pub(crate) fn values_unused(&self) -> Result<Values> {
        let txn = self.txn()?;
        txn.values_unused()
    }

    /// Retrieve a [`Value`] by its string name
    ///   - **Exact match** searching
    pub(crate) fn value_by_name<S: AsRef<str>>(&self, name: S, ignore_case: bool) -> Result<Value> {
        let txn = self.txn()?;
        txn.value_by_name(name, ignore_case)
    }

    /// Retrieve all [`Value`]s matching a vector of names
    ///   - **Exact match** searching
    pub(crate) fn values_by_names<S: AsRef<str>>(
        &self,
        names: &[S],
        ignore_case: bool,
    ) -> Result<Values> {
        let txn = self.txn()?;
        txn.values_by_names(names, ignore_case).map_err(Into::into)
    }

    /// Retrieve all [`Value`]s matching a `TagId`
    pub(crate) fn values_by_tagid(&self, tid: TagId) -> Result<Values> {
        let txn = self.txn()?;
        txn.values_by_tagid(tid)
    }

    // ============================== Pattern =============================
    // ====================================================================

    /// Query for [`Values`] using a the `regex` custom function
    pub(crate) fn select_values_by_regex(&self, reg: &str) -> Result<Values> {
        self.txn()?.select_values_by_regex(reg)
    }

    /// Query for [`Values`] using a the `regex` custom function
    pub(crate) fn select_values_by_iregex(&self, reg: &str) -> Result<Values> {
        self.txn()?.select_values_by_iregex(reg)
    }

    /// Query for [`Values`] using a the `glob` custom function
    pub(crate) fn select_values_by_glob(&self, glob: &str) -> Result<Values> {
        self.txn()?.select_values_by_glob(glob)
    }

    /// Query for [`Values`] using a the `iglob` custom function
    pub(crate) fn select_values_by_iglob(&self, glob: &str) -> Result<Values> {
        self.txn()?.select_values_by_iglob(glob)
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Insert a [`Value`] into the database
    pub(crate) fn insert_value<S: AsRef<str>>(&self, name: S) -> Result<Value> {
        Value::validate_name(&name)?;
        let txn = self.txn()?;
        txn.insert_value(name)
    }

    /// Update the [`Value`] by changing its' name
    pub(crate) fn update_value<S: AsRef<str>>(&self, id: ValueId, name: S) -> Result<Value> {
        Value::validate_name(&name)?;
        let txn = self.txn()?;
        txn.update_value(id, name).map_err(Into::into)
    }

    /// Remove a [`Value`] from the database
    pub(crate) fn delete_value<S: AsRef<str>>(&self, id: ValueId) -> Result<()> {
        let txn = self.txn()?;
        txn.delete_filetag_by_valueid(id)?;
        txn.delete_implication_by_valueid(id)?;
        txn.delete_value(id).map_err(Into::into)
    }
}
