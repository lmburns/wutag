//! Raw interface with the `GPGME` library

use crate::{
    encryption::types::{Ciphertext, Plaintext},
    wutag_fatal,
};
use anyhow::Result;
use colored::Colorize;
use gpgme::{Context, EncryptFlags, Key};
use thiserror::Error;
use zeroize::Zeroize;

/// GPGME encryption flags
const ENCRYPT_FLAGS: EncryptFlags = EncryptFlags::ALWAYS_TRUST;

/// Encrypt the [`Plaintext`](crate::encryption::Plaintext) to
/// [`Ciphertext`](crate::encryption::Ciphertext), only used with the
/// [`wutag`](crate) [`Registry`](crate::registry::Registry)
///
/// - `context`: GPGME context
/// - `recipients`: recipients whose fingerprint will be used to encrypt with
/// - `plaintext`: plaintext to be encrypted
///
/// # Panics
/// Will panic and display an error message if there are no recipients passed to
/// the function
pub(crate) fn encrypt(
    context: &mut Context,
    recipients: &[&str],
    plaintext: &Plaintext,
) -> Result<Ciphertext> {
    if recipients.is_empty() {
        wutag_fatal!("recipients must not be empty");
    }

    let mut ciphertext = vec![];
    let keys = fingerprints_to_keys(context, recipients)?;
    context
        .encrypt_with_flags(
            keys.iter(),
            plaintext.unsecure_ref(),
            &mut ciphertext,
            ENCRYPT_FLAGS,
        )
        .map_err(Error::Encrypt)?;

    Ok(Ciphertext::from(ciphertext))
}

/// Decrypt [`Ciphertext`]
///
/// - `context`: GPGME context
/// - `ciphertext`: ciphertext to be decrypted
pub(crate) fn decrypt(context: &mut Context, ciphertext: &Ciphertext) -> Result<Plaintext> {
    let mut plaintext = vec![];
    context
        .decrypt(ciphertext.unsecure_ref(), &mut plaintext)
        .map_err(Error::Decrypt)?;

    Ok(Plaintext::from(plaintext))
}

/// Check whether the ciphertext can be decrypted with the specified
/// fingerprint.
///
/// This checks whether whether we own the secret key to decrypt the given
/// ciphertext. Assumes `true` if GPGME returns an error different than
/// `NO_SECKEY`.
///
/// - `context`: GPGME context
/// - `ciphertext`: ciphertext to check
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn can_decrypt(context: &mut Context, ciphertext: &Ciphertext) -> Result<bool> {
    // Try to decrypt, explicit zeroing of unsecure buffer required
    let mut plaintext = vec![];
    let result = context.decrypt(ciphertext.unsecure_ref(), &mut plaintext);
    plaintext.zeroize();

    match result {
        Err(err) if gpgme::error::Error::NO_SECKEY.code() == err.code() => Ok(false),
        _ => Ok(true),
    }
}

/// Get all public keys from keychain.
///
/// - `context`: GPGME context
pub(crate) fn public_keys(context: &mut Context) -> Result<Vec<KeyId>> {
    Ok(context
        .keys()?
        .into_iter()
        .filter_map(Result::ok)
        .filter(Key::can_encrypt)
        .map(Into::into)
        .collect())
}

/// Get all private/secret keys from keychain.
///
/// - `context`: GPGME context
pub(crate) fn private_keys(context: &mut Context) -> Result<Vec<KeyId>> {
    Ok(context
        .secret_keys()?
        .into_iter()
        .filter_map(Result::ok)
        .filter(Key::can_encrypt)
        .map(Into::into)
        .collect())
}

/// Access emails within the keychain
pub(crate) fn user_emails(context: &mut Context) -> Result<Vec<String>> {
    let mut emails = vec![];
    context
        .secret_keys()?
        .into_iter()
        .filter_map(Result::ok)
        .filter(Key::can_encrypt)
        .map(|key| {
            key.user_ids()
                .map(|k| {
                    if let Ok(email) = k.email() {
                        if !email.trim().is_empty() {
                            emails.push(email.to_owned());
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .for_each(drop);

    Ok(emails)
}

/// A key identifier with a fingerprint and user IDs.
#[derive(Clone)]
pub(crate) struct KeyId(pub(crate) String, pub(crate) Vec<String>);

impl From<Key> for KeyId {
    fn from(key: Key) -> Self {
        Self(
            key.fingerprint()
                .expect("GPGME key does not have fingerprint")
                .to_string(),
            key.user_ids()
                .map(|user| {
                    let mut parts = vec![];
                    if let Ok(name) = user.name() {
                        if !name.trim().is_empty() {
                            parts.push(name.into());
                        }
                    }
                    if let Ok(comment) = user.comment() {
                        if !comment.trim().is_empty() {
                            parts.push(format!("({})", comment));
                        }
                    }
                    if let Ok(email) = user.email() {
                        if !email.trim().is_empty() {
                            parts.push(format!("<{}>", email));
                        }
                    }
                    parts.join(" ")
                })
                .collect(),
        )
    }
}

/// Transform fingerprints into GPGME keys.
///
/// Errors if a fingerprint does not match a public key.
fn fingerprints_to_keys(context: &mut Context, fingerprints: &[&str]) -> Result<Vec<Key>> {
    let mut keys = vec![];
    for fp in fingerprints {
        keys.push(
            context
                .get_key(fp.to_owned())
                .map_err(Error::UnknownFingerprint)?,
        );
    }
    Ok(keys)
}

/// [`GPGME`] library error
#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("failed to encrypt plaintext")]
    Encrypt(#[source] gpgme::Error),

    #[error("failed to decrypt ciphertext")]
    Decrypt(#[source] gpgme::Error),

    #[error("fingerprint does not match public key in keychain")]
    UnknownFingerprint(#[source] gpgme::Error),
}
