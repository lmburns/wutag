//! Operations on paths of [`File`]s

use crate::{path_str, regex};
use anyhow::{Context, Result};
use lexiclean::Lexiclean;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::{
    borrow::Cow,
    env, fs,
    path::{self, Path, PathBuf},
};

/// A path on the filesystem. Basically a wrapper for [`PathBuf`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FsPath {
    /// Is the `Path` a directory?
    is_dir: bool,
    /// The `PathBuf` of the file
    path:   PathBuf,
}

/// Unescape an octal sequence
pub(crate) fn unescape_octal(s: &str) -> Cow<'_, str> {
    let reg = regex!(r"([0-7]{3})");
    let unescape = |s: &str| u32::from_str_radix(s, 8).expect("failed to parse to octal");

    reg.replace_all(s, |caps: &Captures| {
        unescape(caps.get(0).map_or("", |m| m.as_str())).to_string()
    })
}

impl FsPath {
    /// Is the file a basename?
    pub(crate) fn is_dirname(&self) -> Result<bool> {
        Ok(self
            .path
            .parent()
            .context("couldn't get parent of path")?
            .lexiclean()
            == self.path.lexiclean())
    }

    /// Dereference a symbolic link
    pub(crate) fn dereference(&self) -> Result<PathBuf> {
        self.path.canonicalize().context("failed to canonicalize")
    }

    /// Return the relative path of the file
    pub(crate) fn relative(&self) -> Result<PathBuf> {
        let path = self.path.canonicalize().context("failed to canonicalize")?;
        let cwd = env::current_dir()
            .context("failed to get CWD")?
            .canonicalize()?;

        // TODO: If expanded, change to string
        if path == cwd {
            return Ok(PathBuf::from("."));
        }

        let trailing_separator = |p: &Path| -> PathBuf {
            let sep = &path::MAIN_SEPARATOR.to_string();
            let path = p.iter().map(|p| path_str!(p)).collect::<Vec<_>>();

            let mut joined = if &path[0] == sep {
                format!("{}{}", sep, path[1..].join(sep))
            } else {
                path.join(sep)
            };

            joined.push(path::MAIN_SEPARATOR);

            PathBuf::from(joined)
        };

        let prefix = trailing_separator(&cwd);
        let s_path = path_str!(path);
        let s_prefix = path_str!(prefix);
        if s_path.starts_with(&s_prefix) {
            return Ok(PathBuf::from(format!(
                ".{}{}",
                path::MAIN_SEPARATOR,
                &s_path[s_prefix.len()..]
            )));
        }

        let cwd = cwd.parent().context("failed to get parent")?;
        let prefix = trailing_separator(&cwd.to_path_buf());
        let s_prefix = path_str!(prefix);
        if s_path.starts_with(&s_prefix) {
            return Ok(PathBuf::from(format!(
                "..{}{}",
                path::MAIN_SEPARATOR,
                &s_path[s_prefix.len()..]
            )));
        }

        Ok(path)
    }
}

impl From<PathBuf> for FsPath {
    fn from(p: PathBuf) -> Self {
        Self {
            is_dir: p.is_dir(),
            path:   p,
        }
    }
}

impl From<&Path> for FsPath {
    fn from(p: &Path) -> Self {
        Self {
            is_dir: p.is_dir(),
            path:   p.to_path_buf(),
        }
    }
}
