//! Optional encryption interface the `wutag` crate

pub(crate) mod backend;
pub(crate) mod protocol;
pub(crate) mod recipients;
pub(crate) mod types;
pub(crate) mod util;

use crate::consts::encrypt::REGISTRY_UMASK;
pub(crate) use recipients::Recipients;
pub(crate) use types::{Ciphertext, Plaintext};

use anyhow::Result;
use std::{fmt, fs, io::Write, os::unix::fs::OpenOptionsExt, path::Path};
use thiserror::Error;

/// Crypto protocol.
///
/// This list contains all protocols supported by wutag at the moment
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum Proto {
    /// GPG crypto
    Gpg,
}

impl Proto {
    /// Get the protocol display name
    pub(crate) const fn name(&self) -> &str {
        match self {
            Self::Gpg => "GPG",
        }
    }
}

/// Cryptography configuration
///
/// Allows configuration of protocol type
pub(crate) struct EncryptConfig {
    /// Protocol used
    pub(crate) proto: Proto,

    /// Whether to use TTY or pinentry
    pub(crate) gpg_tty: bool,
}

impl EncryptConfig {
    /// Construct config with given protocol
    pub(crate) const fn from(proto: Proto) -> Self {
        Self {
            proto,
            gpg_tty: false,
        }
    }
}

/// Represents a key
///
/// The key type may be any of the supported crypto proto types
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub(crate) enum Key {
    /// A GPG key
    // #[cfg(feature = "_encrypt-gpg")]
    Gpg(protocol::gpg::Key),
}

impl Key {
    /// Get type of protocol for `Key`
    pub(crate) const fn proto(&self) -> Proto {
        match self {
            // #[cfg(feature = "_encrypt-gpg")]
            Key::Gpg(_) => Proto::Gpg,
        }
    }

    /// Key's fingerprint
    pub(crate) fn fingerprint(&self, short: bool) -> String {
        match self {
            // #[cfg(feature = "_encrypt-gpg")]
            Key::Gpg(key) => key.fingerprint(short),
        }
    }

    /// Display string for user data
    pub(crate) fn display(&self) -> String {
        match self {
            // #[cfg(feature = "_encrypt-gpg")]
            Key::Gpg(key) => key.display_user(),
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} - {}",
            self.proto().name(),
            self.fingerprint(true),
            self.display(),
        )
    }
}

/// Get cryptography context for given proto type at runtime
///
/// This selects a compatible crypto context at runtime
///
/// # Errors
///
/// Errors if no compatible crypto context is available for the selected
/// protocol because no backend is providing it. Also errors if creating the
/// context fails.
#[allow(unreachable_code)]
pub(crate) fn context(config: &EncryptConfig) -> Result<Context, Error> {
    // Select proper backend
    match config.proto {
        Proto::Gpg => {
            #[cfg(feature = "encrypt-gpgme")]
            return Ok(Context::from(Box::new(
                backend::gpgme::context::context(config).map_err(|e| Error::Context(e.into()))?,
            )));
        },
    }

    Err(Error::Unsupported(config.proto))
}

/// Generic context.
pub(crate) struct Context {
    /// Inner context that implements the [`InnerCtx`] trait
    context: Box<dyn InnerCtx>,
}

impl Context {
    /// Convert a type implementing [`InnerCtx`] to the [`Context`] wrapper
    pub(crate) fn from(context: Box<dyn InnerCtx>) -> Self {
        Self { context }
    }
}

impl InnerCtx for Context {
    fn encrypt(&mut self, recipients: &Recipients, plaintext: Plaintext) -> Result<Ciphertext> {
        self.context.encrypt(recipients, plaintext)
    }

    fn decrypt(&mut self, ciphertext: Ciphertext) -> Result<Plaintext> {
        self.context.decrypt(ciphertext)
    }

    fn can_decrypt(&mut self, ciphertext: Ciphertext) -> Result<bool> {
        self.context.can_decrypt(ciphertext)
    }

    fn keys_public(&mut self) -> Result<Vec<Key>> {
        self.context.keys_public()
    }

    fn keys_private(&mut self) -> Result<Vec<Key>> {
        self.context.keys_private()
    }

    /// Return a vector of user emails
    fn user_emails(&mut self) -> Result<Vec<String>> {
        self.context.user_emails()
    }

    fn supports_proto(&self, proto: Proto) -> bool {
        self.context.supports_proto(proto)
    }
}

/// Defines generic crypto context. Meant to be used with age and gpg encryption
///
/// Implemented on backend specific cryptography contexts, makes using it
/// possible through a single simple interface.
pub(crate) trait InnerCtx {
    /// Encrypt [`Plaintext`](types::Plaintext) using the
    /// [`Recipients`](recipients::Recipients)
    fn encrypt(&mut self, recipients: &Recipients, plaintext: Plaintext) -> Result<Ciphertext>;

    /// Encrypt plaintext and write it to the file.
    fn encrypt_file(
        &mut self,
        recipients: &Recipients,
        plaintext: Plaintext,
        path: &Path,
    ) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .mode(0o666 - (0o666 & *REGISTRY_UMASK))
            .write(true)
            .create(true)
            .open(&path)?;

        file.write_all(self.encrypt(recipients, plaintext)?.unsecure_ref())
            .map_err(|e| Error::WriteFile(e).into())
    }

    /// Decrypt a [`Ciphertext`](types::Ciphertext)
    fn decrypt(&mut self, ciphertext: Ciphertext) -> Result<Plaintext>;

    /// Decrypt ciphertext from file.
    fn decrypt_file(&mut self, path: &Path) -> Result<Plaintext> {
        self.decrypt(fs::read(path).map_err(Error::ReadFile)?.into())
    }

    /// Determine whether the user possesses the correct key to decrypt the
    /// [`Ciphertext`](types::Ciphertext)
    fn can_decrypt(&mut self, ciphertext: Ciphertext) -> Result<bool>;

    /// Check whether we can decrypt ciphertext from fil.
    fn can_decrypt_file(&mut self, path: &Path) -> Result<bool> {
        self.can_decrypt(fs::read(path).map_err(Error::ReadFile)?.into())
    }

    /// Return a vector of public [`Key`](self::Key)'s
    fn keys_public(&mut self) -> Result<Vec<Key>>;

    /// Return a vector of private [`Key`](self::Key)'s
    fn keys_private(&mut self) -> Result<Vec<Key>>;

    /// Obtain user emails
    fn user_emails(&mut self) -> Result<Vec<String>>;

    /// Obtain a public key from keychain for fingerprint
    fn get_public_key(&mut self, fingerprint: &str) -> Result<Key> {
        self.keys_public()?
            .into_iter()
            .find(|key| util::fingerprints_equal(key.fingerprint(false), fingerprint))
            .ok_or_else(|| Error::UnknownFingerprint.into())
    }

    /// Find public keys from keychain for fingerprints.
    ///
    /// Skips fingerprints no key is found for.
    fn find_public_keys(&mut self, fingerprints: &[&str]) -> Result<Vec<Key>> {
        let keys = self.keys_public()?;
        Ok(fingerprints
            .iter()
            .filter_map(|fingerprint| {
                keys.iter()
                    .find(|key| util::fingerprints_equal(key.fingerprint(false), fingerprint))
                    .cloned()
            })
            .collect())
    }

    /// Determine whether the context supports the correct protocol
    fn supports_proto(&self, proto: Proto) -> bool;
}

/// General cryptography error
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// Unable to obtain `GPGME` context
    #[error("failed to obtain GPGME cryptography context")]
    Context(#[source] anyhow::Error),

    /// Unsupported `GPGME` protocol
    #[error("failed to build context, protocol not supported: {:?}", _0)]
    Unsupported(Proto),

    /// Unable to write to a file
    #[error("failed to write to file")]
    WriteFile(#[source] std::io::Error),

    /// Unable to read a file
    #[error("failed to read from file")]
    ReadFile(#[source] std::io::Error),

    /// Fingerprint is unknown and not in the keychain
    #[error("fingerprint does not match public key in keychain")]
    #[allow(dead_code)]
    UnknownFingerprint,
}

/// Prelude for common crypto traits.
pub(crate) mod prelude {
    pub(crate) use super::InnerCtx;
}
