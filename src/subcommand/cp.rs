use super::{uses::*, App};

#[derive(Clap, Debug, Clone)]
pub struct CpOpts {
    /// Path to the file from which to copy tags from
    #[clap(
        value_name = "input_path",
        value_hint = ValueHint::FilePath,
        validator = |t| fs::metadata(t)
                            .map_err(|_| "must be a valid path")
                            .map(|_| ())
                            .map_err(|e| e.to_string()),
    )]
    pub input_path: PathBuf,
    /// A glob pattern like "*.png".
    #[clap(value_name = "pattern")]
    pub pattern:    String,
}

impl App {
    pub(crate) fn cp(&mut self, opts: &CpOpts) {
        // TODO: Add global option
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive);
        let path = opts.input_path.as_path();

        match list_tags(path) {
            Ok(tags) => {
                let selfc = Arc::new(Mutex::new(self.clone()));
                if let Err(e) = reg_ok(
                    Arc::new(re),
                    &Arc::new(self.clone()),
                    move |entry: &ignore::DirEntry| {
                        let selfc = Arc::clone(&selfc);
                        let mut selfu = selfc.lock().unwrap();
                        println!(
                            "{}:",
                            fmt_path(entry.path(), selfu.base_color, selfu.ls_colors)
                        );
                        for tag in &tags {
                            if let Err(e) = entry.tag(tag) {
                                err!('\t', e, entry)
                            } else {
                                let entry = EntryData::new(entry.path());
                                let id = selfu.registry.add_or_update_entry(entry);
                                selfu.registry.tag_entry(tag, id);
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
