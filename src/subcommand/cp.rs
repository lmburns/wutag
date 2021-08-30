use super::{
    uses::{
        err, fmt_err, fmt_path, fmt_tag, glob_builder, list_tags, osstr_to_bytes, parse_path,
        reg_ok, regex_builder, wutag_error, Arc, Clap, Colorize, Cow, DirEntryExt, EntryData,
        OsStr, PathBuf, ValueHint,
    },
    App,
};

#[derive(Clap, Debug, Clone, PartialEq)]
pub(crate) struct CpOpts {
    /// Path to the file from which to copy tags from
    #[clap(
        value_name = "input_path",
        value_hint = ValueHint::FilePath,
        validator = |t| parse_path(t)
    )]
    pub(crate) input_path: PathBuf,
    /// A glob pattern like "*.png".
    #[clap(value_name = "pattern")]
    pub(crate) pattern:    String,
}

impl App {
    pub(crate) fn cp(&mut self, opts: &CpOpts) {
        log::debug!("CpOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());

        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        let path = opts.input_path.as_path();

        // FIX: Manage both globs
        if self.global {
            let exclude_pattern = regex_builder(
                self.exclude.join("|").as_str(),
                self.case_insensitive,
                self.case_sensitive,
            );

            for (_, entry) in self.registry.clone().list_entries_and_ids() {
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
                    println!("MATCH: {}", entry.path().display());
                    let entry_path = &PathBuf::from(entry.path());
                    match list_tags(entry.path()) {
                        Ok(tags) =>
                            for tag in &tags {
                                if let Err(e) = entry_path.tag(tag) {
                                    err!('\t', e, entry)
                                } else {
                                    let entry = EntryData::new(entry.path());
                                    let id = self.registry.add_or_update_entry(entry);
                                    self.registry.tag_entry(tag, id);
                                    println!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                                }
                            },
                        Err(e) => wutag_error!(
                            "failed to get source tags from `{}` - {}",
                            path.display(),
                            e
                        ),
                    }
                }
                log::debug!("Saving registry...");
                self.save_registry();
            }
        } else {
            match list_tags(path) {
                Ok(tags) => {
                    if let Err(e) = reg_ok(
                        Arc::new(re),
                        &Arc::new(self.clone()),
                        |entry: &ignore::DirEntry| {
                            println!(
                                "{}:",
                                fmt_path(entry.path(), self.base_color, self.ls_colors)
                            );
                            for tag in &tags {
                                if let Err(e) = entry.tag(tag) {
                                    err!('\t', e, entry)
                                } else {
                                    let entry = EntryData::new(entry.path());
                                    let id = self.registry.add_or_update_entry(entry);
                                    self.registry.tag_entry(tag, id);
                                    println!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                                }
                            }
                        },
                    ) {
                        wutag_error!("{}", e);
                    }
                    log::debug!("Saving registry...");
                    self.save_registry();
                },
                Err(e) => wutag_error!(
                    "failed to get source tags from `{}` - {}",
                    path.display(),
                    e
                ),
            }
        }
    }
}
