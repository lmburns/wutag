//! Provides interface for crypto recipients.

use super::Key;
use crate::wutag_fatal;

/// A list of recipients
///
/// In the future there may be support for age encryption
#[derive(Clone, PartialEq)]
pub(crate) struct Recipients {
    /// Inner vector of `Key`s
    keys: Vec<Key>,
}

impl Recipients {
    /// Construct recipients set from list of keys
    pub(crate) fn from(keys: Vec<Key>) -> Self {
        if !keys_same_proto(&keys) {
            wutag_fatal!("recipient keys must use same protocol");
        }

        Self { keys }
    }

    /// Get recipient keys.
    pub(crate) fn keys(&self) -> &[Key] {
        &self.keys
    }
}

/// Check if given keys all use same protocol
///
/// Succeeds if no key is given
fn keys_same_proto(keys: &[Key]) -> bool {
    if keys.len() < 2 {
        true
    } else {
        let proto = keys[0].proto();
        keys[1..].iter().all(|k| k.proto() == proto)
    }
}
