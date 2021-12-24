//! File-hashing operations

use crate::config::Config;
use anyhow::{Context, Result};
use crossbeam_channel as channel;
use crossbeam_utils::thread;
use ignore::WalkBuilder;
use std::{
    fs, io,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    sync::{Arc, Mutex},
};

/// Use the [`blake3`] hashing function on text
pub(crate) fn blake3_hash_text<S: AsRef<str>>(txt: S) -> String {
    blake3::hash(txt.as_ref().as_bytes()).to_string()
}

/// Use the [`blake3`] hashing function on a file's contents
pub(crate) fn blake3_hash<P: AsRef<Path>>(path: P, perm: u32) -> Result<blake3::Hash> {
    let path = path.as_ref();

    let mut file =
        fs::File::open(&path).context(format!("failed to open file: {}", path.display()))?;
    let mut hasher = blake3::Hasher::new();

    // Hash the file's contents
    io::copy(&mut file, &mut hasher).context("failed to copy file to hasher")?;
    // Add the file's permissions to the hash
    hasher.update(&perm.to_be_bytes());

    Ok(hasher.finalize())
}

/// Use the [`blake3`](blake3) hashing function to get the hash of an entire
/// directory
pub(crate) fn hash_dir<P, F>(follow_links: bool, dir: P, f: F) -> Result<blake3::Hash>
where
    P: AsRef<Path>,
    F: Fn(&Path, u32) -> Result<blake3::Hash> + Send + Sync,
{
    let mut walker = WalkBuilder::new(&dir.as_ref());
    walker
        .threads(num_cpus::get())
        .follow_links(follow_links)
        .hidden(false)
        .ignore(false)
        .git_global(false)
        .git_ignore(false)
        .git_exclude(false)
        .parents(false)
        .max_depth(Some(1));

    let build = walker.build_parallel();
    let mut hashes = Arc::new(Mutex::new(vec![]));

    thread::scope(|scope| {
        let (tx, rx) = channel::unbounded::<ignore::DirEntry>();

        scope.spawn(|_| {
            let rx = rx;
            let hashes = Arc::clone(&hashes);
            let mut h = hashes.lock().expect("failed to lock hashes");

            // Else's here are unnecessary, but it'd be good to log the errors
            while let Ok(entry) = rx.recv() {
                let path = entry.path();
                if let Ok(meta) = entry.metadata() {
                    let mode = meta.permissions().mode();
                    if let Ok(hash) = f(path, mode) {
                        h.push(hash.to_string());
                    } else {
                        log::error!("unable to calculate hash hash: {}", path.display());
                    }
                } else {
                    log::error!("unable to get metadata: {}", path.display());
                }
            }
        });

        scope.spawn(|_| {
            let tx = tx;
            build.run(|| {
                let tx = tx.clone();
                Box::new(move |res| {
                    match res {
                        Ok(entry) =>
                            if let Err(e) = tx.send(entry) {
                                return ignore::WalkState::Quit;
                            },
                        Err(err) => {
                            log::error!("unable to access entry {}", err);
                        },
                    }

                    ignore::WalkState::Continue
                })
            });
        });
    });

    let hashes = Arc::try_unwrap(hashes)
        .expect("failed to unwrap Arc")
        .into_inner()
        .context("failed to get inner Mutex")?;

    let hashstr = hashes.join("\0");
    let mut hasher = blake3::Hasher::new();
    hasher.update(hashstr.as_bytes());

    Ok(hasher.finalize())
}
