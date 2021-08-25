use super::{uses::*, App};

#[derive(Clap, Clone, Debug)]
pub struct SetOpts {
    /// Clear all tags before setting them
    #[clap(long, short)]
    pub clear:   bool,
    /// A glob pattern like "*.png".
    /// Explicitly select color for tag
    #[clap(long, short = 'C', takes_value = true,
        validator = |t| parse_color(t)
                            .map_err(|_| "must be a valid hex color")
                            .map(|_| ())
                            .map_err(|e| e.to_string())
    )]
    pub color:   Option<String>,
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

        let re = regex_builder(&pat, self.case_insensitive);
        log::debug!("Compiled pattern: {}", re);

        let optsc = Arc::new(Mutex::new(opts.clone()));
        let selfc = Arc::new(Mutex::new(self.clone()));

        if let Err(e) = reg_ok(
            Arc::new(re),
            &Arc::new(self.clone()),
            move |entry: &ignore::DirEntry| {
                let optsc = Arc::clone(&optsc);
                let opts = optsc.lock().unwrap();

                let selfc = Arc::clone(&selfc);
                let mut selfu = selfc.lock().unwrap();
                println!(
                    "{}:",
                    fmt_path(entry.path(), selfu.base_color, selfu.ls_colors)
                );
                tags.iter().for_each(|tag| {
                    if opts.clear {
                        log::debug!(
                            "Using registry in threads: {}",
                            selfu.registry.path.display()
                        );
                        if let Some(id) = selfu.registry.find_entry(entry.path()) {
                            selfu.registry.clear_entry(id);
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
                        err!('\t', e, entry);
                    } else {
                        let entry = EntryData::new(entry.path());
                        let id = selfu.registry.add_or_update_entry(entry);
                        selfu.registry.tag_entry(tag, id);
                        print!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                    }
                });
                println!();
                log::debug!("Saving registry...");
                selfu.save_registry();
            },
        ) {
            wutag_error!("{}", e);
        }
    }
}
