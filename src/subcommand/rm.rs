use super::{
    uses::{
        err, fmt_err, fmt_path, fmt_tag, glob_builder, list_tags, osstr_to_bytes, reg_ok,
        regex_builder, wutag_error, Arc, Clap, Colorize, Cow, DirEntryExt, OsStr,
    },
    App,
};

#[derive(Clap, Clone, Debug, PartialEq)]
pub(crate) struct RmOpts {
    /// A glob pattern like "*.png" (or regex).
    pub(crate) pattern: String,
    pub(crate) tags:    Vec<String>,
}

impl App {
    pub(crate) fn rm(&mut self, opts: &RmOpts) {
        // Global will match a glob only against files that are tagged
        // Could add a fixed string option
        log::debug!("RmOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        log::debug!("Compiled pattern: {}", re);

        if self.global {
            let ctags = opts.tags.iter().collect::<Vec<_>>();
            let exclude_pattern = regex_builder(
                self.exclude.join("|").as_str(),
                self.case_insensitive,
                self.case_sensitive,
            );
            for (&id, entry) in self.registry.clone().list_entries_and_ids() {
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

                if re.is_match(&search_bytes) {
                    list_tags(entry.path())
                        .map(|tags| {
                            tags.iter().fold(Vec::new(), |mut acc, tag| {
                                acc.push((
                                    ctags.iter().find(|c| **c == &tag.to_string()),
                                    tag.clone(),
                                ));
                                acc
                            })
                        })
                        .unwrap_or_default()
                        .iter()
                        .for_each(|(search, realtag)| {
                            if search.is_some() {
                                // println!("SEARCH: {:?} REAL: {:?}", search, realtag);
                                self.registry.untag_by_name(search.unwrap(), id);
                                println!(
                                    "{}:",
                                    fmt_path(entry.path(), self.base_color, self.ls_colors)
                                );

                                if let Err(e) = realtag.remove_from(entry.path()) {
                                    err!('\t', e, entry);
                                } else {
                                    print!("\t{} {}", "X".bold().red(), fmt_tag(realtag));
                                }

                                println!();
                            }
                        });
                }
                log::debug!("Saving registry...");
                self.save_registry();
            }
        } else if let Err(e) = reg_ok(
            Arc::new(re),
            &Arc::new(self.clone()),
            |entry: &ignore::DirEntry| {
                log::debug!("Using WalkParallel");
                let id = self.registry.find_entry(entry.path());
                let tags = opts
                    .tags
                    .iter()
                    .map(|tag| {
                        if let Some(id) = id {
                            self.registry.untag_by_name(tag, id);
                        }
                        entry.get_tag(tag)
                    })
                    .collect::<Vec<_>>();

                if tags.is_empty() {
                    return;
                }

                println!(
                    "{}:",
                    fmt_path(entry.path(), self.base_color, self.ls_colors)
                );
                tags.iter().for_each(|tag| {
                    let tag = match tag {
                        Ok(tag) => tag,
                        Err(e) => {
                            err!('\t', e, entry);
                            return;
                        },
                    };
                    if let Err(e) = entry.untag(tag) {
                        err!('\t', e, entry);
                    } else {
                        print!("\t{} {}", "X".bold().red(), fmt_tag(tag));
                    }
                });
                println!();
                log::debug!("Saving registry...");
                self.save_registry();
            },
        ) {
            wutag_error!("{}", e);
        }
    }
}
