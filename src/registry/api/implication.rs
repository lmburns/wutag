//! Interactions with the [`Implication`] object

use super::super::{
    transaction::Txn,
    types::{
        file::FileId,
        implication::{Implication, Implications},
        tag::{TagId, TagValueCombo, TagValueCombos},
        value::ValueId,
    },
    Registry,
};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;

impl Registry {
    // ============================ Retrieving ============================
    // ====================================================================

    /// Retrieve the [`Implications`] within the database
    #[allow(clippy::redundant_closure_for_method_calls)] // Doesn't work
    pub(crate) fn implications(&self) -> Result<Implications> {
        self.txn_wrap(|txn| txn.implications())
    }

    // TODO: Needs testing
    /// Retrieve the [`Implications`] matching the given [`TagValueCombo`]
    ///
    /// Note: This function requires that [`Txn`] be passed by value to prevent
    /// multiple transactions from being open in the database. Nearly all other
    /// functions in this program rely on one transaction and therefore do not
    /// require a pass by value.
    ///
    /// A possible future refactor of the code could clean this up
    #[allow(clippy::unused_self)]
    pub(crate) fn implications_for(
        &self,
        txn: &Txn,
        tvc: &[TagValueCombo],
    ) -> Result<Implications> {
        let mut res_implications = Implications::new(vec![]);
        let mut implied_pairs = TagValueCombos::new(tvc.to_vec());

        for tv in tvc.iter() {
            let implications = txn.implications_for(implied_pairs.inner())?;
            // implied_pairs = TagValueCombos::new(vec![]);
            implied_pairs.clear();

            for implication in implications.iter() {
                if !res_implications.contains(implication) {
                    res_implications.push(implication.clone());
                    implied_pairs.push(TagValueCombo::new(
                        implication.implied_tag().id(),
                        implication.implied_val().id(),
                    ));
                }
            }
        }

        Ok(res_implications)
    }

    /// Retrieve the [`Implications`] that imply the given [`TagValueCombo`]
    pub(crate) fn implications_implying(&self, tvc: &[TagValueCombo]) -> Result<Implications> {
        self.txn_wrap(|txn| {
            let mut res_implications = Implications::new(vec![]);
            let mut implying_pairs = TagValueCombos::new(tvc.to_vec());

            for tv in tvc.iter() {
                let implications = txn.implications_implying(implying_pairs.inner())?;
                // implying_pairs = TagValueCombos::new(vec![]);
                implying_pairs.clear();

                for implication in implications.iter() {
                    if res_implications.contains(implication) {
                        res_implications.push(implication.clone());
                        implying_pairs.push(TagValueCombo::new(
                            implication.implying_tag().id(),
                            implication.implying_val().id(),
                        ));
                    }
                }
            }

            Ok(res_implications)
        })
    }

    // ============================= Modifying ============================
    // ====================================================================

    /// Add the [`Implication`] to the database
    pub(crate) fn insert_implication(
        &self,
        txn: &Txn,
        pair: &TagValueCombo,
        implied_pair: &TagValueCombo,
    ) -> Result<()> {
        let implications = self
            .implications_for(txn, &[implied_pair.clone()])
            .context(format!(
                "failed to get implications for {}",
                &implied_pair.to_string()
            ))?;

        #[allow(clippy::blocks_in_if_conditions)]
        if implications.any(|im| {
            im.implied_tag().id() == pair.tag_id()
                && (pair.value_id() == ValueId::null() || im.implied_val().id() == pair.value_id())
        }) {
            return Err(anyhow!("this implication would create a loop"));
        }

        txn.insert_implication(pair, implied_pair)
    }

    /// Remove the [`Implication`] from the database
    pub(crate) fn delete_implication(
        &self,
        pair: &TagValueCombo,
        implied_pair: &TagValueCombo,
    ) -> Result<()> {
        self.wrap_commit(|txn| {
            txn.delete_implication(pair, implied_pair)?;
            Ok(())
        })
    }

    /// Remove the [`Implication`] matching [`TagId`]
    pub(crate) fn delete_implication_by_tagid(&self, tx: &Txn, id: TagId) -> Result<()> {
        self.wrap_commit_by(tx, |txn| txn.delete_implication_by_tagid(id))
    }

    /// Remove the [`Implication`] matching [`ValueId`]
    pub(crate) fn delete_implication_by_valueid(&self, id: ValueId) -> Result<()> {
        self.wrap_commit(|txn| txn.delete_implication_by_valueid(id))
    }
}
