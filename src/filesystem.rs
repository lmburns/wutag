use std::{
    borrow::Cow,
    env,
    ffi::OsStr,
    fs::{self, File, Metadata},
    io::{self, Write},
    path::{Path, PathBuf},
};

use rand::{distributions::Alphanumeric, Rng};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};

use colored::Colorize;
use thiserror::Error;

use crate::wutag_error;

/// FileTypes to filter against when searching (taken from `fd`)
#[derive(Debug, Clone)]
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

impl Default for FileTypes {
    fn default() -> FileTypes {
        FileTypes {
            files:            false,
            directories:      false,
            symlinks:         false,
            block_devices:    false,
            char_devices:     false,
            sockets:          false,
            fifos:            false,
            executables_only: false,
            empty_only:       false,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("No metadata exists for {0}")]
    Metadata(String),
    #[error("IO Error: {0}")]
    IOError(String),
}

pub(crate) type FileInfoResult<T> = Result<T, Error>;

pub(crate) trait FileInfo {
    fn path(&self) -> &Path;
    fn file_type(&self) -> Option<fs::FileType>;
    fn meta(&self) -> FileInfoResult<Metadata>;
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
        let metadata = fs::metadata(self.path()).unwrap();
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
    pub(crate) fn should_ignore(&self, entry: &impl FileInfo) -> bool {
        if let Some(ref entry_type) = entry.file_type() {
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
        } else {
            true
        }
    }
}

pub(crate) fn is_empty(entry: &impl FileInfo) -> bool {
    if let Some(file_type) = entry.file_type() {
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
    } else {
        false
    }
}

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
    tmp_path.display().to_string()
}

pub(crate) fn modify_temp_ignore<P: AsRef<Path>>(
    path: P,
    content: &dyn Fn(&mut File) -> io::Result<()>,
) -> Result<PathBuf, Error> {
    let res = File::create(&path);
    let path = path.as_ref().to_path_buf();

    match res {
        Ok(mut fd) => match content(&mut fd) {
            Ok(_) => match fd.sync_all() {
                Ok(_) => Ok(path),
                Err(e) => Err(Error::IOError(format!(
                    "problem when syncing {}: {}",
                    path.display(),
                    e
                ))),
            },
            Err(e) => Err(Error::IOError(format!(
                "problem when writing closure {}: {}",
                path.display(),
                e
            ))),
        },
        Err(e) => Err(Error::IOError(format!(
            "problem when creating {}: {}",
            path.display(),
            e
        ))),
    }
}

pub(crate) fn create_temp_ignore(content: &dyn Fn(&mut File) -> io::Result<()>) -> String {
    let tmp = create_temp_path();
    match modify_temp_ignore(&tmp, content) {
        Ok(tmp) => tmp.display().to_string(),
        Err(e) => {
            wutag_error!("unable to create temporary ignore file: {} {}", tmp, e);
            std::process::exit(1);
        },
    }
}

pub(crate) fn write_temp_ignore(ignores: &[String], file: &File) -> io::Result<()> {
    let mut writer = io::BufWriter::new(file);

    for i in ignores.iter() {
        writeln!(&mut writer, "{}", i).expect("Unable to write to ignore file");
    }

    Ok(())
}

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
        .display()
        .to_string()
        .contains(path.as_ref().to_str().unwrap())
}

/// Convert an OsStr to bytes for RegexBuilder
pub(crate) fn osstr_to_bytes(input: &OsStr) -> Cow<[u8]> {
    use std::os::unix::ffi::OsStrExt;
    Cow::Borrowed(input.as_bytes())
}
