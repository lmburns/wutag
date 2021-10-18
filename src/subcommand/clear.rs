use super::{
    uses::{
        clear_tags, err, fmt_err, fmt_ok, fmt_path, fs, glob_builder, has_tags, osstr_to_bytes,
        reg_ok, regex_builder, wutag_error, Arc, Args, Colorize, Cow, DirEntryExt, OsStr,
    },
    App,
};

#[derive(Args, Debug, Clone, PartialEq)]
pub(crate) struct ClearOpts {
    // Opts::into_app().get_matches_from(env::args_os()).is_present("global")
    /// Clear all files from registry that no longer exist (requires --global)
    #[clap(long, short)]
    pub(crate) non_existent: bool,
    /// A glob pattern like "*.png".
    pub(crate) pattern:      String,
}

impl App {
    pub(crate) fn clear(&mut self, opts: &ClearOpts) {
        log::debug!("ClearOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.registry.path.display());
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);

        if self.global {
            let exclude_pattern = regex_builder(
                self.exclude.join("|").as_str(),
                self.case_insensitive,
                self.case_sensitive,
            );
            for (&id, entry) in self.registry.clone().list_entries_and_ids() {
                let search_str: Cow<OsStr> = Cow::Owned(entry.path().as_os_str().to_os_string());
                let search_bytes = &osstr_to_bytes(search_str.as_ref());
                if !self.exclude.is_empty() && exclude_pattern.is_match(search_bytes) {
                    continue;
                }

                if let Some(ref ext) = self.extension {
                    if !ext.is_match(search_bytes) {
                        continue;
                    }
                }

                if re.is_match(search_bytes) {
                    self.registry.clear_entry(id);
                    if opts.non_existent && fs::metadata(entry.path()).is_err() {
                        println!(
                            "{}\n\t{} {}",
                            fmt_path(entry.path(), self.base_color, self.ls_colors),
                            fmt_ok("cleared"),
                            "old entry".magenta().bold()
                        );
                    } else {
                        match has_tags(entry.path()) {
                            Ok(has_tags) =>
                                if has_tags {
                                    println!(
                                        "{}:",
                                        fmt_path(entry.path(), self.base_color, self.ls_colors)
                                    );
                                    if let Err(e) = clear_tags(entry.path()) {
                                        err!('\t', e, entry);
                                    } else {
                                        println!("\t{}", fmt_ok("cleared"));
                                    }
                                },
                            Err(e) => {
                                err!(e, entry);
                            },
                        }
                    }
                }
            }
            log::debug!("Saving registry...");
            self.save_registry();
        } else {
            // The parser for clap is great, though I've not figured out a way to get
            // options of a subcommand to communicate with options of the main
            // binary
            if opts.non_existent && !self.global {
                wutag_error!(
                    "{} requires {}",
                    "--non-existent".green(),
                    "--global".green()
                );
                std::process::exit(1);
            }

            reg_ok(
                &Arc::new(re),
                &Arc::new(self.clone()),
                |entry: &ignore::DirEntry| {
                    if let Some(id) = self.registry.find_entry(entry.path()) {
                        self.registry.clear_entry(id);
                    }

                    match entry.has_tags() {
                        Ok(has_tags) =>
                            if has_tags {
                                println!(
                                    "{}:",
                                    fmt_path(entry.path(), self.base_color, self.ls_colors)
                                );
                                if let Err(e) = entry.clear_tags() {
                                    err!('\t', e, entry);
                                } else {
                                    println!("\t{}", fmt_ok("cleared"));
                                }
                            },
                        Err(e) => {
                            err!(e, entry);
                        },
                    }
                    log::debug!("Saving registry...");
                    self.save_registry();
                },
            );
        }
    }
}
