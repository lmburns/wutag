use super::{
    uses::{
        bold_entry, clear_tags, contained_path, create_temp_path, fmt_path, fmt_tag, fs,
        glob_builder, osstr_to_bytes, process, raw_local_path, reg_ok, regex_builder, ternary,
        wutag_error, wutag_fatal, wutag_info, Arc, ArgSettings, Args, BTreeMap, Captures, Colorize,
        Cow, DirEntryExt, EntryData, IntoParallelRefIterator, Lexiclean, OsStr, ParallelIterator,
        PathBuf, Regex, Result, Tag, Write, DEFAULT_EDITOR,
    },
    App,
};

#[derive(Args, Debug, Clone, PartialEq, Default)]
pub(crate) struct ViewOpts {
    /// Open tags in selected edtor (use only with vi, vim, neovim)
    #[clap(
        long, short,
        env = "EDITOR",
        default_value = DEFAULT_EDITOR.as_ref(),
        value_name = "editor",
        setting = ArgSettings::HideEnv,
        setting = ArgSettings::HideDefaultValue,
    )]
    pub(crate) editor:  String,
    #[clap(long, short = 'a')]
    pub(crate) all:     bool,
    /// Format of file to view results (toml, yaml, json)
    #[clap(
        name = "format",
        long, short = 'f',
        possible_values = &["toml", "yaml", "yml", "json"],
        setting = ArgSettings::HidePossibleValues,
        long_about = "\
        Format of the file viewed in the editor with the matching search results.\
        The possible values are: 'toml', 'yaml|yml', 'json'."
    )]
    pub(crate) format:  Option<String>,
    /// Search with a tag as a filter
    #[clap(
        name = "tags",
        long,
        short,
        long_about = "\
        Limit search results even further by using a tag as a filter. Can search just for tags by \
                      not using '--pattern'"
    )]
    pub(crate) tags:    Vec<String>,
    /// Pattern to search for and open result in editor
    #[clap(
        name = "pattern",
        long,
        short = 'p',
        long_about = "\
        This pattern is optional. If no pattern is given, all files that have a tag will be shown \
                      in the editor. Otherwise, the results that match the pattern will be shown."
    )]
    pub(crate) pattern: Option<String>,
}

impl App {
    pub(crate) fn view(&mut self, opts: &ViewOpts) -> Result<()> {
        log::debug!("ViewOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());
        let pat = if let Some(pattern) = &opts.pattern {
            if self.pat_regex {
                String::from(pattern)
            } else {
                glob_builder(pattern)
            }
        } else {
            glob_builder("*")
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        let exclude_pattern = regex_builder(
            self.exclude.join("|").as_str(),
            self.case_insensitive,
            self.case_sensitive,
        );

        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();

        if opts.all {
            reg_ok(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    map.insert(
                        ternary!(
                            self.global,
                            entry.path().display().to_string(),
                            raw_local_path(entry.path(), &self.base_dir)
                        ),
                        match entry.has_tags() {
                            Ok(has_tags) =>
                                if has_tags {
                                    if let Some(id) = self.registry.find_entry(entry.path()) {
                                        self.registry
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
                },
            );
        } else {
            for (id, entry) in self.registry.list_entries_and_ids() {
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
                    if !opts.tags.is_empty() && !self.registry.entry_has_any_tags(*id, &opts.tags) {
                        continue;
                    }

                    map.insert(
                        ternary!(
                            self.global,
                            entry.path().display().to_string(),
                            raw_local_path(entry.path(), &self.base_dir)
                        ),
                        self.registry
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
                "toml" => toml::to_string(&map)
                    .unwrap_or_else(|e| wutag_fatal!("serialization to toml failed: {}", e)),
                "json" => serde_json::to_string_pretty(&map)
                    .unwrap_or_else(|e| wutag_fatal!("serialization to json failed: {}", e)),
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

        tmp_path.set_extension(if let Some(format) = &opts.format {
            match format.as_str() {
                f @ ("toml" | "yaml" | "yml" | "json") => f.to_string(),
                _ => unreachable!(),
            }
        } else {
            match self.format.as_str() {
                f @ ("toml" | "yaml" | "yml" | "json") => f.to_string(),
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
        process::Command::new(opts.editor.to_string())
            .arg(&tmp_path)
            .status()
            .expect("could not spawn editor");

        // Used to help highlight errors for whatever reason
        let re = Regex::new(r"^(\b[[:word:].]+\b): (.*)$").unwrap();
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
                if let Some(id) = self.registry.find_entry(entry) {
                    self.registry.clear_entry(id);
                }

                if !self.quiet {
                    println!("{}:", fmt_path(entry, self.base_color, self.ls_colors));
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
                    .par_iter()
                    .map(|t| {
                        if let Some(t) = self.registry.get_tag(t) {
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
                        let id = self.registry.add_or_update_entry(entry);
                        self.registry.tag_entry(&tag, id);
                        if !self.quiet {
                            println!("\t{} {}", "+".bold().green(), fmt_tag(&tag));
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
