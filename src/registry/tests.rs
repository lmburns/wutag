//! Tests to check that database queries are working correctly.

use super::{
    common::hash,
    transaction::Txn,
    types::{
        file::{File, FileId, Files, MimeType},
        filetag::{FileTag, FileTags},
        query::{Queries, Query},
        tag::{Tag, TagId, Tags},
        value::{Value, ValueId},
    },
    Registry,
};
use crate::filesystem as wfs;
use anyhow::{Context, Result};
use rusqlite::{self as rsq, params, Connection};
use std::{env, os::unix::prelude::MetadataExt, path::PathBuf, str::FromStr};
use wutag_core::color::parse_color;

const DB_NAME: &str = "./tests/my.db";

macro_rules! get_0 {
    ($f:tt) => {
        $f.get(0).context("first idx")?
    };
}

macro_rules! first_idx {
    ($files:tt) => {
        assert_eq!(get_0!($files).name(), "README.md");
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

// ============================== Files ===============================
// ====================================================================

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

        first_idx!(files);

        let files = txn.select_files_by_glob("name", "*.md")?;
        first_idx!(files);

        let files = txn.select_files_by_iglob("name", "*.MD")?;
        first_idx!(files);

        let files = txn.select_files_by_glob_fp("**/*.md")?;
        first_idx!(files);

        let files = txn.select_files_by_iglob_fp("**/*.md")?;
        first_idx!(files);

        Ok(())
    })
}

#[test]
fn regex_matching() -> Result<()> {
    setup_dbfn(|txn| {
        let files = txn.select_files_by_regex("name", "READ.*")?;
        first_idx!(files);

        let files = txn.select_files_by_iregex("name", "rEaD.*")?;
        first_idx!(files);

        let files = txn.select_files_by_regex_fp(".*.md")?;
        first_idx!(files);

        let files = txn.select_files_by_iregex_fp(".*.md")?;
        first_idx!(files);

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
        let pb = PathBuf::from("./README.md").canonicalize()?;
        let f = txn.select_file_by_path(&pb)?;
        txn.delete_file(f.id)?;

        assert!(txn.select_file_by_path(pb).is_err());

        Ok(())
    })
}

// ============================= Filetag ==============================
// ====================================================================

fn setup_dbfn_wfiletag<F>(mut f: F) -> Result<()>
where
    F: FnMut(&Txn) -> Result<()>,
{
    setup_dbfn(|txn| {
        let test = PathBuf::from("./README.md");
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));

        txn.insert_file(test)?;
        txn.insert_filetag(&ftag)?;

        f(txn);

        Ok(())
    })
}

#[test]
fn delete_filetag() -> Result<()> {
    setup_dbfn_wfiletag(|txn| {
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));
        txn.delete_filetag(&ftag)?;

        let f2 = FileTag::new(2.into(), 1.into(), 2.into());

        txn.insert_filetag(&f2)?;
        assert!(txn.filetag_exists(&f2)?);
        txn.delete_filetag_by_fileid(2.into())?;
        assert!(!txn.filetag_exists(&f2)?);

        txn.insert_filetag(&f2)?;
        assert!(txn.filetag_exists(&f2)?);
        txn.delete_filetag_by_tagid(1.into())?;
        assert!(!txn.filetag_exists(&f2)?);

        txn.insert_filetag(&f2)?;
        assert!(txn.filetag_exists(&f2)?);
        txn.delete_filetag_by_valueid(2.into())?;
        assert!(!txn.filetag_exists(&f2)?);

        Ok(())
    })
}

#[test]
fn filetag_select() -> Result<()> {
    setup_dbfn_wfiletag(|txn| {
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));

        assert!(txn.filetag_exists(&ftag)?);
        assert_eq!(txn.select_filetag_count()?, 1);

        let new = txn.select_filetags()?;
        assert_eq!(*new.get(0).context("failed to get first idx")?, ftag);

        let ftag2 = FileTag::new(FileId::new(1), TagId::new(2), ValueId::new(2));
        txn.insert_filetag(&ftag2)?;

        let f = txn.select_filetags_by_fileid(FileId::new(2))?;
        assert_eq!(*get_0!(f), ftag);

        let f = txn.select_filetags_by_tagid(TagId::new(1))?;
        assert_eq!(*get_0!(f), ftag);

        let f = txn.select_filetags_by_valueid(ValueId::new(1))?;
        assert_eq!(*get_0!(f), ftag);

        Ok(())
    })
}

#[test]
fn filetag_count() -> Result<()> {
    setup_dbfn_wfiletag(|txn| {
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));
        assert!(txn.filetag_exists(&ftag)?);

        let ftag2 = FileTag::new(FileId::new(1), TagId::new(2), ValueId::new(2));
        txn.insert_filetag(&ftag2)?;

        // === count ===
        assert_eq!(txn.select_filetag_count()?, 2);
        assert_eq!(txn.select_filetag_count_by_fileid(FileId::new(2))?, 2);
        assert_eq!(txn.select_filetag_count_by_tagid(TagId::new(1))?, 2);
        assert_eq!(txn.select_filetag_count_by_valueid(ValueId::new(1))?, 2);

        Ok(())
    })
}

#[test]
fn filetag_copy() -> Result<()> {
    setup_dbfn_wfiletag(|txn| {
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));
        assert!(txn.filetag_exists(&ftag)?);

        let ftag2 = FileTag::new(FileId::new(1), TagId::new(2), ValueId::new(2));
        txn.insert_filetag(&ftag2)?;

        // == copying ===
        let f = txn.copy_filetags(TagId::new(1), TagId::new(2));
        assert!(f.is_ok());

        Ok(())
    })
}

// =========================== Implications ===========================
// ====================================================================

// #[test]
// fn implication_tests() -> Result<()> {
//     setup_dbfn_wfiletag(|txn| {
//         let l = "hi";
//
//         Ok(())
//     })
// }

// ================================ Tag ===============================
// ====================================================================

#[test]
fn tag_tests() -> Result<()> {
    setup_dbfn_wfiletag(|txn| {
        use colored::Color;

        let t1 = txn.insert_tag("tag1", parse_color("#FF01FF")?)?;
        assert_eq!(t1.color(), Color::truecolor(255, 1, 255));

        let t2 = txn.update_tag_name(t1.id(), "tag2")?;
        assert_ne!(t1.name(), t2.name());

        let t3 = txn.update_tag_color(t2.id(), parse_color("#01FF01")?)?;
        assert_eq!(t3.color(), Color::truecolor(1, 255, 1));

        let tags = txn.tags()?;
        assert_eq!(tags.len(), 1);

        txn.delete_tag(t3.id())?;
        assert_eq!(txn.tags()?.len(), 0);

        let t1 = txn.insert_tag("foo1", parse_color("#BBAABB")?)?;
        let t2 = txn.insert_tag("foo2", parse_color("#AABBAA")?)?;

        let info = txn.tag_information()?;
        assert!(!info.into_iter().any(|tf| tf.count() > 0));

        Ok(())
    })
}

#[test]
fn tag_query_exact() -> Result<()> {
    setup_dbfn_wfiletag(|txn| {
        let t1 = txn.insert_tag("tag1", parse_color("#FF01FF")?)?;
        let t2 = txn.insert_tag("tag2", parse_color("#01FF01")?)?;
        let t3 = txn.insert_tag("tag3", parse_color("#FFFFFF")?)?;

        let tag = txn.tag_by_name("tag1", false)?;
        assert_eq!(tag.name(), "tag1");

        let tag = txn.tag_by_name("TAG1", true)?;
        assert_eq!(tag.name(), "tag1");

        let tags = txn.tags_by_names(&["tag1", "tag2"], true)?;
        assert_eq!(tags.len(), 2);

        let tags = txn.tags_by_ids(&[1, 3].map(TagId::new))?;
        assert_eq!(tags.len(), 2);

        let tag = txn.tag(TagId::new(3))?;
        assert_eq!(tag.name(), "tag3");

        let cnt = txn.tag_count()?;
        assert_eq!(cnt, 3);

        Ok(())
    })
}

#[test]
fn tag_query_pattern() -> Result<()> {
    setup_dbfn_wfiletag(|txn| {
        let t1 = txn.insert_tag("tag1", parse_color("#FF01FF")?)?;
        let t2 = txn.insert_tag("tag2", parse_color("#01FF01")?)?;
        let t3 = txn.insert_tag("tag3", parse_color("#FFFFFF")?)?;

        let tags = txn.select_tags_by_regex("name", "ta.*")?;
        assert_eq!(tags.len(), 3);

        let tags = txn.select_tags_by_iregex("name", "TAG\\d")?;
        assert_eq!(tags.len(), 3);

        let tags = txn.select_tags_by_glob("color", "#<F:2,>*")?;
        assert_eq!(tags.len(), 2);

        let tags = txn.select_tags_by_iglob("color", "#<f:2,>*")?;
        assert_eq!(tags.len(), 2);

        Ok(())
    })
}

// ============================== Query ===============================
// ====================================================================

#[test]
fn query_tests() -> Result<()> {
    setup_dbfn_wfile(|txn| {
        let qu = txn.insert_query("select * from file;");
        let qu2 = txn.insert_query("set '*.{md,rs}' tag1");
        let qu3 = txn.insert_query("set '*.rs' rust");

        assert_eq!(txn.queries()?.len(), 3);
        assert!(txn.query("set '*.rs' rust").is_ok());

        txn.delete_query("set '*.rs' rust")?;
        assert_eq!(txn.queries()?.len(), 2);

        Ok(())
    })
}

// ============================== Value ===============================
// ====================================================================

#[test]
fn value_tests() -> Result<()> {
    setup_dbfn_wfile(|txn| {
        use itertools::Itertools;

        let val = txn.insert_value("value1")?;
        assert_eq!(val.name(), "value1");

        let val2 = txn.insert_value("vvvv")?;
        assert_eq!(txn.values()?.len(), 2);
        assert_eq!(txn.value_count()? as usize, txn.values()?.len());

        let val3 = txn.insert_value("foo")?;
        let ret = txn.update_value(val3.id(), "bar")?;
        assert!(txn.select_values_by_glob("f*").is_err());

        let val4 = txn.insert_value("zaf")?;
        txn.delete_value(val4.id())?;
        assert!(!txn.values().iter().any(|v| v.contains_name("zaf", true)));

        let vals = txn.values_by_names(&["value1", "vvvv"], true)?;
        assert_eq!(vals.len(), 2);

        // TEST:
        // let vals = txn.values_by_tagid(val2.id())?;
        // assert_eq!(vals.len(), 1);

        let val = txn.value_by_name("vvvv", true)?;
        assert_eq!(val.name(), "vvvv");

        let vals = txn.values_by_valueids(vec![val2.id(), val.id()])?;
        assert_eq!(vals.len(), 2);

        let val = txn.value(val.id())?;
        assert_eq!(val.name(), "value1");

        Ok(())
    })
}

#[test]
fn value_query() -> Result<()> {
    setup_dbfn_wfile(|txn| {
        let val = txn.insert_value("value1")?;
        let val2 = txn.insert_value("vvvv")?;
        let val3 = txn.insert_value("foo")?;

        let values = txn.select_values_by_regex("v.*")?;
        assert_eq!(values.len(), 2);

        let values = txn.select_values_by_iregex("V.*")?;
        assert_eq!(values.len(), 2);

        let values = txn.select_values_by_glob("<v:4>")?;
        assert_eq!(values.len(), 1);

        let values = txn.select_values_by_iglob("V*")?;
        assert_eq!(values.len(), 2);

        Ok(())
    })
}
