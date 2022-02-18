//! Common cryptography utilities

use super::{prelude::*, Context, EncryptConfig, Key, Proto};
use anyhow::Result;
use std::{
    env,
    path::{Path, PathBuf},
};

/// Local TTY path.
const LOCAL_TTY_PATH: &str = "/dev/stdin";
/// Max depth to traverse symlinks
const SYMLINK_MAX_DEPTH: u8 = 31;

/// Format fingerprint in consistent format
#[allow(dead_code)]
pub(crate) fn format_fingerprint<S: AsRef<str>>(fingerprint: S) -> String {
    fingerprint.as_ref().trim().to_uppercase()
}

/// Check whether two fingerprints match
#[allow(dead_code)]
pub(crate) fn fingerprints_equal<S: AsRef<str>, T: AsRef<str>>(a: S, b: T) -> bool {
    !format_fingerprint(&a).is_empty() && format_fingerprint(a) == format_fingerprint(b)
}

/// Check whether a list of keys contains the given fingerprint
#[allow(dead_code)]
pub(crate) fn keys_contain_fingerprint<S: AsRef<str>>(keys: &[Key], fingerprint: S) -> bool {
    keys.iter()
        .any(|key| fingerprints_equal(key.fingerprint(false), fingerprint.as_ref()))
}

/// Check whether the user has any private/secret key in their keychain
#[allow(dead_code)]
pub(crate) fn has_private_key(config: &EncryptConfig) -> Result<bool> {
    Ok(!super::context(config)?.keys_private()?.is_empty())
}

/// Check whether `GPG_TTY` is set
pub(crate) fn has_gpg_tty() -> bool {
    env::var_os("GPG_TTY").map_or(false, |v| !v.is_empty())
}

/// Get `TTY` path for this process.
///
/// Returns `None` if not in a `TTY`. Always returns `None` if not Linux,
/// FreeBSD or OpenBSD.
pub(crate) fn get_tty() -> Option<PathBuf> {
    /// Resolve a symblink but do not traverse deeper than `SYMLINK_MAX_DEPTH`
    fn resolve_symlink(path: &Path, depth: u8) -> Option<PathBuf> {
        assert!(
            (depth < SYMLINK_MAX_DEPTH),
            "failed to resolve symlink because it is too deep, possible loop?"
        );

        // Read symlink path, recursively find target
        match path.read_link() {
            Ok(path) => resolve_symlink(&path, depth + 1),
            Err(_) if depth == 0 => None,
            Err(_) => Some(path.into()),
        }
    }

    // Unsupported platforms
    if cfg!(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
    ))) {
        return None;
    }

    let path = PathBuf::from(LOCAL_TTY_PATH);

    resolve_symlink(&path, 0)
}

/// Construct crypto config, respect CLI arguments.
pub(crate) const fn config(tty: bool) -> EncryptConfig {
    // Change if age gets introduced
    let mut encrypt_config = EncryptConfig::from(Proto::Gpg);
    encrypt_config.gpg_tty = tty;
    encrypt_config
}

/// Construct crypto context based on `wutag.yml`
/// [`EncryptConfig`](crate::config::EncryptConfig)
pub(crate) fn context(tty: bool) -> Result<Context, super::Error> {
    super::context(&self::config(tty))
}
