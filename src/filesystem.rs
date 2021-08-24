use std::{
    borrow::Cow,
    ffi::OsStr,
    fs::{self, Metadata},
    io::{self, Write},
    path::Path,
};

use crate::wutag_error;
/// registry::EntryData;
use colored::Colorize;

use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use thiserror::Error;

/// FileTypes to filter against when searching (taken from `fd`)
#[derive(Debug, Clone)]
pub struct FileTypes {
    pub files:            bool,
    pub directories:      bool,
    pub symlinks:         bool,
    pub block_devices:    bool,
    pub char_devices:     bool,
    pub sockets:          bool,
    pub fifos:            bool,
    pub executables_only: bool,
    pub empty_only:       bool,
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
pub enum Error {
    #[error("No metadata exists for {0}")]
    Metadata(String),
}

pub type FileInfoResult<T> = std::result::Result<T, Error>;

pub trait FileInfo {
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
    pub fn should_ignore(&self, entry: &impl FileInfo) -> bool {
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

pub fn is_empty(entry: &impl FileInfo) -> bool {
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

pub fn create_tmp_ignore(
    content: &dyn Fn(&mut fs::File) -> io::Result<()>,
    append: bool,
) -> String {
    let tmp = fsio::path::get_temporary_file_path("wutag_ignore");
    match fsio::file::modify_file(&tmp, content, append) {
        Ok(_) => tmp,
        Err(e) => {
            wutag_error!("Unable to create wutag ignore file: {:?}", &e);
            std::process::exit(1);
        },
    }
}

pub fn write_ignore(ignores: &[String], file: &fs::File) -> io::Result<()> {
    let mut writer = io::BufWriter::new(file);

    ignores.iter().for_each(|i| {
        writeln!(&mut writer, "{}", i).expect("Unable to write to ignore file");
    });

    Ok(())
}

pub fn delete_file(file: String) {
    match fsio::file::delete(&file) {
        Ok(_) => log::debug!("Ignore file deleted: {}", &file),
        Err(err) => log::debug!("Unable to delete ignore file: {} {:#?}", &file, err),
    }
}

/// Determine whether file (path) contains path and if so, return true
pub fn contained_path<P: AsRef<Path>>(file: P, path: P) -> bool {
    file.as_ref()
        .display()
        .to_string()
        .contains(path.as_ref().to_str().unwrap())
}

/// Convert an OsStr to bytes for RegexBuilder
pub fn osstr_to_bytes(input: &OsStr) -> Cow<[u8]> {
    use std::os::unix::ffi::OsStrExt;
    Cow::Borrowed(input.as_bytes())
}
