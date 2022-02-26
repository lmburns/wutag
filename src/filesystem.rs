//! Utility functions to execute on files or having to do with the `filesystem`

#![allow(unused)]

use crate::{wutag_error, wutag_info};
use anyhow::Result;
use colored::Colorize;
use rand::{distributions::Alphanumeric, Rng};
use std::{
    borrow::Cow,
    env,
    ffi::OsStr,
    fmt,
    fs::{self, File, Metadata},
    io::{self, Write},
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::{Path, PathBuf},
};
use thiserror::Error;

/// FileTypes to filter against when searching (taken from `fd`)
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FileTypes {
    pub(crate) files:            bool,
    pub(crate) directories:      bool,
    pub(crate) symlinks:         bool,
    pub(crate) block_devices:    bool,
    pub(crate) char_devices:     bool,
    pub(crate) sockets:          bool,
    pub(crate) fifos:            bool,
    pub(crate) executables_only: bool,
    pub(crate) empty_only:       bool,
}

/// Errors used within the `filesystem` module
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// Missing metadata
    #[error("No metadata exists for {0}")]
    Metadata(String),

    /// General `io` error
    #[error("IO Error: {0}")]
    IOError(String),
}

/// Shorter `typedef` for a `Result`
pub(crate) type FileInfoResult<T> = Result<T, Error>;

/// Used to gain extra information about a file. The path of the file can be a
/// `Path` or a [`DirEntry`](ignore::DirEntry)
pub(crate) trait FileInfo {
    /// Return the [`Path`](std::path::Path)
    fn path(&self) -> &Path;
    /// Return the [`FileType`](std::fs::FileType)
    fn file_type(&self) -> Option<fs::FileType>;
    /// Return the [`Metadata`](std::fs::Metadata)
    fn meta(&self) -> FileInfoResult<Metadata>;
    /// Is the file an executable?
    fn is_executable(&self) -> bool;
}

impl FileInfo for ignore::DirEntry {
    fn path(&self) -> &Path {
        self.path()
    }

    fn file_type(&self) -> Option<fs::FileType> {
        self.file_type()
    }

    fn meta(&self) -> FileInfoResult<Metadata> {
        match self.metadata() {
            Ok(meta) => Ok(meta),
            Err(e) => Err(Error::Metadata(e.to_string())),
        }
    }

    fn is_executable(&self) -> bool {
        self.metadata()
            .map(|m| &m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
}

impl FileInfo for &Path {
    fn path(&self) -> &Path {
        self
    }

    fn file_type(&self) -> Option<fs::FileType> {
        let metadata = fs::metadata(self.path()).expect("failed to determine file's metadata");
        Some(metadata.file_type())
    }

    fn meta(&self) -> FileInfoResult<Metadata> {
        match self.metadata() {
            Ok(meta) => Ok(meta),
            Err(e) => Err(Error::Metadata(e.to_string())),
        }
    }

    fn is_executable(&self) -> bool {
        self.metadata()
            .map(|m| &m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
}

impl FileTypes {
    /// File types that should be ignored based on CLI arguments
    pub(crate) fn should_ignore(&self, entry: &impl FileInfo) -> bool {
        entry.file_type().as_ref().map_or(true, |entry_type| {
            (!self.files && entry_type.is_file())
                || (!self.directories && entry_type.is_dir())
                || (!self.symlinks && entry_type.is_symlink())
                || (!self.block_devices && entry_type.is_block_device())
                || (!self.char_devices && entry_type.is_char_device())
                || (!self.sockets && entry_type.is_socket())
                || (!self.fifos && entry_type.is_fifo())
                || (self.executables_only && !entry.is_executable())
                || (self.empty_only && !is_empty(entry))
                || !(entry_type.is_file()
                    || entry_type.is_dir()
                    || entry_type.is_symlink()
                    || entry_type.is_block_device()
                    || entry_type.is_char_device()
                    || entry_type.is_socket()
                    || entry_type.is_fifo())
        })
    }
}

// ========================== FileFlag ==========================

/// A module to only have to write the feature once, instead of several times
///
/// Used for Linux file flags
#[cfg(all(
    feature = "file-flags",
    target_family = "unix",
    not(target_os = "macos")
))]
pub(crate) mod ext4 {
    use e2p_fileflags::Flags;
    use rusqlite::{
        self as rsq,
        types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    };
    use std::str::FromStr;

    /// Easier way to check whether [`FileFlag`] contains a flag
    ///
    /// [`FileFlag`]: self::FileFlags
    macro_rules! fileflag_contains {
        ($name:tt, $flag:path) => {
            /// Test whether [`FileFlag`] contains any given [`Flags`]
            pub(crate) const fn $name(&self) -> bool {
                self.0.contains($flag)
            }
        };
    }

    /// `ext2`, `ext3`, `ext4` file flags used with `chattr(1)` and `lsattr(1)`
    ///
    /// Used for custom implementations and a better description of flag names
    #[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
    pub(crate) struct FileFlag(Flags);

    #[rustfmt::skip]
    impl FileFlag {
        fileflag_contains!(has_append,     Flags::APPEND);
        fileflag_contains!(has_noatime,    Flags::NOATIME);
        fileflag_contains!(has_compressed, Flags::COMPR);
        fileflag_contains!(has_nocow,      Flags::NOCOW);
        fileflag_contains!(has_nodump,     Flags::NODUMP);
        fileflag_contains!(has_sync,       Flags::DIRSYNC);
        fileflag_contains!(has_extent,     Flags::EXTENTS);
        fileflag_contains!(has_immutable,  Flags::IMMUTABLE);
        fileflag_contains!(has_journal,    Flags::JOURNAL_DATA);
        fileflag_contains!(has_secdel,     Flags::SECRM);
        fileflag_contains!(has_notail,     Flags::NOTAIL);
        fileflag_contains!(has_undel,      Flags::UNRM);

        /// Test whether the [`Flags`] contain a character
        pub(crate) const fn has_flag(&self, ch: char) -> bool {
            match ch {
                'a' => self.has_append(),
                'A' => self.has_noatime(),
                'c' => self.has_compressed(),
                'C' => self.has_nocow(),
                'd' => self.has_nodump(),
                'D' => self.has_sync(),
                'e' => self.has_extent(),
                'i' => self.has_immutable(),
                'j' => self.has_journal(),
                's' => self.has_secdel(),
                't' => self.has_notail(),
                'u' => self.has_undel(),
                _ => false
            }
        }

        /// Test whether all [`Flags`] (as a string) are contained within [`FileFlag`]
        ///
        /// For example, `eia` is an argument that is wanted
        pub(crate) fn has_flags<S: AsRef<str>>(&self, flags: S) -> bool {
            flags.as_ref().chars().all(|ch| self.has_flag(ch))
        }
    }

    impl From<Flags> for FileFlag {
        fn from(flags: Flags) -> Self {
            Self(flags)
        }
    }

    impl From<&Flags> for FileFlag {
        fn from(flags: &Flags) -> Self {
            Self(*flags)
        }
    }

    impl TryFrom<char> for FileFlag {
        type Error = anyhow::Error;

        fn try_from(ch: char) -> Result<Self, Self::Error> {
            Self::from_str(&ch.to_string())
        }
    }

    impl FromStr for FileFlag {
        type Err = anyhow::Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s.trim() {
                "a" => Ok(Self(Flags::APPEND)),
                "A" => Ok(Self(Flags::NOATIME)),
                "c" => Ok(Self(Flags::COMPR)),
                "C" => Ok(Self(Flags::NOCOW)),
                "d" => Ok(Self(Flags::NODUMP)),
                "D" => Ok(Self(Flags::DIRSYNC)),
                "e" => Ok(Self(Flags::EXTENTS)),
                "i" => Ok(Self(Flags::IMMUTABLE)),
                "j" => Ok(Self(Flags::JOURNAL_DATA)),
                "s" => Ok(Self(Flags::SECRM)),
                "t" => Ok(Self(Flags::NOTAIL)),
                "u" => Ok(Self(Flags::UNRM)),
                _ => Err(anyhow::anyhow!("invalid flag was given: {}", s)),
            }
        }
    }

    impl ToSql for FileFlag {
        fn to_sql(&self) -> rsq::Result<ToSqlOutput<'_>> {
            Ok(ToSqlOutput::from(self.0.bits()))
        }
    }

    impl FromSql for FileFlag {
        fn column_result(val: ValueRef) -> rsq::Result<Self, FromSqlError> {
            let u = val.as_i64().expect("failed to convert to i64") as u32;
            let bits = Flags::from_bits(u).ok_or(FromSqlError::InvalidType)?;
            Ok(Self(bits))
        }
    }
}

// ========================== Functions ==========================

/// Check whether the file is empty
#[allow(clippy::filetype_is_file)]
pub(crate) fn is_empty(entry: &impl FileInfo) -> bool {
    entry.file_type().map_or(false, |file_type| {
        if file_type.is_dir() {
            if let Ok(mut entries) = fs::read_dir(entry.path()) {
                entries.next().is_none()
            } else {
                false
            }
        } else if file_type.is_file() {
            entry.meta().map(|m| m.len() == 0).unwrap_or(false)
        } else {
            false
        }
    })
}

/// Create a path to a temporary file
pub(crate) fn create_temp_path() -> String {
    let mut tmp_path = env::temp_dir();
    tmp_path.push(format!(
        "{}-{}",
        env!("CARGO_PKG_NAME"),
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect::<String>()
    ));
    tmp_path.to_string_lossy().to_string()
}

/// Modify the temporary ignores file that is built from the configuration file
pub(crate) fn modify_temp_ignore<P: AsRef<Path>>(
    path: P,
    content: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<PathBuf, Error> {
    let res = File::create(&path);
    let path = path.as_ref().to_path_buf();

    let expand_err = |s: &str, e: io::Error| -> Error {
        Error::IOError(format!("problem when {} {}: {}", s, path.display(), e))
    };

    match res {
        Ok(mut fd) => match content(&mut fd) {
            Ok(_) => match fd.sync_all() {
                Ok(_) => Ok(path),
                Err(e) => Err(expand_err("syncing", e)),
            },
            Err(e) => Err(expand_err("writing", e)),
        },
        Err(e) => Err(expand_err("creating", e)),
    }
}

/// Create the temporary ignore-file with the given contents
pub(crate) fn create_temp_ignore(content: impl FnOnce(&mut File) -> io::Result<()>) -> String {
    let tmp = create_temp_path();
    match modify_temp_ignore(&tmp, content) {
        Ok(tmp) => return tmp.display().to_string(),
        Err(e) => {
            wutag_error!("unable to create temporary ignore file: {} {}", tmp, e);
            std::process::exit(1);
        },
    }
}

/// Write the temporary ignore file (passed to [`create_temp_ignore`], which
/// returns a `String` of the contents of the file)
pub(crate) fn write_temp_ignore(ignores: &[String], file: &File) -> io::Result<()> {
    let mut writer = io::BufWriter::new(file);

    for i in ignores.iter() {
        writeln!(writer, "{}", i)?;
    }

    Ok(())
}

/// Delete any file (used for temporary files)
pub(crate) fn delete_file<P: AsRef<Path>>(file: P) {
    let path = file.as_ref().to_path_buf();

    if path.exists() && path.is_file() {
        match fs::remove_file(&path) {
            Ok(_) => log::debug!("Ignore file deleted: {}", &path.display()),
            Err(err) => wutag_info!("Unable to delete ignore file: {} {}", &path.display(), err),
        }
    }
}

/// Determine whether file (path) contains path and if so, return true
pub(crate) fn contained_path<P: AsRef<Path>>(file: P, path: P) -> bool {
    file.as_ref()
        .to_string_lossy()
        .starts_with(&path.as_ref().to_string_lossy().to_string())
}

/// Convert an [`OsStr`] to bytes for
/// [`RegexBuilder`](regex::bytes::RegexBuilder)
pub(crate) fn osstr_to_bytes(input: &OsStr) -> Cow<[u8]> {
    use std::os::unix::ffi::OsStrExt;
    Cow::Borrowed(input.as_bytes())
}
