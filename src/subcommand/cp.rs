//! Copy attributes of one tag to another

// TODO: Add mv option
// TODO: Add global option to cp

use super::{
    uses::{
        err, fmt_err, fmt_path, fmt_tag, glob_builder, list_tags, osstr_to_bytes, parse_path,
        reg_ok, regex_builder, wutag_error, wutag_fatal, Arc, Args, Colorize, Cow, DirEntryExt,
        EntryData, OsStr, PathBuf, Result, ValueHint,
    },
    App,
};

#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct CpOpts {
    /// Use a glob to match files (must be global)
    #[clap(
        short = 'G',
        long = "glob",
        takes_value = false,
        long_help = "Use a glob to match the input path instead of matching files in the local \
                     directory. This argument doesn't do anything, and is a placeholder. If the \
                     global option is used, pattern matching is turned on and file-matching is no \
                     longer used"
    )]
    pub(crate) glob:       bool,
    /// Path to the file from which to copy tags from
    #[clap(
        value_name = "input_path",
        value_hint = ValueHint::FilePath,
        // validator = |t| parse_path(t) // Would be nice to be aware of other options
    )]
    pub(crate) input_path: PathBuf,
    /// A glob pattern like "*.png".
    #[clap(value_name = "pattern")]
    pub(crate) pattern:    String,
}

impl App {
    pub(crate) fn cp(&mut self, opts: &CpOpts) -> Result<()> {
        log::debug!("CpOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());

        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        let path = opts.input_path.as_path();

        // FIX: Manage both globs for input and output
        // To do this, a selection menu or something would have to popup to have the
        // user choose which tags to copy, since multiple would match
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
                    // println!("MATCH: {}", entry.path().display());
                    let entry_path = &PathBuf::from(entry.path());
                    match list_tags(entry.path()) {
                        Ok(tags) =>
                            for tag in &tags {
                                if let Err(e) = entry_path.tag(tag) {
                                    err!('\t', e, entry);
                                } else {
                                    let entry = EntryData::new(entry.path())?;
                                    let id = self.registry.add_or_update_entry(entry);
                                    self.registry.tag_entry(tag, id);
                                    if !self.quiet {
                                        println!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                                    }
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
            if let Err(e) = parse_path(path) {
                wutag_error!("{}: {}", e, path.display());
            }

            match list_tags(path) {
                Ok(tags) => {
                    reg_ok(
                        &Arc::new(re),
                        &Arc::new(self.clone()),
                        |entry: &ignore::DirEntry| {
                            if !self.quiet {
                                println!(
                                    "{}:",
                                    fmt_path(entry.path(), self.base_color, self.ls_colors)
                                );
                            }
                            for tag in &tags {
                                if let Err(e) = entry.tag(tag) {
                                    err!('\t', e, entry);
                                } else {
                                    let entry = if let Ok(data) = EntryData::new(entry.path()) {
                                        data
                                    } else {
                                        wutag_fatal!(
                                            "unable to create new entry: {}",
                                            entry.path().display()
                                        );
                                    };
                                    let id = self.registry.add_or_update_entry(entry);
                                    self.registry.tag_entry(tag, id);
                                    if !self.quiet {
                                        println!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                                    }
                                }
                            }
                        },
                    );
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

        Ok(())
    }
}
