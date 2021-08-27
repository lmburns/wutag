use super::{uses::*, App};

#[derive(Clap, Clone, Debug, PartialEq)]
pub struct SetOpts {
    /// Do not show errors that tag already exists
    #[clap(name = "quiet", long, short = 'q')]
    quiet:       bool,
    /// Clear all tags before setting them
    #[clap(long, short)]
    pub clear:   bool,
    /// Explicitly select color for tag
    #[clap(long, short = 'C', takes_value = true,
        validator = |t| parse_color(t)
                            .map_err(|_| "must be a valid hex color")
                            .map(|_| ())
                            .map_err(|e| e.to_string())
    )]
    pub color:   Option<String>,
    /// A glob pattern like "*.png".
    pub pattern: String,
    pub tags:    Vec<String>,
}

impl App {
    pub(crate) fn set(&mut self, opts: &SetOpts) {
        log::debug!("Using registry: {}", self.registry.path.display());
        let tags = opts
            .tags
            .iter()
            .map(|t| {
                if let Some(t) = self.registry.get_tag(t) {
                    t.clone()
                } else if let Some(color) = &opts.color {
                    Tag::new(
                        t,
                        parse_color(color).unwrap_or_else(|e| {
                            wutag_error!("{}", e);
                            DEFAULT_COLOR
                        }),
                    )
                } else {
                    Tag::random(t, &self.colors)
                }
            })
            .collect::<Vec<_>>();

        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        log::debug!("Compiled pattern: {}", re);

        if let Err(e) = reg_ok(
            Arc::new(re),
            &Arc::new(self.clone()),
            |entry: &ignore::DirEntry| {
                println!(
                    "{}:",
                    fmt_path(entry.path(), self.base_color, self.ls_colors)
                );
                tags.iter().for_each(|tag| {
                    if opts.clear {
                        log::debug!(
                            "Using registry in threads: {}",
                            self.registry.path.display()
                        );
                        if let Some(id) = self.registry.find_entry(entry.path()) {
                            self.registry.clear_entry(id);
                        }
                        match entry.has_tags() {
                            Ok(has_tags) =>
                                if has_tags {
                                    if let Err(e) = entry.clear_tags() {
                                        err!('\t', e, entry);
                                    }
                                },
                            Err(e) => {
                                err!(e, entry);
                            },
                        }
                    }

                    if let Err(e) = entry.tag(tag) {
                        // TODO: Make this skip printing path too
                        if !opts.quiet {
                            err!('\t', e, entry);
                        }
                    } else {
                        let entry = EntryData::new(entry.path());
                        let id = self.registry.add_or_update_entry(entry);
                        self.registry.tag_entry(tag, id);
                        print!("\t{} {}", "+".bold().green(), fmt_tag(tag));
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
