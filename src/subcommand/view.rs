use lexiclean::Lexiclean;

use super::{uses::*, App};
use crate::bold_entry;

#[derive(Clap, Debug, Clone)]
pub struct ViewOpts {
    /// Open tags in selected edtor (use only with vi, vim, neovim)
    #[clap(
        long, short,
        env = "EDITOR",
        default_value = DEFAULT_EDITOR.as_ref(),
        value_name = "editor",
        setting = ArgSettings::HideEnv,
        setting = ArgSettings::HideDefaultValue,
    )]
    pub editor:  String,
    #[clap(long, short = 'a')]
    pub all:     bool,
    /// Format of file to view results (toml, yaml, json)
    #[clap(
        name = "format",
        long, short = 'f',
        default_value = "toml",
        possible_values = &["toml", "yaml", "yml", "json"],
        setting = ArgSettings::HideDefaultValue,
        setting = ArgSettings::HidePossibleValues,
        long_about = "\
        Format of the file viewed in the editor with the matching search results.\
        The possible values are: 'toml', 'yaml|yml', 'json'."
    )]
    pub format:  String,
    /// Search with a tag as a filter
    #[clap(
        name = "tags",
        long,
        short,
        long_about = "\
        Limit search results even further by using a tag as a filter. Can search just for tags by \
                      not using '--pattern'"
    )]
    pub tags:    Vec<String>,
    /// Pattern to search for and open result in editor
    #[clap(
        name = "pattern",
        long,
        short = 'p',
        long_about = "\
        This pattern is optional. If no pattern is given, all files that have a tag will be shown \
                      in the editor. Otherwise, the results that match the pattern will be shown."
    )]
    pub pattern: Option<String>,
}

impl App {
    pub(crate) fn view(&mut self, opts: &ViewOpts) {
        // TODO: Move this to search and add as a feature to prevent duplicate code
        let pat = if let Some(pattern) = &opts.pattern {
            if self.pat_regex {
                String::from(pattern)
            } else {
                glob_builder(pattern)
            }
        } else {
            glob_builder("*")
        };

        let re = regex_builder(&pat, self.case_insensitive);
        let exclude_pattern = regex_builder(self.exclude.join("|").as_str(), self.case_insensitive);

        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // if opts.all {
        //     // let optsc = Arc::new(Mutex::new(opts.clone()));
        //     let selfc = Arc::new(Mutex::new(self.clone()));
        //
        //     if let Err(e) = reg_ok(
        //         Arc::new(re),
        //         &Arc::new(self.clone()),
        //         move |entry: &ignore::DirEntry| {
        //             // let optsc = Arc::clone(&optsc);
        //             // let opts = optsc.lock().unwrap();
        //
        //             let selfc = Arc::clone(&selfc);
        //             let selfu = selfc.lock().unwrap();
        //
        //             map.insert(
        //                 ternary!(
        //                     selfu.global,
        //                     entry.path().display().to_string(),
        //                     raw_local_path(entry.path(), &selfu.base_dir)
        //                 ),
        //                 match entry.has_tags() {
        //                     Ok(has_tags) => {
        //                         if has_tags {
        //                             if let Some(id) =
        // selfu.registry.find_entry(entry.path()) {
        // selfu.registry
        // .list_entry_tags(id)
        // .unwrap_or_default()                                     .iter()
        //                                     .map(|t| t.name().to_owned())
        //                                     .collect::<Vec<_>>()
        //                             } else {
        //                                 vec![]
        //                             }
        //                         } else {
        //                             vec![]
        //                         }
        //                     },
        //                     Err(_) => vec![],
        //                 }
        //             );
        //         },
        //     ) {
        //         wutag_error!("{}", e);
        //     }
        // } else {
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
                if !opts.tags.is_empty() && !self.registry.entry_has_tags(*id, &opts.tags) {
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
        // }

        let tag_file = if opts.format == "toml" {
            // to_string_pretty
            toml::to_string(&map).expect("Unable to convert toml")
        } else if opts.format == "json" {
            serde_json::to_string_pretty(&map).expect("Unable to convert to yaml")
        } else {
            serde_yaml::to_string(&map).expect("Unable to convert to yaml")
        };

        let mut tmp_path = PathBuf::from(create_temp_path());

        if opts.format == "toml" {
            tmp_path.set_extension("toml");
        } else if opts.format == "json" {
            tmp_path.set_extension("json");
        } else {
            tmp_path.set_extension("yml");
        }

        let mut tmp_file: fs::File = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&tmp_path)
            .unwrap_or_else(|_| panic!("could not create tmp file: '{}'", tmp_path.display()));

        tmp_file
            .write_all(tag_file.as_bytes())
            .unwrap_or_else(|_| panic!("could not write tmp file: '{}'", tmp_path.display()));

        tmp_file
            .flush()
            .unwrap_or_else(|_| panic!("could not flush tmp file: '{}'", tmp_path.display()));

        process::Command::new(opts.editor.to_string())
            .arg(&tmp_path)
            .status()
            .expect("could not spawn editor");

        let emap: BTreeMap<String, Vec<String>> = if opts.format == "toml" {
            toml::from_slice(&fs::read(&tmp_path).expect("failed to read tagged file"))
                .expect("failed to deserialize tag file")
        } else if opts.format == "json" {
            serde_json::from_slice(&fs::read(&tmp_path).expect("failed to read tagged file"))
                .expect("failed to deserialize tag file")
        } else {
            serde_yaml::from_slice(&fs::read(&tmp_path).expect("failed to read tagged file"))
                .expect("failed to deserialize tag file")
        };

        let diff = emap.into_iter().fold(BTreeMap::new(), |mut acc, path| {
            let (key, val) = path;
            if map.iter().any(|(k, v)| *k == key && *v != val) {
                acc.insert(key, val);
            }
            acc
        });
        log::debug!("Diffs: {:#?}", diff);

        if !diff.is_empty() {
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

            for (local, tags) in diff.iter() {
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

                println!("{}:", fmt_path(entry, self.base_color, self.ls_colors));

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

                tags.par_iter()
                    .map(|t| {
                        if let Some(t) = self.registry.get_tag(t) {
                            log::debug!("Got tag: {:?}", t);
                            t.clone()
                        } else {
                            log::debug!("Setting random tag: {:?}", t);
                            Tag::random(t, &self.colors)
                        }
                    })
                    .collect::<Vec<_>>()
                    .iter()
                    .for_each(|tag| {
                        if let Err(e) = entry.tag(tag) {
                            wutag_error!("{} {}", e, bold_entry!(entry))
                        } else {
                            let entry = EntryData::new(entry);
                            let id = self.registry.add_or_update_entry(entry);
                            self.registry.tag_entry(tag, id);
                            println!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                        }
                    });
            }
        } else {
            log::debug!("There were no diffs");
        }
        log::debug!("Saving registry...");
        self.save_registry();
    }
}
