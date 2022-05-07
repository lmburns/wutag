//! Tests used to check the API calls

use super::super::{
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
    let reg = Registry::new(Some(&path), false)?;
    reg.init()?;

    Ok(reg)
}

fn setup_dbfn<F>(mut f: F) -> Result<()>
where
    F: FnMut(&Registry) -> Result<()>,
{
    // Delete the file when going out of scope
    scopeguard::defer!(wfs::delete_file(DB_NAME));

    // Delete the file now if it exists
    // wfs::delete_file(DB_NAME);

    let reg = setup_db()?;
    f(&reg);

    Ok(())
}

fn setup_dbfn_wfile<F>(mut f: F) -> Result<()>
where
    F: FnMut(&Registry) -> Result<()>,
{
    setup_dbfn(|reg| {
        let test = PathBuf::from("./README.md");
        let ret = reg.insert_file(&test)?;

        f(reg);

        Ok(())
    })
}

#[test]
fn reg_insert_file() -> Result<()> {
    setup_dbfn(|reg| {
        let test = PathBuf::from("./README.md");

        let ret = reg.insert_file(test)?;

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
fn reg_glob_matching() -> Result<()> {
    setup_dbfn_wfile(|reg| {
        let files = reg.files_by_glob_generic("name", "*.md")?;
        first_idx!(files);

        let files = reg.files_by_iglob_generic("name", "*.MD")?;
        first_idx!(files);

        let files = reg.files_by_glob_fp("**/*.md")?;
        first_idx!(files);

        let files = reg.files_by_iglob_fp("**/*.md")?;
        first_idx!(files);

        let files = reg.files_by_glob_fname("*.md")?;
        first_idx!(files);

        let files = reg.files_by_iglob_fname("*.MD")?;
        first_idx!(files);

        Ok(())
    })
}

#[test]
fn reg_regex_matching() -> Result<()> {
    setup_dbfn_wfile(|reg| {
        let files = reg.files_by_regex_generic("name", "READ.*")?;
        first_idx!(files);

        let files = reg.files_by_iregex_generic("name", "rEaD.*")?;
        first_idx!(files);

        let files = reg.files_by_regex_fp(".*.md")?;
        first_idx!(files);

        let files = reg.files_by_iregex_fp(".*.md")?;
        first_idx!(files);

        let files = reg.files_by_regex_fname(".*.md")?;
        first_idx!(files);

        let files = reg.files_by_iregex_fname(".*.MD")?;
        first_idx!(files);

        Ok(())
    })
}

#[test]
fn reg_select_files_by() -> Result<()> {
    setup_dbfn_wfile(|reg| {
        let mfile = PathBuf::from("./README.md");
        let meta = mfile.metadata()?;
        let cwd = env::current_dir()?.display().to_string();

        let files = reg.files(None)?;
        first_idx!(files);

        // === ID ===
        let file = reg.file(FileId::new(0))?;
        assert_eq!(file.name(), "README.md");

        // === path ===
        let files = reg.file_by_path(format!("{}/README.md", cwd))?;
        assert_eq!(files.name(), "README.md");

        // === directory ===
        let files = reg.files_by_directory(&cwd)?;
        first_idx!(files);

        let files = reg.files_by_directories(&[cwd])?;
        first_idx!(files);

        let files = reg.directories()?;
        assert!(files.is_empty());

        // === hash ===
        let files =
            reg.files_by_hash(hash::blake3_hash(mfile.display().to_string(), None)?.to_string())?;
        first_idx!(files);

        // === Mime ===
        let files = reg.files_by_mime("text/markdown")?;
        first_idx!(files);

        // === mtime ===
        let files = reg.files_by_mtime(format!("{}", meta.mtime()))?;
        first_idx!(files);

        // === ctime ===
        let files = reg.files_by_ctime(format!("{}", meta.ctime()))?;
        first_idx!(files);

        // === mode ===
        let files = reg.files_by_mode("644")?;
        first_idx!(files);
        let files = reg.files_by_mode("0644")?;
        first_idx!(files);
        let files = reg.files_by_mode("100644")?;
        first_idx!(files);

        // === inode ===
        let files = reg.files_by_inode(meta.ino())?;
        first_idx!(files);

        // === links ===
        let files = reg.files_by_links(meta.nlink())?;
        first_idx!(files);

        // === uid ===
        let files = reg.files_by_uid(meta.uid().into())?;
        first_idx!(files);

        // === gid ===
        let files = reg.files_by_gid(meta.gid().into())?;
        first_idx!(files);

        // === size ===
        let files = reg.files_by_size(meta.size())?;
        first_idx!(files);

        // === e2pflags ===
        #[cfg(all(
            feature = "file-flags",
            target_family = "unix",
            not(target_os = "macos")
        ))]
        {
            let files = reg.files_by_flags("e")?;
            first_idx!(files);

            let files = reg.files_by_flags("a")?;
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
fn reg_file_flags() -> Result<()> {
    use e2p_fileflags::{FileFlags, Flags};

    setup_dbfn_wfile(|reg| {
        let mfile = PathBuf::from("./README.md");
        let flags = mfile.flags()?;

        let og_files = reg.files_by_flags("e")?;
        first_idx!(og_files);

        mfile.set_flags(Flags::COMPR | flags)?;

        let new = reg.update_file(
            og_files.get(0).context("failed to get first idx")?.id,
            &mfile,
        )?;

        let new_files = reg.files_by_flags("e")?;
        assert!(og_files != new_files);

        let selmore = reg.files_by_flags("ec")?;
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
fn reg_file_counts() -> Result<()> {
    setup_dbfn_wfile(|reg| {
        let c = reg.file_count()?;
        assert_eq!(c, 1);

        let mfile = PathBuf::from("./README.md");
        let count = reg.file_count_by_hash(
            hash::blake3_hash(mfile.display().to_string(), None)?.to_string(),
        )?;
        assert_eq!(count, 1);

        Ok(())
    })
}

#[test]
fn reg_file_delete() -> Result<()> {
    setup_dbfn_wfile(|reg| {
        let pb = PathBuf::from("./README.md").canonicalize()?;
        let f = reg.file_by_path(&pb)?;
        reg.delete_file(f.id)?;

        assert!(reg.file_by_path(pb).is_err());

        Ok(())
    })
}

// ============================= Filetag ==============================
// ====================================================================

fn setup_dbfn_wfiletag<F>(mut f: F) -> Result<()>
where
    F: FnMut(&Registry) -> Result<()>,
{
    setup_dbfn(|reg| {
        let test = PathBuf::from("./README.md");
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));

        reg.insert_file(test)?;
        reg.insert_filetag(&ftag)?;

        f(reg);

        Ok(())
    })
}

#[test]
fn reg_delete_filetag() -> Result<()> {
    setup_dbfn_wfiletag(|reg| {
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));
        reg.delete_filetag(&ftag)?;

        let f2 = FileTag::new(2.into(), 1.into(), 2.into());

        reg.insert_filetag(&f2)?;
        assert!(reg.filetag_exists(&f2)?);
        reg.delete_filetag_by_fileid(2.into())?;
        assert!(!reg.filetag_exists(&f2)?);

        reg.insert_filetag(&f2)?;
        assert!(reg.filetag_exists(&f2)?);
        reg.delete_filetag_by_tagid(1.into())?;
        assert!(!reg.filetag_exists(&f2)?);

        reg.insert_filetag(&f2)?;
        assert!(reg.filetag_exists(&f2)?);
        reg.delete_filetag_by_valueid(2.into())?;
        assert!(!reg.filetag_exists(&f2)?);

        Ok(())
    })
}

#[test]
fn reg_filetag_select() -> Result<()> {
    setup_dbfn_wfiletag(|reg| {
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));

        assert!(reg.filetag_exists(&ftag)?);
        assert_eq!(reg.filetag_count()?, 1);

        let new = reg.filetags()?;
        assert_eq!(*get_0!(new), ftag);

        let ftag2 = FileTag::new(FileId::new(1), TagId::new(2), ValueId::new(2));
        reg.insert_filetag(&ftag2)?;

        let f = reg.filetags_by_fileid(FileId::new(2), false)?;
        assert_eq!(*get_0!(f), ftag);

        let f = reg.filetags_by_tagid(TagId::new(1), false)?;
        assert_eq!(*get_0!(f), ftag);

        let f = reg.filetags_by_valueid(ValueId::new(1))?;
        assert_eq!(*get_0!(f), ftag);

        Ok(())
    })
}

#[test]
fn reg_filetag_count() -> Result<()> {
    setup_dbfn_wfiletag(|reg| {
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));
        assert!(reg.filetag_exists(&ftag)?);

        let ftag2 = FileTag::new(FileId::new(1), TagId::new(2), ValueId::new(2));
        reg.insert_filetag(&ftag2)?;

        // === count ===
        assert_eq!(reg.filetag_count()?, 2);
        assert_eq!(reg.filetag_count_by_fileid(FileId::new(2), false)?, 2);
        assert_eq!(reg.filetag_count_by_tagid(TagId::new(1), false)?, 2);
        assert_eq!(reg.filetag_count_by_valueid(ValueId::new(1))?, 2);

        Ok(())
    })
}

#[test]
fn reg_filetag_copy() -> Result<()> {
    setup_dbfn_wfiletag(|reg| {
        let ftag = FileTag::new(FileId::new(1), TagId::new(1), ValueId::new(1));
        assert!(reg.filetag_exists(&ftag)?);

        let ftag2 = FileTag::new(FileId::new(1), TagId::new(2), ValueId::new(2));
        reg.insert_filetag(&ftag2)?;

        // == copying ===
        let f = reg.copy_filetags(TagId::new(1), TagId::new(2));
        assert!(f.is_ok());

        Ok(())
    })
}

// =========================== Implications ===========================
// ====================================================================

// #[test]
// fn reg_implication_tests() -> Result<()> {
//     setup_dbfn_wfiletag(|txn| {
//         let l = "hi";
//
//         Ok(())
//     })
// }

// ================================ Tag ===============================
// ====================================================================

#[test]
fn reg_tag_tests() -> Result<()> {
    setup_dbfn_wfiletag(|reg| {
        use colored::Color;

        let t1 = reg.insert_tag(&Tag::new_noid("tag1", "#FF01FF"))?;
        assert_eq!(t1.color(), Color::truecolor(255, 1, 255));

        let t2 = reg.update_tag_name(t1.id(), "tag2")?;
        assert_ne!(t1.name(), t2.name());

        let t3 = reg.update_tag_color(t2.id(), "#01FF01")?;
        assert_eq!(t3.color(), Color::truecolor(1, 255, 1));

        let tags = reg.tags()?;
        assert_eq!(tags.len(), 1);

        reg.delete_tag(t3.id())?;
        assert_eq!(reg.tags()?.len(), 0);

        let t1 = reg.insert_tag(&Tag::new_noid("foo1", "#BBAABB"))?;
        let t2 = reg.insert_tag(&Tag::new_noid("foo2", "#AABBAA"))?;

        let info = reg.tag_info()?;
        assert!(!info.into_iter().any(|tf| tf.count() > 0));

        Ok(())
    })
}

#[test]
fn reg_tag_query_exact() -> Result<()> {
    setup_dbfn_wfiletag(|reg| {
        let t1 = reg.insert_tag(&Tag::new_noid("tag1", "#FF01FF"))?;
        let t2 = reg.insert_tag(&Tag::new_noid("tag2", "#01FF01"))?;
        let t3 = reg.insert_tag(&Tag::new_noid("tag3", "#FFFFFF"))?;

        let tag = reg.tag_by_name("tag1")?;
        assert_eq!(tag.name(), "tag1");

        let tag = reg.tag_by_name("TAG1")?;
        assert_eq!(tag.name(), "tag1");

        let tags = reg.tags_by_names(&["tag1", "tag2"])?;
        assert_eq!(tags.len(), 2);

        let tags = reg.tags_by_ids(&[1, 3].map(TagId::new).to_vec().into())?;
        assert_eq!(tags.len(), 2);

        let tag = reg.tag(TagId::new(3))?;
        assert_eq!(tag.name(), "tag3");

        let cnt = reg.tag_count()?;
        assert_eq!(cnt, 3);

        Ok(())
    })
}

#[test]
fn reg_tag_query_pattern() -> Result<()> {
    setup_dbfn_wfiletag(|reg| {
        let t1 = reg.insert_tag(&Tag::new_noid("tag1", "#FF01FF"))?;
        let t2 = reg.insert_tag(&Tag::new_noid("tag2", "#01FF01"))?;
        let t3 = reg.insert_tag(&Tag::new_noid("tag3", "#FFFFFF"))?;

        let tags = reg.tags_by_pcre_name("tag\\d")?;
        assert_eq!(tags.len(), 3);

        let tags = reg.tags_by_regex_name("ta.*")?;
        assert_eq!(tags.len(), 3);

        let tags = reg.tags_by_iregex_name("TAG\\d")?;
        assert_eq!(tags.len(), 3);

        let tags = reg.tags_by_glob_name("#<F:2,>*")?;
        assert_eq!(tags.len(), 2);

        let tags = reg.tags_by_iglob_name("#<f:2,>*")?;
        assert_eq!(tags.len(), 2);

        let tags = reg.tags_by_regex_color(".*FF.*")?;
        assert_eq!(tags.len(), 3);

        let tags = reg.tags_by_pcre_color(".*(FF).*\\1")?;
        assert_eq!(tags.len(), 2);

        Ok(())
    })
}

// ============================== Query ===============================
// ====================================================================

// #[test]
// fn reg_query_tests() -> Result<()> {
//     setup_dbfn_wfile(|reg| {
//         let qu = reg.insert_query("select * from file;");
//         let qu2 = reg.insert_query("set '*.{md,rs}' tag1");
//         let qu3 = reg.insert_query("set '*.rs' rust");
//
//         assert_eq!(reg.queries()?.len(), 3);
//         assert!(reg.query("set '*.rs' rust").is_ok());
//
//         reg.delete_query("set '*.rs' rust")?;
//         assert_eq!(reg.queries()?.len(), 2);
//
//         Ok(())
//     })
// }

// ============================== Value ===============================
// ====================================================================

#[test]
fn reg_value_tests() -> Result<()> {
    setup_dbfn_wfile(|reg| {
        use itertools::Itertools;

        let val = reg.insert_value("value1")?;
        assert_eq!(val.name(), "value1");

        let val2 = reg.insert_value("vvvv")?;
        assert_eq!(reg.values()?.len(), 2);
        assert_eq!(reg.value_count()? as usize, reg.values()?.len());

        let val3 = reg.insert_value("foo")?;
        let ret = reg.update_value(val3.id(), "bar")?;
        assert!(reg.values_by_glob("f*").is_err());

        let val4 = reg.insert_value("zaf")?;
        reg.delete_value(val4.id())?;
        assert!(!reg.values().iter().any(|v| v.contains_name("zaf", true)));

        let vals = reg.values_by_names(&["value1", "vvvv"], true)?;
        assert_eq!(vals.len(), 2);

        // TEST:
        // let vals = reg.values_by_tagid(val2.id())?;
        // assert_eq!(vals.len(), 1);

        let val = reg.value_by_name("vvvv", true)?;
        assert_eq!(val.name(), "vvvv");

        let vals = reg.values_by_valueids(&[val2.id(), val.id()])?;
        assert_eq!(vals.len(), 2);

        let val = reg.value(val.id())?;
        assert_eq!(val.name(), "value1");

        Ok(())
    })
}

#[test]
fn reg_value_query() -> Result<()> {
    setup_dbfn_wfile(|reg| {
        let val = reg.insert_value("value1")?;
        let val2 = reg.insert_value("vvvv")?;
        let val3 = reg.insert_value("foo")?;

        let values = reg.values_by_pcre("((v)\\2){2}")?;
        assert_eq!(values.len(), 1);

        let values = reg.values_by_regex("v.*")?;
        assert_eq!(values.len(), 2);

        let values = reg.values_by_iregex("V.*")?;
        assert_eq!(values.len(), 2);

        let values = reg.values_by_glob("<v:4>")?;
        assert_eq!(values.len(), 1);

        let values = reg.values_by_iglob("V*")?;
        assert_eq!(values.len(), 2);

        Ok(())
    })
}
