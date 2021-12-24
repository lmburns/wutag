//! `Ciphertext` and `Plaintext` types dealing with the encrypted and
//! unencrypted state of the tag registry

use anyhow::Result;
use secstr::SecVec;
use zeroize::Zeroize;

/// Wrapper around the ciphertext bytes to prevent any data leaks
pub(crate) struct Ciphertext(SecVec<u8>);

impl Ciphertext {
    /// Create an empty `Ciphertext`
    #[allow(dead_code)]
    pub(crate) fn empty() -> Self {
        Self(SecVec::new(vec![]))
    }

    /// Reference to the data within the struct. Not meant to be cloned
    /// directly; instead, clone the `Ciphertext` struct
    pub(crate) fn unsecure_ref(&self) -> &[u8] {
        self.0.unsecure()
    }
}

impl From<Vec<u8>> for Ciphertext {
    fn from(mut other: Vec<u8>) -> Self {
        let into = Self(other.clone().into());
        other.zeroize();
        into
    }
}

/// Wrapper around the plaintext bytes to prevent any data leaks
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct Plaintext(SecVec<u8>);

impl Plaintext {
    /// Create an empty `Plaintext`
    #[allow(dead_code)]
    pub(crate) fn empty() -> Self {
        Self(SecVec::new(vec![]))
    }

    /// Reference to the data within the struct. Not meant to be cloned
    /// directly; instead, clone the `Plaintext` struct
    pub(crate) fn unsecure_ref(&self) -> &[u8] {
        self.0.unsecure()
    }

    /// Get an unsecure reference to the UTF-8 data within the once encrypted
    /// bytes. Not meant to be cloned directly; instead, clone the `Plaintext`
    /// struct
    #[allow(dead_code)]
    pub(crate) fn unsecure_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.unsecure_ref())
    }
}

impl From<Vec<u8>> for Plaintext {
    fn from(mut other: Vec<u8>) -> Self {
        // Explicit zeroing of unsecure buffer required
        let into = Self(other.clone().into());
        other.zeroize();
        into
    }
}

impl From<String> for Plaintext {
    fn from(mut other: String) -> Self {
        // Explicit zeroing of unsecure buffer required
        let into = Self(other.as_bytes().into());
        other.zeroize();
        into
    }
}

impl From<&str> for Plaintext {
    fn from(s: &str) -> Self {
        Self(s.as_bytes().into())
    }
}
