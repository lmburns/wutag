//! [`File`] structure to turn abstract information in a database about a file
//! into an object Also contains functions that act on [`Txn`] to modify the
//! `file` table

use super::{
    super::{
        common::{
            hash::{blake3_hash, hash_dir},
            utils::convert_to_datetime,
        },
        Txn,
    },
    from_vec,
    wuid::Wuid,
    ID,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use lexiclean::Lexiclean;
use mime::Mime;
use std::{
    borrow::Cow,
    convert::TryFrom,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};
use uuid::Uuid;

use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};

/// Alias to [`Uuid`](uuid::Uuid)
// pub(crate) type FileId = Wuid;
pub(crate) type FileId = ID;

// ======================= File =======================

/// Representation of a file on the filesystem
#[derive(Debug, Clone)]
pub(crate) struct File {
    /// File ID, similar to UUID
    pub(crate) id:        FileId,
    /// Directory the file is located.
    ///
    /// This is a directory regardless of whether the item is a file or a
    /// directory. If the item is a directory, this will be the directory that
    /// is above the chosen directory. If the item is a file, then this will be
    /// the parent directory that houses that file
    pub(crate) directory: String,
    /// Basename of the filepath. This can be a file or a directory name
    pub(crate) name:      String,
    /// [`blake3`](blake3) hash of the file or directory
    pub(crate) hash:      String,
    /// [`MimeType`](crate::util::MimeType) of a file
    pub(crate) mime:      MimeType,
    /// Modification time
    pub(crate) mtime:     DateTime<Local>,
    /// Creation time
    pub(crate) ctime:     DateTime<Local>,
    /// File permission in base-10
    pub(crate) mode:      u32,
    /// Index node of a file / directory
    pub(crate) inode:     u64,
    /// Number of hard links pointing to the file / directory
    pub(crate) links:     u64,
    /// User ID of the file / directory
    pub(crate) uid:       u32,
    /// Group ID of the file / directory
    pub(crate) gid:       u32,
    /// Size of a file in TODO:
    pub(crate) size:      u64,
    /// Is the file name a directory?
    pub(crate) is_dir:    bool,
}

// To use this, the scan of the files would have to be done regularly
// atime:     DateTime<Local>,

impl File {
    /// Create a new `File`. A file can be a directory
    pub(crate) fn new<P: AsRef<Path>>(path: P, follow_links: bool) -> Result<Self> {
        let path = path
            .as_ref()
            .lexiclean()
            .canonicalize()
            .context("failed to canonicalize")?;
        let file = fs::File::open(&path).context("failed to open file")?;
        let meta = file.metadata().context("failed to get file metadata")?;

        // id:        Wuid::new(),
        Ok(Self {
            id:        ID::null(),
            directory: path
                .parent()
                .context("failed to get parent")?
                .to_string_lossy()
                .to_string(),
            name:      path
                .file_name()
                .context("failed to get file name")?
                .to_string_lossy()
                .to_string(),
            hash:      {
                if path.is_dir() {
                    hash_dir(follow_links, &path, |p, perm| blake3_hash(p, perm))?
                } else {
                    blake3_hash(&path, meta.permissions().mode())?
                }
            }
            .to_string(),
            mime:      MimeType::try_from(&path).context("failed to get mimetype")?,
            mtime:     convert_to_datetime(
                meta.modified().context("failed to get modification time")?,
            ),
            ctime:     convert_to_datetime(meta.created().context("failed to get created time")?),
            mode:      meta.permissions().mode(),
            inode:     meta.ino(),
            links:     meta.nlink(),
            uid:       meta.uid(),
            gid:       meta.gid(),
            size:      meta.len(),
            is_dir:    path.is_dir(),
        })
    }

    /// Join the directory and file name, resulting in the full [`PathBuf`]
    pub(crate) fn path(&self) -> PathBuf {
        PathBuf::from(&self.directory).join(&self.name)
    }

    /// Set the `id` field of the [`File`]
    pub(crate) fn set_id(&mut self, id: FileId) {
        self.id = id;
    }
}

impl TryFrom<&Row<'_>> for File {
    type Error = rsq::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id:        row.get("id")?,
            directory: row.get("directory")?,
            name:      row.get("name")?,
            hash:      row.get("hash")?,
            mime:      row.get("mime")?,
            mtime:     row.get("mtime")?,
            ctime:     row.get("ctime")?,
            mode:      row.get("mode")?,
            inode:     row.get("inode")?,
            links:     row.get("links")?,
            uid:       row.get("uid")?,
            gid:       row.get("gid")?,
            size:      row.get("size")?,
            is_dir:    row.get("is_dir")?,
        })
    }
}

// TODO: cli_table
// impl ToRow<4> for File {
//     fn to_table_row(&self) -> Row<4> {
//         Row::from([self.key.clone(), self.name.clone(),
// self.get_last_update_string(), self.get_remind_interval_string()])     }
// }

// ======================= Files ======================

/// A vector of [`File`]s
#[derive(Debug, Clone)]
pub(crate) struct Files {
    /// The inner vector of [`File`]s
    inner: Vec<File>,
}

from_vec!(File, Files);

impl Files {
    /// Create a new set of [`Files`]
    pub(crate) fn new(v: Vec<File>) -> Self {
        Self { inner: v }
    }

    /// Add a [`File`] to the set of [`Files`]
    pub(crate) fn push(&mut self, file: File) {
        self.inner.push(file);
    }

    /// Adds [`File`] that match a closure to a new [`Files`] struct
    pub(crate) fn matches<F: FnMut(&File) -> bool>(&self, mut f: F) -> Self {
        let mut res = Self::new(vec![]);

        self.inner
            .iter()
            .filter(|file| f(file))
            .for_each(|file| res.push(file.clone()));

        res
    }
}

// ==================== FileTagCnt ====================

/// Struct holding information about a [`File`]'s number of tags
#[derive(Debug, Clone)]
pub(crate) struct FileTagCnt {
    /// The [`FileId`] of a [`File`]
    id:        FileId,
    /// The directory the [`File`] is in
    directory: String,
    /// The name of the [`File`]
    name:      String,
    /// The number of [`Tag`]s the [`File`] has
    count:     usize,
}

// impl<'a> FileTagCnt<'a> {
//     /// Create a new [`FileTagCnt`]
//     pub(crate) fn new<P: AsRef<Path>>(path: &'a P) -> Self {
//     }
// }

// ===================== MimeType =====================

/// [`Mime`](mime::Mime) wrapper for custom methods
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) struct MimeType(pub(crate) Mime);

// Both of these conversion here really seem unnecessary

impl TryFrom<&Path> for MimeType {
    type Error = anyhow::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let mime_db = xdg_mime::SharedMimeInfo::new();
        let mut builder = mime_db.guess_mime_type();
        let guess = builder
            .path(path)
            .metadata(
                fs::metadata(path)
                    .with_context(|| format!("failed to get metadata for: {}", path.display()))?,
            )
            .data(
                &fs::read(path)
                    .with_context(|| format!("failed to read file: {}", path.display()))?,
            )
            .guess();

        let mime = guess.mime_type();

        Ok(Self(mime.clone()))
    }
}

impl TryFrom<&PathBuf> for MimeType {
    type Error = anyhow::Error;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let mime_db = xdg_mime::SharedMimeInfo::new();
        let mut builder = mime_db.guess_mime_type();
        let guess = builder
            .path(path)
            .metadata(
                fs::metadata(path)
                    .with_context(|| format!("failed to get metadata for: {}", path.display()))?,
            )
            .data(
                &fs::read(path)
                    .with_context(|| format!("failed to read file: {}", path.display()))?,
            )
            .guess();

        let mime = guess.mime_type();

        Ok(Self(mime.clone()))
    }
}

impl FromStr for MimeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Mime::from_str(s).context("failed to get mime")?))
    }
}

impl ToSql for MimeType {
    fn to_sql(&self) -> rsq::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.to_string()))
    }
}

impl FromSql for MimeType {
    fn column_result(val: ValueRef) -> rsq::Result<Self, FromSqlError> {
        match Self::from_str(val.as_str().expect("failed to convert value to str")) {
            Ok(v) => Ok(v),
            Err(err) => Err(FromSqlError::InvalidType),
        }
    }
}
