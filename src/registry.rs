// #![allow(dead_code)]

// TODO: look into using an actual database

use crate::{
    config::EncryptConfig,
    encryption::{util, InnerCtx, Plaintext, Recipients},
    filesystem::contained_path,
    opt::Opts,
    wutag_error, wutag_fatal, wutag_info,
};
use anyhow::{Context, Result};
use colored::{Color, Colorize};
use once_cell::sync::{Lazy, OnceCell};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use shellexpand::LookupError;
use wutag_core::tag::Tag;

// use rusqlite::{
//     self as rsq, params,
//     types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput},
//     Connection,
// };

use std::{
    borrow::Cow,
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

/// Name of registry file
const REGISTRY_FILE: &str = "wutag.registry";
/// Only print 'matching key info' once
static KEY_INFO: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(true));
// static KEY_INFO: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));
/// Used for the recursion of the '-x/-X' flags in the search subcommand
static ENCRYPTION: OnceCell<Result<()>> = OnceCell::new();

/// Representation of a tagged file
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub(crate) struct EntryData {
    /// Path of the file entry with tags
    path: PathBuf,
}

impl EntryData {
    /// Generate a new `EntryData` instance
    pub(crate) fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Return the path of the `EntryData` instance
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

/// Alias to `usize`, which is a hashed timestamp written to the files extended
/// attributes
pub(crate) type EntryId = usize;

/// Representation of the entire registry
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct TagRegistry {
    /// Path to the `TagRegistry`
    pub(crate) path:    PathBuf,
    /// Hash of the `Tag` name and the file id (`EntryId`) in which these tags
    /// are associated with
    pub(crate) tags:    BTreeMap<Tag, Vec<EntryId>>,
    /// Hash of the file id (`EntryId`) and the entries data (`EntryData`)
    pub(crate) entries: BTreeMap<EntryId, EntryData>,
    /* /// The connection to the database
     * pub(crate) connection: rsq::Connection, */
}

impl Default for TagRegistry {
    fn default() -> Self {
        let state_file = {
            #[cfg(target_os = "macos")]
            let data_dir_og = env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .or_else(|| dirs::home_dir().map(|d| d.join(".local").join("share")))
                .context("Invalid data directory");

            #[cfg(not(target_os = "macos"))]
            let data_dir_og = dirs::data_local_dir();

            let data_dir = data_dir_og
                .map(|p| p.join("wutag"))
                .expect("unable to join registry path");

            if !data_dir.exists() {
                fs::create_dir_all(&data_dir).unwrap_or_else(|_| {
                    wutag_fatal!(
                        "unable to create tag registry directory: {}",
                        data_dir.display()
                    )
                });
            }

            data_dir.join(REGISTRY_FILE)
        };

        Self {
            path:    state_file,
            tags:    BTreeMap::new(),
            entries: BTreeMap::new(),
        }
    }
}

impl TagRegistry {
    /// Creates a new instance of `TagRegistry` with a `path` without loading
    pub(crate) fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            ..Self::default()
        }
    }

    // /// Open the database connection
    // pub(crate) fn open_db<P: AsRef<Path>>(path: P) -> Result<Connection> {
    //     Connection::open(&path).map_err(|e| anyhow!(e))
    // }

    /// Loads a registry from the specified `path`.
    pub(crate) fn load<P: AsRef<Path>>(path: P, config: &EncryptConfig) -> Result<Self> {
        let path = path.as_ref();

        #[cfg(feature = "encrypt-gpgme")]
        if is_encrypted(path) {
            log::debug!("registry is encrypted");
            // Should only happen once
            if !config.to_encrypt {
                wutag_info!("switching to non-encrypted registry configuration");
            }

            // If it is encrypted, decrypt it to read the data
            ENCRYPTION
                .get_or_init(|| Self::crypt_registry(path, config, false))
                .as_ref()
                .map_err(|e| anyhow::anyhow!(e))?;

        // Detect -x/-X (execute) command and do not display info
        } else if config.to_encrypt
            && KEY_INFO.load(Ordering::Relaxed)
            && atty::is(atty::Stream::Stdout)
        {
            log::debug!("registry is unencrypted");
            wutag_info!("switching to encrypted registry configuration");
        }

        let data = fs::read(path).context("failed to read saved registry")?;
        serde_yaml::from_slice(&data).context("failed to deserialize tag registry")

        // serde_cbor::from_slice(&data).context("")
    }

    /// Saves the registry serialized to the path from which it was loaded.
    pub(crate) fn save(&self) -> Result<()> {
        // let serialized = serde_cbor::to_vec(&self).context("")?;
        let serialized = serde_yaml::to_vec(&self).context("failed to serialize tag registry")?;

        fs::write(&self.path, &serialized).context("failed to save registry")
    }

    /// Clears this tag registry by removing all entries and tags.
    pub(crate) fn clear(&mut self) {
        self.tags.clear();
        self.entries.clear();
    }

    /// Updates the entry or adds it if it is not present.
    pub(crate) fn add_or_update_entry(&mut self, entry: EntryData) -> EntryId {
        let pos = self
            .list_entries_and_ids()
            .find(|(_, e)| **e == entry)
            .map(|(idx, _)| *idx);

        if let Some(pos) = pos {
            let e = self
                .entries
                .get_mut(&pos)
                .unwrap_or_else(|| wutag_fatal!("failure to get immutable reference: {}", pos));
            *e = entry;
            pos
        } else {
            let timestamp = chrono::Utc::now().timestamp_nanos();
            let timestamp = if timestamp < 0 {
                timestamp.abs() as usize
            } else {
                timestamp as usize
            };
            self.entries.insert(timestamp, entry);
            timestamp
        }
    }

    fn mut_tag_entries(&mut self, tag: &Tag) -> &mut Vec<EntryId> {
        let exists = self.tags.par_iter().find_any(|(t, _)| t == &tag);

        if exists.is_none() {
            self.tags.insert(tag.clone(), Vec::new());
        }

        self.tags.get_mut(tag).unwrap()
    }

    /// Adds the `tag` to an entry with `entry` id. Returns the id if the entry
    /// was already tagged or `None` if the tag was added.
    pub(crate) fn tag_entry(&mut self, tag: &Tag, entry: EntryId) -> Option<EntryId> {
        let entries = self.mut_tag_entries(tag);

        if let Some(entry) = entries.par_iter().find_any(|&e| *e == entry) {
            return Some(*entry);
        }
        entries.push(entry);

        None
    }

    fn clean_tag_if_no_entries(&mut self, tag: &Tag) {
        let remove = if let Some(entries) = self.tags.get(tag) {
            entries.is_empty()
        } else {
            false
        };

        if remove {
            self.tags.remove(tag);
        }
    }

    /// Removes the `tag` from an entry with `entry` id. Returns the entry data
    /// if it has no tags left or `None` otherwise.
    pub(crate) fn untag_entry(&mut self, tag: &Tag, entry: EntryId) -> Option<EntryData> {
        let entries = self.mut_tag_entries(tag);

        if let Some(pos) = entries.par_iter().position_first(|e| *e == entry) {
            let entry = entries.remove(pos);

            self.clean_tag_if_no_entries(tag);

            if self.list_entry_tags(entry).is_none() {
                return self.entries.remove(&entry);
            }
        }

        None
    }

    /// Removes the tag with the `tag_name` from the `entry` returning the entry
    /// if it has no tags left or `None` otherwise.
    pub(crate) fn untag_by_name(&mut self, tag_name: &str, entry: EntryId) -> Option<EntryData> {
        let tag = self.get_tag(tag_name)?.clone();
        self.untag_entry(&tag, entry)
    }

    /// Clears all tags of the `entry`.
    pub(crate) fn clear_entry(&mut self, entry: EntryId) {
        let mut to_remove = vec![];
        self.tags.iter_mut().for_each(|(tag, entries)| {
            if let Some(idx) = entries.iter().copied().position(|e| e == entry) {
                entries.remove(idx);
            }
            if entries.is_empty() {
                to_remove.push(tag.clone());
            }
        });

        for tag in to_remove {
            self.tags.remove(&tag);
        }

        self.entries.remove(&entry);
    }

    /// Finds the entry by a `path`. Returns the id of the entry if found.
    pub(crate) fn find_entry<P: AsRef<Path>>(&self, path: P) -> Option<EntryId> {
        self.entries
            .iter()
            .find(|(_, entry)| entry.path == path.as_ref())
            .map(|(idx, _)| *idx)
    }

    /// Lists tags of the `entry` if such entry exists.
    pub(crate) fn list_entry_tags(&self, entry: EntryId) -> Option<Vec<&Tag>> {
        let tags = self
            .tags
            .iter()
            .fold(Vec::new(), |mut acc, (tag, entries)| {
                if entries.iter().any(|id| entry == *id) {
                    acc.push(tag);
                }
                acc
            });

        if tags.is_empty() {
            None
        } else {
            Some(tags)
        }
    }

    // TODO: better parsing // use or delete
    /// Check if the file entry has either tag
    #[allow(dead_code)]
    pub(crate) fn entry_has_or_tags(&self, id: EntryId, tags: &[String]) -> bool {
        let pos = tags.iter().position(|t| t == "@o" || t == "or");

        if let Some(p) = pos {
            if p == 0 || p == tags.len() - 1 {
                false
            } else {
                self.entry_has_any_tags(id, &[tags[p - 1].clone()])
                    || self.entry_has_any_tags(id, &[tags[p + 1].clone()])
            }
        } else {
            false
        }
    }

    /// Check if the file entry has all and only all specified tags
    pub(crate) fn entry_has_only_all_tags(&self, id: EntryId, tags: &[String]) -> bool {
        use std::collections::HashSet;

        let entry_tags = self.list_entry_tags(id).unwrap_or_else(Vec::new);
        let entry_hash: HashSet<String> = entry_tags.iter().map(|e| e.name().to_string()).collect();
        let inp_hash: HashSet<String> = tags.iter().cloned().collect();

        let diff: HashSet<_> = entry_hash.symmetric_difference(&inp_hash).collect();

        diff.is_empty()
    }

    /// Check if the file entry has all specific tags
    pub(crate) fn entry_has_all_tags(&self, id: EntryId, tags: &[String]) -> bool {
        let entry_tags = self.list_entry_tags(id).unwrap_or_else(Vec::new);

        // Reverse what is being checked
        tags.iter()
            .all(|t| entry_tags.iter().any(|inp| inp.name() == t))
    }

    /// Check if the file entry has any specific tags
    pub(crate) fn entry_has_any_tags(&self, id: EntryId, tags: &[String]) -> bool {
        let entry_tags = self.list_entry_tags(id).unwrap_or_else(Vec::new);

        entry_tags
            .iter()
            .any(|t| tags.iter().any(|inp| inp == t.name()))
    }

    /// Returns entries that have all of the `tags`.
    #[allow(dead_code)]
    pub(crate) fn list_entries_with_tags<T, S>(&self, tags: T) -> Vec<EntryId>
    where
        T: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut entries = tags.into_iter().fold(Vec::new(), |mut acc, tag| {
            if let Some(entries) = self
                .tags
                .iter()
                .find(|(t, _)| t.name() == tag.as_ref())
                .map(|(_, e)| e)
            {
                acc.extend_from_slice(&entries[..]);
            }
            acc
        });

        entries.dedup();

        entries
    }

    /// Return a vector of `PathBuf`'s that have a specific tag or tags
    #[allow(dead_code)]
    pub(crate) fn list_entries_paths<T, S>(
        &self,
        tags: T,
        global: bool,
        base_dir: &Path,
    ) -> Vec<PathBuf>
    where
        T: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut entries = tags
            .into_iter()
            .fold(Vec::new(), |mut acc, tag| {
                if let Some(entries) = self
                    .tags
                    .iter()
                    .find(|(t, _)| t.name() == tag.as_ref())
                    .map(|(_, e)| e)
                {
                    acc.extend_from_slice(&entries[..]);
                }
                acc
            })
            .iter()
            .fold(Vec::new(), |mut acc, id| {
                if let Some(entry) = self.get_entry(*id) {
                    if !global && !contained_path(entry.path(), base_dir) {
                    } else {
                        acc.push(PathBuf::from(entry.path()));
                    }
                }
                acc
            });
        entries.dedup();
        entries
    }

    /// Return the conceptualized data structure that all these functions
    /// correspond to. i.e., a hashmap of the file's path corresponding to a
    /// vector of the tag *names* as strings
    pub(crate) fn list_all_paths_and_tags_as_strings(&self) -> BTreeMap<PathBuf, Vec<String>> {
        let mut path_tags = BTreeMap::new();

        for (id, data) in self.list_entries_and_ids() {
            path_tags.insert(
                data.path().to_path_buf(),
                self.list_entry_tags(*id)
                    .unwrap_or_default()
                    .iter()
                    .map(|t| t.name().to_owned())
                    .collect::<Vec<_>>(),
            );
        }

        path_tags
    }

    /// Return the conceptualized data structure that all these functions
    /// correspond to. i.e., a hashmap of the file's path corresponding to a
    /// vector of the `Tag`s
    pub(crate) fn list_all_paths_and_tags(&self) -> BTreeMap<PathBuf, Vec<Tag>> {
        let mut path_tags = BTreeMap::new();

        for (id, data) in self.list_entries_and_ids() {
            path_tags.insert(
                data.path().to_path_buf(),
                self.list_entry_tags(*id)
                    .unwrap_or_default()
                    .iter()
                    .map(ToOwned::to_owned)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>(),
            );
        }

        path_tags
    }

    /// Lists ids of all entries present in the registry.
    pub(crate) fn list_entries_ids(&self) -> impl Iterator<Item = &EntryId> {
        self.entries.keys()
    }

    /// Lists data of all entries present in the registry.
    #[allow(dead_code)]
    pub(crate) fn list_entries(&self) -> impl Iterator<Item = &EntryData> {
        self.entries.values()
    }

    /// Lists ids and data of all entries present in the registry.
    pub(crate) fn list_entries_and_ids(&self) -> impl Iterator<Item = (&EntryId, &EntryData)> {
        self.entries.iter()
    }

    /// Lists available tags.
    pub(crate) fn list_tags(&self) -> impl Iterator<Item = &Tag> {
        self.tags.keys()
    }

    /// Returns data of the entry with `id` if such entry exists.
    pub(crate) fn get_entry(&self, id: EntryId) -> Option<&EntryData> {
        self.entries.get(&id)
    }

    /// Returns the tag with the name `tag` if it exists.
    pub(crate) fn get_tag<T: AsRef<str>>(&self, tag: T) -> Option<&Tag> {
        self.tags.keys().find(|t| t.name() == tag.as_ref())
    }

    /// Updates the color of the `tag`. Returns `true` if the tag was found and
    /// updated and `false` otherwise.
    pub(crate) fn update_tag_color<T: AsRef<str>>(&mut self, tag: T, color: Color) -> bool {
        if let Some(mut t) = self.tags.keys().find(|t| t.name() == tag.as_ref()).cloned() {
            let data = self
                .tags
                .remove(&t)
                .unwrap_or_else(|| wutag_fatal!("failure to remove tag: {}", t));

            t.set_color(&color);
            self.tags.insert(t, data);
            true
        } else {
            false
        }
    }

    /// Update / rename the name of the tag
    pub(crate) fn update_tag_name<T: AsRef<str>>(&mut self, tag: T, rename: T) -> bool {
        if let Some(mut t) = self.tags.keys().find(|t| t.name() == tag.as_ref()).cloned() {
            let data = self
                .tags
                .remove(&t)
                .unwrap_or_else(|| wutag_fatal!("failure to remove tag: {}", t));

            t.set_name(&rename);
            self.tags.insert(t, data);
            true
        } else {
            false
        }
    }

    /// Encrypt or decrypt the registry
    #[cfg(feature = "encrypt-gpgme")]
    pub(crate) fn crypt_registry<P: AsRef<Path>>(
        path: P,
        config: &EncryptConfig,
        encrypt: bool,
    ) -> Result<()> {
        let path = path.as_ref();
        if let Some(public) = config.public_key.clone() {
            let public = public
                .trim()
                .strip_prefix("0x")
                .unwrap_or_else(|| public.trim())
                .to_uppercase();

            let mut ctx =
                util::context(config.tty).context("failure to get cryptography context")?;
            let all_recipients =
                Recipients::from(ctx.keys_private().context("no private keys were found")?);

            let fatal_fingerprint = || -> ! {
                wutag_fatal!(
                    r#"database encryption/decryption failure.
Available keys are:
{}
Use an (1) email, (2) short fingerprint, or (3) full fingerprint"#,
                    all_recipients
                        .keys()
                        .iter()
                        .fold(String::new(), |mut acc, key| {
                            acc.push_str(&format!("\t{} {}\n", "+".red().bold(), key));
                            acc
                        })
                )
            };

            // (1) E93ACCAAAEB024788C106EDEC011CBEF6628B679
            // (2) C011CBEF6628B679
            // (3) lmb@lmburns.com
            if let Some(found) = all_recipients.keys().iter().find(|key| {
                public == key.fingerprint(false)
                    || public == key.fingerprint(true)
                    || ctx.user_emails().iter().any(|emails| {
                        emails
                            .iter()
                            .any(|email| email.trim().to_uppercase() == public)
                    })
            }) {
                // Run this only once since it will be ran be encrypting it back as well
                if KEY_INFO.load(Ordering::Relaxed) {
                    log::info!("found matching key: {}", found);
                    KEY_INFO.store(false, Ordering::Relaxed);
                }
                // KEY_INFO.get_or_init(|| log::info!("found matching key: {}", found));

                // ## If the content is encrypted
                if is_encrypted(path) && !encrypt {
                    log::debug!("decrypting registry");

                    // 1. Decrypt file
                    let plaintext = ctx
                        .decrypt_file(path)
                        .context("failure to decrypt registry")?;

                    // 2. Serialize the decrypted string to a registry
                    let yaml: TagRegistry = serde_yaml::from_slice(plaintext.unsecure_ref())
                        .context("failure to convert decrypted registry to TagRegistry")?;

                    // 3. Write the serialized structure to a file
                    fs::write(path, &serde_yaml::to_vec(&yaml)?)
                        .context("failed to save registry")?;

                    // self.encrypted = false;
                } else if encrypt {
                    // ## If the content is not encrypted

                    // 1. Serialize the unencrypted string to a registry
                    let yaml: TagRegistry = serde_yaml::from_slice(
                        &fs::read(path).context("failed to read registry file")?,
                    )
                    .context("encrypted file is invalid UTF-8")?;

                    // 2. Convert it to a structure that can be encrypted
                    let plaintext = Plaintext::from(serde_yaml::to_string(&yaml)?);

                    log::debug!("encrypting registry");

                    // 3. Encrypt and write the file
                    ctx.encrypt_file(&Recipients::from(vec![found.clone()]), plaintext, path)
                        .context("failure to encrypt registry")?;

                    // self.encrypted = true;
                } else {
                    fatal_fingerprint();
                }
            }
        } else {
            wutag_fatal!("you want to encrypt the database but provided no key");
        }

        Ok(())
    }
}

#[cfg(feature = "encrypt-gpgme")]
pub(crate) fn is_encrypted<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();

    if !path.exists() {
        return false;
    }

    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| wutag_fatal!("failure to read registry file to string"));

    content.contains("-----BEGIN PGP MESSAGE-----") && content.contains("-----END PGP MESSAGE-----")
}

/// Load the `TagRegistry`
pub(crate) fn load_registry(opts: &Opts, config: &EncryptConfig) -> Result<TagRegistry> {
    // Default location of registry
    let def_registry = TagRegistry::default();
    let state_file = def_registry.path;

    let registry = if let Some(opt_reg) = &opts.reg {
        // Expand both tlide '~' and environment variables in 'WUTAG_REGISTRY' env var
        let registry = &PathBuf::from(
            shellexpand::full(&opt_reg.display().to_string())
                .unwrap_or_else(|_| {
                    Cow::from(
                        LookupError {
                            var_name: "Unkown environment variable".into(),
                            cause:    env::VarError::NotPresent,
                        }
                        .to_string(),
                    )
                })
                .to_string(),
        );

        if registry.is_file() && registry.file_name().is_some() {
            log::debug!("using a non-default registry: {}", registry.display());
            TagRegistry::load(&registry, config).unwrap_or_else(|_| TagRegistry::new(&registry))
            //\\
        } else if registry.is_dir() && registry.file_name().is_some() {
            wutag_error!(
                "{} is not a file. Using default registry: {}",
                registry.display().to_string().green(),
                state_file.display().to_string().green(),
            );
            TagRegistry::load(&state_file, config).unwrap_or_else(|_| TagRegistry::new(&state_file))
            //\\
        } else if registry.display().to_string().ends_with('/') {
            wutag_error!(
                "{} last error is a directory path. Using default registry: {}",
                registry.display().to_string().green(),
                state_file.display().to_string().green(),
            );
            TagRegistry::load(&state_file, config).unwrap_or_else(|_| TagRegistry::new(&state_file))
            //\\
        } else {
            log::debug!("using a non-default registry: {}", registry.display());
            fs::create_dir_all(
                &registry
                    .parent()
                    .context("Could not get parent of nonexisting path")?,
            )
            .with_context(|| {
                format!(
                    "unable to create registry directory: {}",
                    registry.display()
                )
            })?;

            TagRegistry::load(&registry, config).unwrap_or_else(|_| {
                log::debug!("creating a non-default registry");
                TagRegistry::new(&registry)
            })
        }
    } else {
        log::debug!("using default registry");
        TagRegistry::load(&state_file, config).unwrap_or_else(|_| {
            log::debug!("creating default registry");
            TagRegistry::new(&state_file)
        })
    };

    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::DEFAULT_COLORS;
    use colored::Color::{Black, Red};

    #[test]
    fn adds_and_tags_entry() {
        let path = PathBuf::from("/tmp");
        let entry = EntryData::new(path.clone());
        let mut registry = TagRegistry::default();
        registry.add_or_update_entry(entry.clone());
        let id = registry.find_entry(&path).unwrap();

        let entry_ = registry.get_entry(id).unwrap();
        assert_eq!(entry_.path, entry.path);

        let tag = Tag::random("test", DEFAULT_COLORS);
        let second = Tag::random("second", DEFAULT_COLORS);

        assert_eq!(registry.tag_entry(&tag, id), None);
        assert_eq!(registry.list_entry_tags(id), Some(vec![&tag]));
        assert_eq!(registry.tag_entry(&second, id), None);
        assert!(registry.list_entry_tags(id).unwrap().contains(&&tag));
        assert!(registry.list_entry_tags(id).unwrap().contains(&&second));
        assert_eq!(registry.untag_entry(&tag, id), None);
        assert_eq!(registry.list_entry_tags(id), Some(vec![&second]));
        assert_eq!(registry.untag_entry(&tag, id), None);
        assert_eq!(registry.untag_entry(&second, id), Some(entry));
        assert_eq!(registry.list_entry_tags(id), None);
    }

    #[test]
    fn adds_multiple_entries() {
        let mut registry = TagRegistry::default();

        let entry = EntryData::new("/tmp");
        let fst_id = registry.add_or_update_entry(entry.clone());
        let snd_entry = EntryData::new("/tmp/123");
        let snd_id = registry.add_or_update_entry(snd_entry.clone());

        assert_eq!(registry.list_entries().count(), 2);

        let entries: Vec<_> = registry.list_entries_and_ids().collect();
        assert!(entries.contains(&(&fst_id, &entry)));
        assert!(entries.contains(&(&snd_id, &snd_entry)));
    }

    #[test]
    fn updates_tag_color() {
        let entry = EntryData::new("/tmp");

        let mut registry = TagRegistry::default();
        let id = registry.add_or_update_entry(entry);

        let tag = Tag::new("test", Black);

        assert!(registry.tag_entry(&tag, id).is_none());
        assert!(registry.update_tag_color("test", Red));
        assert_eq!(registry.list_tags().next().unwrap().color(), &Red);
    }

    #[test]
    fn removes_an_entry_when_no_tags_left() {
        let entry = EntryData::new("/tmp");

        let mut registry = TagRegistry::default();
        let id = registry.add_or_update_entry(entry.clone());

        let tag1 = Tag::new("test", Black);
        let tag2 = Tag::new("test2", Red);

        assert!(registry.tag_entry(&tag1, id).is_none());
        assert_eq!(registry.tags.iter().next(), Some((&tag1, &vec![id])));
        assert_eq!(registry.list_entries().count(), 1);
        assert_eq!(registry.untag_entry(&tag1, id), Some(entry.clone()));
        assert_eq!(registry.list_entries().count(), 0);
        assert!(registry.tags.is_empty());

        let id = registry.add_or_update_entry(entry.clone());
        assert!(registry.tag_entry(&tag2, id).is_none());
        assert_eq!(registry.tags.iter().next(), Some((&tag2, &vec![id])));
        assert_eq!(registry.list_entries().count(), 1);
        assert_eq!(registry.untag_by_name(tag2.name(), id), Some(entry.clone()));
        assert_eq!(registry.list_entries().count(), 0);
        assert!(registry.tags.is_empty());

        let id = registry.add_or_update_entry(entry);
        assert!(registry.tag_entry(&tag1, id).is_none());
        assert!(registry.tag_entry(&tag2, id).is_none());
        let tags: Vec<_> = registry.tags.iter().collect();
        assert!(tags.contains(&(&tag1, &vec![id])));
        assert!(tags.contains(&(&tag2, &vec![id])));
        assert_eq!(registry.list_entries().count(), 1);
        registry.clear_entry(id);
        assert_eq!(registry.list_entries().count(), 0);
        assert!(registry.tags.is_empty());
    }

    #[test]
    fn lists_entry_tags() {
        let mut registry = TagRegistry::default();

        let tag1 = Tag::new("src", Black);
        let tag2 = Tag::new("code", Red);

        let entry = EntryData::new("/tmp");

        let id = registry.add_or_update_entry(entry);
        registry.tag_entry(&tag1, id);
        registry.tag_entry(&tag2, id);

        let tags = registry.list_entry_tags(id).unwrap();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&&tag1));
        assert!(tags.contains(&&tag2));
    }

    #[test]
    fn lists_entries_with_tags() {
        let mut registry = TagRegistry::default();

        let tag1 = Tag::new("src", Black);
        let tag2 = Tag::new("code", Red);

        let entry = EntryData::new("/tmp");
        let entry1 = EntryData::new("/tmp/1");
        let entry2 = EntryData::new("/tmp/2");
        let entry3 = EntryData::new("/tmp/3");

        let id = registry.add_or_update_entry(entry);
        let id1 = registry.add_or_update_entry(entry1);
        let id2 = registry.add_or_update_entry(entry2);
        let id3 = registry.add_or_update_entry(entry3);

        registry.tag_entry(&tag1, id);
        registry.tag_entry(&tag1, id2);

        registry.tag_entry(&tag2, id1);
        registry.tag_entry(&tag2, id3);

        let entries1 = registry.list_entries_with_tags(vec![tag1.name()]);
        assert_eq!(entries1.len(), 2);
        assert!(entries1.contains(&id));
        assert!(entries1.contains(&id2));

        let entries2 = registry.list_entries_with_tags(vec![tag2.name()]);
        assert_eq!(entries2.len(), 2);
        assert!(entries2.contains(&id1));
        assert!(entries2.contains(&id3));

        let entries = registry.list_entries_with_tags(vec![tag2.name(), tag1.name()]);
        assert_eq!(entries.len(), 4);
        assert!(entries.contains(&id));
        assert!(entries.contains(&id1));
        assert!(entries.contains(&id2));
        assert!(entries.contains(&id3));
    }

    #[test]
    fn saves_and_loads() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let registry_path = tmp_dir.path().join("wutag.registry");

        let mut registry = TagRegistry::new(&registry_path);

        let tag = Tag::new("src", Black);
        let entry = EntryData::new("/tmp");

        let id = registry.add_or_update_entry(entry.clone());
        registry.tag_entry(&tag, id);

        registry.save().unwrap();

        let registry = TagRegistry::load(registry_path, &EncryptConfig::default()).unwrap();
        let mut entries = registry.list_entries_and_ids();
        let (got_id, got_entry) = entries.next().unwrap();
        assert!(entries.next().is_none());
        assert_eq!(got_id, &id);
        assert_eq!(got_entry, &entry);
        assert_eq!(registry.list_entries_with_tags(vec![tag.name()]), vec![id]);
    }
}
