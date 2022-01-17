//! Utility functions to execute on files or having to do with the `filesystem`

use anyhow::Result;
use rand::{distributions::Alphanumeric, Rng};
use std::{
    borrow::Cow,
    env,
    ffi::OsStr,
    fs::{self, File, Metadata},
    io::{self, Write},
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::{Path, PathBuf},
};

use colored::Colorize;
use thiserror::Error;

use crate::wutag_error;

/// FileTypes to filter against when searching (taken from `fd`)
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Clone, Default)]
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

/// Create the temporary ignore file with the given contents
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

/// Write the temporary ignore file (passed to `create_temp_ignore()`, which
/// returns a string of the contents of the file)
pub(crate) fn write_temp_ignore(ignores: &[String], file: &File) -> io::Result<()> {
    let mut writer = io::BufWriter::new(file);

    for i in ignores.iter() {
        writeln!(&mut writer, "{}", i)?;
    }

    Ok(())
}

/// Delete the temporarily created ignore file
pub(crate) fn delete_file<P: AsRef<Path>>(file: P) {
    let path = file.as_ref().to_path_buf();

    if path.exists() && path.is_file() {
        match fs::remove_file(&path) {
            Ok(_) => log::debug!("Ignore file deleted: {}", &path.display()),
            Err(err) => log::debug!(
                "Unable to delete ignore file: {} {:#?}",
                &path.display(),
                err
            ),
        }
    } else {
        println!();
    }
}

/// Determine whether file (path) contains path and if so, return true
pub(crate) fn contained_path<P: AsRef<Path>>(file: P, path: P) -> bool {
    file.as_ref()
        .to_string_lossy()
        .to_string()
        .starts_with(&path.as_ref().to_string_lossy().to_string())
}

/// Convert an OsStr to bytes for RegexBuilder
pub(crate) fn osstr_to_bytes(input: &OsStr) -> Cow<[u8]> {
    use std::os::unix::ffi::OsStrExt;
    Cow::Borrowed(input.as_bytes())
}
