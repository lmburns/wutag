//! Provides GPGME binary context adapter.

use anyhow::Result;
use gpgme::{Context as GpgmeContext, PinentryMode, Protocol};
use thiserror::Error;

use super::raw;
use crate::encryption::{
    protocol, util, Ciphertext, EncryptConfig, InnerCtx, Key, Plaintext, Proto, Recipients,
};
use std::env;

/// Protocol to use for Gpg/Pgp
const PROTO: Protocol = Protocol::OpenPgp;

/// Create GPGME crypto context.
pub(crate) fn context(config: &EncryptConfig) -> Result<Context, Error> {
    // Set environment when using GPG TTY
    if config.gpg_tty && !util::has_gpg_tty() {
        if let Some(tty) = util::get_tty() {
            env::set_var("GPG_TTY", tty);
        }
    }

    let mut context = gpgme::Context::from_protocol(PROTO).map_err(Error::Context)?;

    // Set pinentry mode when using GPG TTY
    if config.gpg_tty {
        context
            .set_pinentry_mode(PinentryMode::Loopback)
            .map_err(Error::Context)?;
    }

    context.set_armor(true);

    Ok(Context::from(context))
}

/// GPGME crypto context.
pub(crate) struct Context {
    /// GPGME cryptography context.
    context: GpgmeContext,
}

impl Context {
    /// Convert a `GpgmeContext` to a `Context`
    pub(crate) fn from(context: GpgmeContext) -> Self {
        Self { context }
    }
}

impl InnerCtx for Context {
    fn encrypt(&mut self, recipients: &Recipients, plaintext: Plaintext) -> Result<Ciphertext> {
        let fingerprints: Vec<String> = recipients
            .keys()
            .iter()
            .map(|key| key.fingerprint(false))
            .collect();

        let fingerprints = fingerprints.iter().map(String::as_str).collect::<Vec<_>>();
        raw::encrypt(&mut self.context, &fingerprints, &plaintext)
    }

    fn decrypt(&mut self, ciphertext: Ciphertext) -> Result<Plaintext> {
        raw::decrypt(&mut self.context, &ciphertext)
    }

    fn can_decrypt(&mut self, ciphertext: Ciphertext) -> Result<bool> {
        raw::can_decrypt(&mut self.context, &ciphertext)
    }

    fn keys_public(&mut self) -> Result<Vec<Key>> {
        Ok(raw::public_keys(&mut self.context)?
            .into_iter()
            .map(|key| {
                Key::Gpg(protocol::gpg::Key {
                    fingerprint: key.0,
                    user_ids:    key.1,
                })
            })
            .collect())
    }

    fn keys_private(&mut self) -> Result<Vec<Key>> {
        Ok(raw::private_keys(&mut self.context)?
            .into_iter()
            .map(|key| {
                Key::Gpg(protocol::gpg::Key {
                    fingerprint: key.0,
                    user_ids:    key.1,
                })
            })
            .collect())
    }

    fn user_emails(&mut self) -> Result<Vec<String>> {
        raw::user_emails(&mut self.context)
    }

    fn supports_proto(&self, proto: Proto) -> bool {
        proto == Proto::Gpg
    }
}

/// GPGME context error
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// Unable to obtain `GPGME` context
    #[error("failed to obtain GPGME cryptography context")]
    Context(#[source] gpgme::Error),
}
