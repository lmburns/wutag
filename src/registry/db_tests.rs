//! Tests to check that database queries are working correctly.

use super::{
    common::hash,
    transaction::Txn,
    types::file::{File, FileId, Files, MimeType},
    Registry,
};
use crate::filesystem as wfs;
use anyhow::{Context, Result};
use rusqlite::{self as rsq, params, Connection};
use std::{env, os::unix::prelude::MetadataExt, path::PathBuf, str::FromStr};

const DB_NAME: &str = "./tests/my.db";

// ============================== Files ===============================
// ====================================================================

macro_rules! first_idx {
    ($files:ident) => {
        assert_eq!(
            $files.inner().get(0).context("first index")?.name(),
            "README.md"
        );
    };
}

fn setup_db() -> Result<Registry> {
    let path = PathBuf::from(DB_NAME);
    let conn = Connection::open(&path)?;
    let reg = Registry::new(&path, conn, false)?;
    reg.init()?;

    Ok(reg)
}

fn setup_dbfn<F>(mut f: F) -> Result<()>
where
    F: FnMut(&Txn) -> Result<()>,
{
    scopeguard::defer!(wfs::delete_file(DB_NAME));
    let reg = setup_db()?;
    let txn = Txn::new(&reg)?;
    f(&txn);

    txn.commit()?;

    Ok(())
}

fn setup_dbfn_wfile<F>(mut f: F) -> Result<()>
where
    F: FnMut(&Txn) -> Result<()>,
{
    setup_dbfn(|txn| {
        let test = PathBuf::from("./README.md");
        let ret = txn.insert_file(test)?;

        f(txn);

        Ok(())
    })
}

#[test]
fn insert_file() -> Result<()> {
    setup_dbfn(|txn| {
        let test = PathBuf::from("./README.md");
        let ret = txn.insert_file(test)?;

        assert_eq!(ret.name(), "README.md");
        assert_eq!(ret.directory(), &env::current_dir()?.display().to_string());
        assert_eq!(ret.mime(), &MimeType::from_str("text/markdown")?);

        #[cfg(all(
            feature = "file-flags",
            target_family = "unix",
            not(target_os = "macos")
        ))]
        {
            use crate::filesystem::ext4::FileFlag;
            use e2p_fileflags::Flags;
            assert_eq!(ret.e2pflags(), &FileFlag::from(Flags::EXTENTS));
        }

        Ok(())
    })
}

#[test]
fn glob_matching() -> Result<()> {
    setup_dbfn(|txn| {
        let files: Vec<File> = txn
            .query_vec(
                "SELECT * FROM file
                WHERE glob('*.md', name) == 1",
                params![],
                |row| row.try_into().expect("failed to convert to `File`"),
            )
            .context("failed to query for `File` with regex: ")?;

        assert_eq!(
            files.get(0).context("failed to get first index")?.name(),
            "README.md"
        );

        let files = txn.select_files_by_glob("name", "*.md")?;
        assert_eq!(
            files.inner().get(0).context("first index")?.name(),
            "README.md"
        );

        let files = txn.select_files_by_iglob("name", "*.MD")?;
        assert_eq!(
            files.inner().get(0).context("first index")?.name(),
            "README.md"
        );

        let files = txn.select_files_by_glob("fullpath(directory, name)", "**/*.md")?;
        assert_eq!(
            files.inner().get(0).context("first index")?.name(),
            "README.md"
        );

        Ok(())
    })
}

#[test]
fn regex_matching() -> Result<()> {
    setup_dbfn(|txn| {
        let files = txn.select_files_by_regex("name", "READ.*")?;
        assert_eq!(
            files.inner().get(0).context("first index")?.name(),
            "README.md"
        );

        let files = txn.select_files_by_iregex("name", "rEaD.*")?;
        assert_eq!(
            files.inner().get(0).context("first index")?.name(),
            "README.md"
        );

        let files = txn.select_files_by_regex("fullpath(directory, name)", ".*.md")?;
        assert_eq!(
            files.inner().get(0).context("first index")?.name(),
            "README.md"
        );

        Ok(())
    })
}

#[test]
fn select_files_by() -> Result<()> {
    setup_dbfn(|txn| {
        let mfile = PathBuf::from("./README.md");
        let meta = mfile.metadata()?;
        let cwd = env::current_dir()?.display().to_string();

        // === ID ===
        let file = txn.select_file(FileId::new(0))?;
        assert_eq!(file.name(), "README.md");

        // === path ===
        let files = txn.select_file_by_path(format!("{}/README.md", cwd))?;
        assert_eq!(files.name(), "README.md");

        // === directory ===
        let files = txn.select_files_by_directory(cwd, false)?;
        first_idx!(files);

        // === hash ===
        let files = txn.select_files_by_hash(
            hash::blake3_hash(mfile.display().to_string(), None)?.to_string(),
        )?;
        first_idx!(files);

        // === Mime ===
        let files = txn.select_files_by_mime("text/markdown")?;
        first_idx!(files);

        // === mtime ===
        let files = txn.select_files_by_mtime(format!("{}", meta.mtime()))?;
        first_idx!(files);

        // === ctime ===
        let files = txn.select_files_by_ctime(format!("{}", meta.ctime()))?;
        first_idx!(files);

        // === mode ===
        let files = txn.select_files_by_mode("644")?;
        first_idx!(files);
        let files = txn.select_files_by_mode("0644")?;
        first_idx!(files);
        let files = txn.select_files_by_mode("100644")?;
        first_idx!(files);

        // === inode ===
        let files = txn.select_files_by_inode(meta.ino())?;
        first_idx!(files);

        // === links ===
        let files = txn.select_files_by_links(meta.nlink())?;
        first_idx!(files);

        // === uid ===
        let files = txn.select_files_by_uid(meta.uid().into())?;
        first_idx!(files);

        // === gid ===
        let files = txn.select_files_by_gid(meta.gid().into())?;
        first_idx!(files);

        // === size ===
        let files = txn.select_files_by_size(meta.size())?;
        first_idx!(files);

        // === directories ===
        let files = txn.select_directories()?;
        assert!(files.is_empty());

        // === e2pflags ===
        #[cfg(all(
            feature = "file-flags",
            target_family = "unix",
            not(target_os = "macos")
        ))]
        {
            let files = txn.select_files_by_flag("e")?;
            first_idx!(files);

            let files = txn.select_files_by_flag("a")?;
            assert!(files.is_empty());
        }

        Ok(())
    })
}

#[cfg(all(
    feature = "file-flags",
    target_family = "unix",
    not(target_os = "macos")
))]
#[test]
fn file_flags() -> Result<()> {
    use e2p_fileflags::{FileFlags, Flags};

    setup_dbfn_wfile(|txn| {
        let mfile = PathBuf::from("./README.md");
        let flags = mfile.flags()?;

        let og_files = txn.select_files_by_flag("e")?;
        first_idx!(og_files);

        mfile.set_flags(Flags::COMPR | flags)?;

        let new = txn.update_file(
            og_files.get(0).context("failed to get first idx")?.id,
            &mfile,
        )?;

        let new_files = txn.select_files_by_flag("e")?;
        assert!(og_files != new_files);

        let selmore = txn.select_files_by_flag("ec")?;
        let e2pf = selmore
            .get(0)
            .context("failed to get first idx")?
            .e2pflags();
        assert!(e2pf.has_extent(), "file does not have extent flag");
        assert!(e2pf.has_compressed(), "file does not have compressed flag");

        // Return flags back to normal
        mfile.set_flags(flags)?;

        Ok(())
    })
}

#[test]
fn file_counts() -> Result<()> {
    setup_dbfn_wfile(|txn| {
        let c = txn.select_file_count()?;
        assert_eq!(c, 1);

        Ok(())
    })
}

#[test]
fn file_delete() -> Result<()> {
    setup_dbfn_wfile(|txn| {
        let f = txn.select_file_by_path(PathBuf::from("./README.md").canonicalize()?)?;
        txn.delete_file(f.id)?;

        assert!(txn
            .select_file_by_path(PathBuf::from("./README.md").canonicalize()?)
            .is_err());

        Ok(())
    })
}
