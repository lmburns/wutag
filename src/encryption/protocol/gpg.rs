//! Crypto GPG protocol

use crate::encryption;

/// Represents a *GPG* key
#[derive(Debug, Clone)]
pub(crate) struct Key {
    /// Full fingerprint of user
    pub(crate) fingerprint: String,

    /// Displayable user ID strings. Includes name and email as well
    pub(crate) user_ids: Vec<String>,
}

impl Key {
    /// Fingerprint of the `Key`
    pub(crate) fn fingerprint(&self, short: bool) -> String {
        if short {
            // C011CBEF6628B679
            &self.fingerprint[self.fingerprint.len() - 16..]
        } else {
            // E93ACCAAAEB024788C106EDEC011CBEF6628B679
            &self.fingerprint
        }
        .trim()
        .to_uppercase()
    }

    /// Displayable user data of the `Key`
    pub(crate) fn display_user(&self) -> String {
        self.user_ids.join("; ")
    }

    /// Transform into generic key
    #[allow(dead_code)]
    pub(crate) fn into_key(self) -> encryption::Key {
        encryption::Key::Gpg(self)
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.fingerprint.trim().to_uppercase() == other.fingerprint.trim().to_uppercase()
    }
}
