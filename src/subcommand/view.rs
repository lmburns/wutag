// TODO: Prevent clearing of file beforehand to keep previous tag colors

use super::App;
use crate::{
    bold_entry,
    consts::DEFAULT_EDITOR,
    filesystem::{contained_path, create_temp_path, osstr_to_bytes},
    oregistry::EntryData,
    regex,
    utils::{crawler, fmt, glob_builder, regex_builder},
    wutag_error, wutag_fatal, wutag_info,
    xattr::tag_old::{clear_tags, DirEntryExt, Tag},
};
use anyhow::Result;
use clap::Args;
use colored::Colorize;
use lexiclean::Lexiclean;
use regex::Captures;
use std::{
    borrow::Cow, collections::BTreeMap, ffi::OsStr, fs, io::Write, path::PathBuf, process,
    sync::Arc,
};

#[derive(Args, Debug, Clone, PartialEq, Default)]
pub(crate) struct ViewOpts {
    /// Open tags in selected edtor (use only with vi, vim, neovim)
    #[clap(
        long, short,
        env = "EDITOR",
        default_value = DEFAULT_EDITOR.as_ref(),
        value_name = "editor",
        hide_env = true,
        hide_default_value = true,
    )]
    pub(crate) editor: String,

    /// View all tags
    #[clap(long, short = 'a')]
    pub(crate) all: bool,

    /// Format of file to view results (toml, yaml, json)
    #[clap(
        name = "format",
        long, short = 'f',
        possible_values = &["toml", "yaml", "yml", "json"],
        hide_possible_values = true,
        long_help = "\
            Format of the file viewed in the editor with the matching search results.\
            The possible values are: 'toml', 'yaml|yml', 'json'.",
    )]
    pub(crate) format: Option<String>,

    /// Search with a tag as a filter
    #[clap(
        name = "tags",
        long,
        short,
        long_help = "\
            Limit search results even further by using a tag as a filter. Can search just for tags \
                     by not using '--pattern'"
    )]
    pub(crate) tags:    Vec<String>,
    /// Pattern to search for and open result in editor
    #[clap(
        name = "pattern",
        long,
        short = 'p',
        long_help = "\
        This pattern is optional. If no pattern is given, all files that have a tag will be shown \
                     in the editor. Otherwise, the results that match the pattern will be shown."
    )]
    pub(crate) pattern: Option<String>,
}

impl App {
    /// View tags within an `$EDITOR`
    pub(crate) fn view(&mut self, opts: &ViewOpts) -> Result<()> {
        log::debug!("ViewOpts: {:#?}", opts);

        let pat = if let Some(pattern) = &opts.pattern {
            if self.pat_regex {
                String::from(pattern)
            } else if self.fixed_string {
                regex::escape(pattern)
            } else {
                glob_builder(pattern, self.wildcard_matches_sep)
            }
        } else {
            glob_builder("*", self.wildcard_matches_sep)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        let exclude_pattern = regex_builder(
            self.exclude.join("|").as_str(),
            self.case_insensitive,
            self.case_sensitive,
        );

        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();

        if opts.all {
            crawler(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    map.insert(
                        tern::t!(
                            self.global
                            ? entry.path().display().to_string()
                            : self.fmt_raw_local_path(entry.path())
                        ),
                        match entry.has_tags() {
                            Ok(has_tags) =>
                                if has_tags {
                                    if let Some(id) = self.oregistry.find_entry(entry.path()) {
                                        self.oregistry
                                            .list_entry_tags(id)
                                            .unwrap_or_default()
                                            .iter()
                                            .map(|t| t.name().to_owned())
                                            .collect::<Vec<_>>()
                                    } else {
                                        vec![]
                                    }
                                } else {
                                    vec![]
                                },
                            Err(_) => vec![],
                        },
                    );

                    Ok(())
                },
            );
        } else {
            for (id, entry) in self.oregistry.list_entries_and_ids() {
                if !self.global && !contained_path(entry.path(), &self.base_dir) {
                    continue;
                }

                let search_str: Cow<OsStr> = Cow::Owned(entry.path().as_os_str().to_os_string());
                let search_bytes = osstr_to_bytes(search_str.as_ref());

                if !self.exclude.is_empty() && exclude_pattern.is_match(&search_bytes) {
                    continue;
                }

                if let Some(ref ext) = self.extension {
                    if !ext.is_match(&search_bytes) {
                        continue;
                    }
                }

                if let Some(ref file_types) = self.file_type {
                    if file_types.should_ignore(&entry.path()) {
                        log::debug!("Ignoring: {}", entry.path().display());
                        continue;
                    }
                }

                if re.is_match(&search_bytes) {
                    if !opts.tags.is_empty() && !self.oregistry.entry_has_any_tags(*id, &opts.tags)
                    {
                        continue;
                    }

                    map.insert(
                        tern::t!(
                            self.global
                            ? entry.path().display().to_string()
                            : self.fmt_raw_local_path(entry.path())
                        ),
                        self.oregistry
                            .list_entry_tags(*id)
                            .unwrap_or_default()
                            .iter()
                            .map(|t| t.name().to_owned())
                            .collect::<Vec<_>>(),
                    );
                }
            }
        }

        // Opts needs to overwrite config, which is why it's matched first
        let match_format = |format: &String| -> String {
            match format.as_str() {
                #[cfg(feature = "toml-backend")]
                "toml" => toml::to_string(&map)
                    .unwrap_or_else(|e| wutag_fatal!("serialization to toml failed: {}", e)),
                "json" => serde_json::to_string_pretty(&map)
                    .unwrap_or_else(|e| wutag_fatal!("serialization to json failed: {}", e)),
                #[cfg(feature = "yaml-backend")]
                "yaml" | "yml" => serde_yaml::to_string(&map)
                    .unwrap_or_else(|e| wutag_fatal!("serialization to yaml failed: {}", e)),
                _ => unreachable!(),
            }
        };

        let tag_file = if let Some(format) = &opts.format {
            match_format(format)
        } else {
            match_format(&self.format)
        };

        let mut tmp_path = PathBuf::from(create_temp_path());

        #[allow(clippy::match_same_arms)]
        tmp_path.set_extension(if let Some(format) = &opts.format {
            match format.as_str() {
                #[cfg(feature = "toml-backend")]
                f @ "toml" => f.to_owned(),
                #[cfg(feature = "yaml-backend")]
                f @ ("yaml" | "yml") => f.to_owned(),
                f @ "json" => f.to_owned(),
                _ => unreachable!(),
            }
        } else {
            match self.format.as_str() {
                #[cfg(feature = "toml-backend")]
                f @ "toml" => f.to_owned(),
                #[cfg(feature = "yaml-backend")]
                f @ ("yaml" | "yml") => f.to_owned(),
                f @ "json" => f.to_owned(),
                _ => unreachable!(),
            }
        });

        let mut tmp_file: fs::File = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&tmp_path)
            .unwrap_or_else(|_| {
                wutag_fatal!("could not create tmp file: '{}'", tmp_path.display())
            });

        tmp_file
            .write_all(tag_file.as_bytes())
            .unwrap_or_else(|_| wutag_fatal!("could not write tmp file: '{}'", tmp_path.display()));

        tmp_file
            .flush()
            .unwrap_or_else(|_| wutag_fatal!("could not flush tmp file: '{}'", tmp_path.display()));

        // Option does not need to be given, as it is read from an environment variable
        process::Command::new(&opts.editor)
            .arg(&tmp_path)
            .status()
            .expect("could not spawn editor");

        // Used to help highlight errors for whatever reason
        let re = regex!(r"^(\b[[:word:].]+\b): (.*)$");
        let color_file = |e: String| -> String {
            let cloned = e.clone();
            let string = if re.is_match(&cloned) {
                re.replace(&cloned, |caps: &Captures| {
                    format!(
                        "{}: {}",
                        caps.get(1).map_or(String::from(""), |m| m
                            .as_str()
                            .red()
                            .bold()
                            .to_string()),
                        caps.get(2)
                            .map_or(String::from(""), |m| m.as_str().to_string())
                    )
                })
            } else {
                Cow::from(e)
            };
            String::from(string)
        };

        let serialized_format = |format: &String| -> BTreeMap<String, Vec<String>> {
            match format.as_str() {
                #[cfg(feature = "toml-backend")]
                "toml" =>
                    toml::from_slice(&fs::read(&tmp_path).expect("failed to read tagged file"))
                        .unwrap_or_else(|e| {
                            wutag_fatal!(
                                "toml deserialization failed:\n\t{} {}",
                                "+".red().bold(),
                                color_file(e.to_string())
                            )
                        }),
                "json" => serde_json::from_slice(
                    &fs::read(&tmp_path).expect("failed to read tagged file"),
                )
                .unwrap_or_else(|e| {
                    wutag_fatal!(
                        "json deserialization failed:\n\t{} {}",
                        "+".red().bold(),
                        color_file(e.to_string())
                    )
                }),
                #[cfg(feature = "yaml-backend")]
                "yaml" | "yml" => serde_yaml::from_slice(
                    &fs::read(&tmp_path).expect("failed to read tagged file"),
                )
                .unwrap_or_else(|e| {
                    wutag_fatal!(
                        "yaml deserialization failed:\n\t{} {}",
                        "+".red().bold(),
                        color_file(e.to_string())
                    )
                }),
                _ => unreachable!(),
            }
        };

        let emap: BTreeMap<String, Vec<String>> = if let Some(format) = &opts.format {
            serialized_format(format)
        } else {
            serialized_format(&self.format)
        };

        let diff = emap.into_iter().fold(BTreeMap::new(), |mut acc, path| {
            let (key, val) = path;
            if map.iter().any(|(k, v)| *k == key && *v != val) {
                acc.insert(key, val);
            }
            acc
        });
        log::debug!("Diffs: {:#?}", diff);

        if diff.is_empty() {
            // log::debug!("There were no diffs");
            wutag_info!(
                "there were no new tags created in {}",
                self.base_dir.display().to_string().green()
            );
        } else {
            let base = &self.base_dir.clone();
            // let is_symlink = |entry: fs::Metadata, local| {
            //     if entry.file_type().is_symlink() {
            //         base.join(local)
            //             .canonicalize()
            //             .expect("failed to canonicalize path")
            //     } else {
            //         base.join(local).lexiclean()
            //     }
            // };

            for (local, tags) in &diff {
                let entry = &if fs::symlink_metadata(local).is_ok()
                    || fs::symlink_metadata(base.join(local)).is_ok()
                {
                    base.join(local).lexiclean()
                } else {
                    // Should never be reached since the diff iterator would filter it
                    wutag_error!(
                        "{} {} does not exist",
                        "X".red().bold(),
                        self.base_dir
                            .join(local)
                            .display()
                            .to_string()
                            .magenta()
                            .bold()
                    );
                    continue;
                };
                log::debug!("Using entry: {}", entry.display());

                // Clear all tags before writing new ones so there wouldn't
                // be a need to check if one is missing and delete it, or vice-versa
                // with adding it
                if let Some(id) = self.oregistry.find_entry(entry) {
                    self.oregistry.clear_entry(id);
                }

                if !self.quiet {
                    println!("{}:", self.fmt_path(entry));
                }

                match entry.has_tags() {
                    Ok(has_tags) =>
                        if has_tags {
                            log::debug!("Entry: {} has tags", entry.display());
                            if let Err(e) = clear_tags(entry) {
                                wutag_error!("\t{} {}", e, bold_entry!(entry));
                            }
                        },
                    Err(e) => {
                        wutag_error!("{} {}", e, bold_entry!(entry));
                    },
                }

                let tags = tags
                    .iter()
                    .map(|t| {
                        if let Some(t) = self.oregistry.get_tag(t) {
                            log::debug!("Got tag: {:?}", t);
                            t.clone()
                        } else {
                            log::debug!("Setting random tag: {:?}", t);
                            Tag::random(t, &self.colors)
                        }
                    })
                    .collect::<Vec<_>>();

                for tag in tags {
                    if let Err(e) = entry.tag(&tag) {
                        wutag_error!("{} {}", e, bold_entry!(entry));
                    } else {
                        let entry = EntryData::new(entry)?;
                        let id = self.oregistry.add_or_update_entry(entry);
                        self.oregistry.tag_entry(&tag, id);
                        if !self.quiet {
                            println!("\t{} {}", "+".bold().green(), fmt::tag_old(&tag));
                        }
                    }
                }
            }
        }
        log::debug!("Saving registry...");
        self.save_registry();

        Ok(())
    }
}
