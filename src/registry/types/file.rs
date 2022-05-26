//! [`File`] structure to turn abstract information in a database about a file
//! into an object Also contains functions that act on [`Txn`] to modify the
//! `file` table

// TODO: Implement a method on files which compares more than one array
//      For example, a query of /regex/ && value(4)
//      Collect all files matching regex (call to SQL), and collect all files
//      matching value. Then compare the two

use super::{
    super::{
        common::{
            hash::{blake3_hash, hash_dir},
            utils::convert_to_datetime,
        },
        Txn,
    },
    from_vec, impl_vec, ID,
};
use crate::{fail, filesystem::ext4::FileFlag, inner_immute};
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use lexiclean::Lexiclean;
use mime::Mime;
use std::{
    borrow::Cow,
    convert::TryFrom,
    fs::{self, Metadata},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use rusqlite::{
    self as rsq,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Row,
};

#[cfg(all(
    feature = "file-flags",
    target_family = "unix",
    not(target_os = "macos")
))]
use e2p_fileflags::{FileFlags, Flags};

// ╭──────────────────────────────────────────────────────────╮
// │                            ID                            │
// ╰──────────────────────────────────────────────────────────╯

/// Alias to [`ID`](super::ID)
pub(crate) type FileId = ID;

/// A vector of `FileId`s
#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct FileIds {
    inner: Vec<FileId>,
}

from_vec!(FileId, FileIds);
impl_vec!(FileIds, FileId);

// ╭──────────────────────────────────────────────────────────╮
// │                           File                           │
// ╰──────────────────────────────────────────────────────────╯

/// Representation of a file on the filesystem
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub(crate) struct File {
    /// File ID, similar to UUID
    pub(crate) id: FileId,
    /// Directory the file is located.
    ///
    /// This is a directory regardless of whether the item is a file or a
    /// directory. If the item is a directory, this will be the directory that
    /// is above the chosen directory. If the item is a file, then this will be
    /// the parent directory that houses that file
    directory:     String,
    /// Basename of the file-path. This can be a file or a directory name
    name:          String,
    /// [`blake3`](blake3) hash of the file or directory
    hash:          String,
    /// [`MimeType`](crate::util::MimeType) of a file
    mime:          MimeType,
    /// Modification time
    mtime:         DateTime<Local>,
    /// Creation time
    ctime:         DateTime<Local>,
    /// File permission in base-10
    mode:          u32,
    /// Index node of a file / directory
    inode:         u64,
    /// Number of hard links pointing to the file / directory
    links:         u64,
    /// User ID of the file / directory
    uid:           u32,
    /// Group ID of the file / directory
    gid:           u32,
    /// Size of a file in TODO:
    size:          u64,
    /// Is the file name a directory?
    is_dir:        bool,
    /// Is the file or directory a symbolic link?
    is_symlink:    bool,

    #[cfg(all(
        feature = "file-flags",
        target_family = "unix",
        not(target_os = "macos")
    ))]
    /// `e2fsprogs` file attributes
    e2pflags: FileFlag,
}

#[rustfmt::skip]
impl File {
    inner_immute!(id, FileId, false);
    inner_immute!(directory, String);
    inner_immute!(name, String);
    inner_immute!(hash, String);
    inner_immute!(mime, MimeType);
    inner_immute!(mtime, DateTime<Local>);
    inner_immute!(ctime, DateTime<Local>);
    inner_immute!(mode, u32, false);
    inner_immute!(inode, u64, false);
    inner_immute!(links, u64, false);
    inner_immute!(uid, u32, false);
    inner_immute!(gid, u32, false);
    inner_immute!(size, u64, false);
    inner_immute!(is_dir, bool, false);
    inner_immute!(is_symlink, bool, false);

    #[cfg(all(
        feature = "file-flags",
        target_family = "unix",
        not(target_os = "macos")
    ))]
    inner_immute!(e2pflags, FileFlag);

    fn clean_path<P: AsRef<Path>>(path: P, follow_links: bool) -> Result<PathBuf> {
        let mut p = path.as_ref().lexiclean();

        if follow_links {
            p = p.canonicalize().context(fail!("canonicalize path"))?;
        }

        Ok(p)
    }

    /// Set the `id` field of the [`File`]
    pub(crate) fn set_id_mut(&mut self, id: FileId) {
        self.id = id;
    }

    /// Set the `id` field of the [`File`]
    pub(crate) const fn set_id(mut self, id: FileId) -> Self {
        self.id = id;
        self
    }

    /// Modify the [`File`]s directory, due to moving a file
    pub(crate) fn set_directory(mut self, path: &Path) -> Result<Self> {
        self.directory = path
            .parent()
            .context(fail!("get parent"))?
            .to_string_lossy()
            .to_string();

        Ok(self)
    }

    /// Modify the [`File`]s file name, due to file name changes
    pub(crate) fn set_filename(mut self, path: &Path) -> Result<Self> {
        self.name = path
            .file_name()
            .context(fail!("get file name"))?
            .to_string_lossy()
            .to_string();

        Ok(self)
    }

    /// Modify the [`File`]s hash, due to a file modification
    pub(crate) fn set_hash(mut self, path: &Path, follow_links: bool) -> Result<Self> {
        // Doesn't matter if symlink is followed when calculating the hash
        // It's needed for another function however
        let path = Self::clean_path(path, follow_links)?;
        self.hash = {
            if path.is_dir() {
                hash_dir(follow_links, &path, |p, _perm| blake3_hash(p, None))?
            } else {
                blake3_hash(&path, None)?
            }
        }
        .to_string();

        Ok(self)
    }

    /// Modify the [`File`]s mime, due to file type changes
    pub(crate) fn set_mime(mut self, path: &Path) -> Result<Self> {
        // Doesn't matter if symlink is followed when finding the mime
        let path = Self::clean_path(path, true)?;
        self.mime = MimeType::try_from(&path).context(fail!("getting mime-type"))?;

        Ok(self)
    }

    /// Modify the [`File`]s modification time, due to file changes
    pub(crate) fn set_mtime(mut self, meta: &Metadata) -> Result<Self> {
        self.mtime =
            convert_to_datetime(meta.modified().context(fail!("getting modification time"))?);
        Ok(self)
    }

    /// Modify the [`File`]s creation time. Shouldn't really ever change
    pub(crate) fn set_ctime(mut self, meta: &Metadata) -> Result<Self> {
        self.ctime = convert_to_datetime(meta.created().context(fail!("getting created time"))?);
        Ok(self)
    }

    /// Modify the [`File`]s permissions, due to permission changes
    pub(crate) fn set_mode(mut self, meta: &Metadata) -> Self {
        self.mode = format!("{:o}", meta.permissions().mode())
            .parse::<u32>()
            .expect("failed parsing octal digits");
        self
    }

    /// Modify the [`File`]s inode
    pub(crate) fn set_inode(mut self, meta: &Metadata) -> Self {
        self.inode = meta.ino();
        self
    }

    /// Modify the [`File`]s links due to hard/soft link creation
    pub(crate) fn set_links(mut self, meta: &Metadata) -> Self {
        self.links = meta.nlink();
        self
    }

    /// Modify the [`File`]s UID (user ID) due to ownership changes
    pub(crate) fn set_uid(mut self, meta: &Metadata) -> Self {
        self.uid = meta.uid();
        self
    }

    /// Modify the [`File`]s GID (group ID) due to ownership changes
    pub(crate) fn set_gid(mut self, meta: &Metadata) -> Self {
        self.gid = meta.gid();
        self
    }

    /// Modify the [`File`]s size, due to modifications
    pub(crate) fn set_size(mut self, meta: &Metadata) -> Self {
        self.size = meta.len();
        self
    }

    /// Modify the [`File`]s `is_dir` attribute
    pub(crate) fn set_is_dir(mut self, path: &Path) -> Self {
        self.is_dir = path.is_dir();
        self
    }

    /// Modify the [`File`]s `is_symlink` attribute
    pub(crate) fn set_is_symlink(mut self, path: &Path) -> Self {
        self.is_symlink = path.is_symlink();
        self
    }

    /// Modify the [`File`]s [`Metadata`] attributes
    pub(crate) fn set_metadata(mut self, meta: &Metadata) -> Result<Self> {
        self.mtime =
            convert_to_datetime(meta.modified().context(fail!("getting modification time"))?);
        self.ctime = convert_to_datetime(meta.created().context(fail!("getting created time"))?);
        self.mode = format!("{:o}", meta.permissions().mode())
            .parse::<u32>()
            .expect("failed to parse octal digits");;
        self.inode = meta.ino();
        self.links = meta.nlink();
        self.uid = meta.uid();
        self.gid = meta.gid();
        self.size = meta.len();

        Ok(self)
    }

    /// Modify the [`File`]s `ext4` flags, due to flag changes
    ///
    /// Note that symlinks are empty
    #[cfg(all(
        feature = "file-flags",
        target_family = "unix",
        not(target_os = "macos")
    ))]
    pub(crate) fn set_e2pflags(mut self, flags: Flags) -> Self {
        self.e2pflags = FileFlag::from(flags);
        self
    }

    /// Create a new `File`. A file can be a directory
    pub(crate) fn new<P: AsRef<Path>>(path: P, follow_links: bool) -> Result<Self> {
        let path = Self::clean_path(path, follow_links)?;
        // let meta = file.metadata().context(fail!("getting file metadata"))?;
        let file = fs::File::open(&path).context(fail!("opening file"))?;
        let meta = fs::symlink_metadata(&path).context(fail!("getting symlink metadata"))?;

        let mut f = Self::default()
            .set_directory(&path)?
            .set_filename(&path)?
            .set_hash(&path, follow_links)?
            .set_mime(&path)?
            .set_metadata(&meta)?
            .set_is_dir(&path)
            .set_is_symlink(&path);

        #[cfg(all(
            feature = "file-flags",
            target_family = "unix",
            not(target_os = "macos")
        ))]
        let f = f.set_e2pflags(file.flags().context(fail!("getting the file's flags"))?);

        // .set_mtime(&meta)?
        // .set_ctime(&meta)?
        // .set_mode(&meta)
        // .set_inode(&meta)
        // .set_links(&meta)
        // .set_uid(&meta)
        // .set_gid(&meta)
        // .set_size(&meta)

        Ok(f)
    }

    /// Join the directory and file name, resulting in the full [`PathBuf`]
    pub(crate) fn path(&self) -> PathBuf {
        PathBuf::from(&self.directory).join(&self.name)
    }
}

impl TryFrom<&Row<'_>> for File {
    type Error = rsq::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id:         row.get("id")?,
            directory:  row.get("directory")?,
            name:       row.get("name")?,
            hash:       row.get("hash")?,
            mime:       row.get("mime")?,
            mtime:      row.get("mtime")?,
            ctime:      row.get("ctime")?,
            mode:       row.get("mode")?,
            inode:      row.get("inode")?,
            links:      row.get("links")?,
            uid:        row.get("uid")?,
            gid:        row.get("gid")?,
            size:       row.get("size")?,
            is_dir:     row.get("is_dir")?,
            is_symlink: row.get("is_symlink")?,

            #[rustfmt::skip]
            #[cfg(all(
                feature = "file-flags",
                target_family = "unix",
                not(target_os = "macos")
            ))]
            e2pflags:  row.get("e2pflags")?,
        })
    }
}

impl Default for File {
    fn default() -> Self {
        Self {
            id:         ID::null(),
            directory:  String::default(),
            name:       String::default(),
            hash:       String::default(),
            mime:       MimeType::default(),
            mtime:      Local::now(),
            ctime:      Local::now(),
            mode:       u32::default(),
            inode:      u64::default(),
            links:      u64::default(),
            uid:        u32::default(),
            gid:        u32::default(),
            size:       u64::default(),
            is_dir:     bool::default(),
            is_symlink: bool::default(),
            e2pflags:   FileFlag::from(Flags::default()),
        }
    }
}

// TODO: cli_table
// impl ToRow<4> for File {
//     fn to_table_row(&self) -> Row<4> {
//         Row::from([self.key.clone(), self.name.clone(),
// self.get_last_update_string(), self.get_remind_interval_string()])     }
// }

// ╭──────────────────────────────────────────────────────────╮
// │                          Files                           │
// ╰──────────────────────────────────────────────────────────╯

/// A vector of [`File`]s
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub(crate) struct Files {
    /// The inner vector of [`File`]s
    inner: Vec<File>,
}

from_vec!(File, Files);

impl Files {
    impl_vec!(File);

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

// ╭──────────────────────────────────────────────────────────╮
// │                        FileTagCnt                        │
// ╰──────────────────────────────────────────────────────────╯

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

// ╭──────────────────────────────────────────────────────────╮
// │                         MimeType                         │
// ╰──────────────────────────────────────────────────────────╯

/// [`Mime`](mime::Mime) wrapper for custom methods
#[derive(Debug, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub(crate) struct MimeType(pub(crate) Mime);

macro_rules! mime_try {
    ($t:tt) => {
        impl TryFrom<&$t> for MimeType {
            type Error = anyhow::Error;

            fn try_from(path: &$t) -> Result<Self, Self::Error> {
                let mime_db = xdg_mime::SharedMimeInfo::new();
                let mut builder = mime_db.guess_mime_type();
                let guess = builder
                    .path(path)
                    .metadata(
                        fs::metadata(path)
                            .context(fail!("getting metadata for: {}", path.display()))?,
                    )
                    .data(&fs::read(path).context(fail!("reading file: {}", path.display()))?)
                    .guess();

                let mime = guess.mime_type();

                Ok(Self(mime.clone()))
            }
        }
    };
}

// Both of these conversion here really seem unnecessary
mime_try!(Path);
mime_try!(PathBuf);

impl FromStr for MimeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Mime::from_str(s).context(fail!("getting mime"))?))
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

impl Default for MimeType {
    fn default() -> Self {
        Self(Mime::from_str("text/plain").expect("failed to set text/plain mime"))
    }
}

mod test {
    use super::{
        super::super::common::utils::convert_to_datetime, File, FileId, FileIds, Mime, MimeType,
    };
    use std::{convert::TryFrom, os::unix::fs::MetadataExt, path::PathBuf, str::FromStr};

    #[test]
    fn unique_fileids() {
        let v = vec![1, 2, 5, 5, 3, 1, 7]
            .iter()
            .map(|i| FileId::new(*i))
            .collect::<Vec<_>>();
        let mut ids = FileIds::new(v);

        assert!(ids.len() == 7);

        ids.unique();
        assert!(ids.len() == 5);

        assert_eq!(ids, FileIds {
            inner: vec![1, 2, 3, 5, 7]
                .iter()
                .map(|i| FileId::new(*i))
                .collect::<Vec<_>>(),
        });
    }

    #[test]
    fn mimetype() {
        let path = PathBuf::from("./src/main.rs");
        let mime = MimeType::try_from(&path).expect("failed to find main.rs");
        assert_eq!(mime, MimeType(Mime::from_str("text/rust").unwrap()));

        let path = PathBuf::from("./Cargo.toml");
        let mime = MimeType::try_from(&path).expect("failed to find Cargo.toml");
        assert_eq!(mime, MimeType(Mime::from_str("application/toml").unwrap()));

        let path = PathBuf::from("./Cargo.lock");
        let mime = MimeType::try_from(&path).expect("failed to find Cargo.lock");
        assert_eq!(mime, MimeType(Mime::from_str("text/plain").unwrap()));
    }

    #[test]
    fn file_struct() {
        let path = PathBuf::from("./src/main.rs");
        let file = File::new(&path, false).expect("failed to find main.rs");
        let meta = path.metadata().expect("failed to get metadata");

        assert!(!file.is_dir());
        assert_eq!(
            file.directory(),
            &PathBuf::from("./src")
                .canonicalize()
                .expect("failed to canonicalize main.rs")
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(file.inode(), meta.ino());
        assert_eq!(file.name(), "main.rs");
        assert_eq!(
            file.ctime(),
            &convert_to_datetime(meta.created().expect("failed to get ctime"))
        );
    }
}
