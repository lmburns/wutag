use super::App;
use crate::{
    bold_entry, err,
    oregistry::EntryData,
    util::{collect_stdin_paths, fmt_err, fmt_path, fmt_tag, glob_builder, reg_ok, regex_builder},
    wutag_error, wutag_fatal,
};
use anyhow::Result;
use clap::{Args, ValueHint};
use colored::Colorize;
use std::sync::Arc;
use wutag_core::{
    color::parse_color,
    tag::{DirEntryExt, Tag, DEFAULT_COLOR},
};

#[derive(Args, Clone, Debug, PartialEq)]
pub(crate) struct SetOpts {
    // TODO: Implement/remove
    /// Do not show errors that tag already exists
    #[clap(name = "quiet", long, short = 'q')]
    pub(crate) quiet:   bool,
    /// Clear all tags before setting them
    #[clap(long, short)]
    pub(crate) clear:   bool,
    /// Explicitly select color for tag
    #[clap(long, short = 'C', takes_value = true,
        validator = |t| parse_color(t)
                            .map_err(|_| "must be a valid hex color")
                            .map(|_| ())
                            .map_err(|e| e.to_string())
    )]
    pub(crate) color:   Option<String>,
    #[clap(name = "stdin", long, short = 's')]
    pub(crate) stdin:   bool,
    /// A glob pattern like "*.png".
    #[clap(
        required_unless_present = "stdin", // Would be nice to have a default_value_if_present
        value_hint = ValueHint::FilePath,
    )]
    pub(crate) pattern: String,
    pub(crate) tags:    Vec<String>,
}

impl App {
    pub(crate) fn set(&mut self, opts: &SetOpts) -> Result<()> {
        log::debug!("SetOpts: {:#?}", opts);
        log::debug!("Using registry: {}", self.oregistry.path.display());

        // Needed because it's not possible (as far as I know) to skip an argument if
        // another is present
        let mut tags = opts.tags.clone();
        if (opts.stdin || atty::isnt(atty::Stream::Stdin)) && atty::is(atty::Stream::Stdout) {
            tags.push(opts.pattern.clone());
        }

        let tags = tags
            .iter()
            .map(|t| {
                if let Some(t) = self.oregistry.get_tag(t) {
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
        } else if self.fixed_string {
            regex::escape(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        log::debug!("Is a TTY?: {}", atty::is(atty::Stream::Stdout));
        let re = regex_builder(&pat, self.case_insensitive, self.case_sensitive);
        log::debug!("Compiled pattern: {}", re);

        if (opts.stdin || atty::isnt(atty::Stream::Stdin)) && atty::is(atty::Stream::Stdout) {
            log::debug!("Using STDIN");
            for entry in &collect_stdin_paths(&self.base_dir) {
                if !self.quiet {
                    println!("{}:", fmt_path(entry, self.base_color, self.ls_colors));
                }

                for tag in &tags {
                    if opts.clear {
                        log::debug!(
                            "Using registry in threads: {}",
                            self.oregistry.path.display()
                        );
                        if let Some(id) = self.oregistry.find_entry(entry) {
                            self.oregistry.clear_entry(id);
                        }
                        match entry.has_tags() {
                            Ok(has_tags) =>
                                if has_tags {
                                    if let Err(e) = entry.clear_tags() {
                                        wutag_error!("\t{} {}", e, bold_entry!(entry));
                                    }
                                },
                            Err(e) => {
                                wutag_error!("{} {}", e, bold_entry!(entry));
                            },
                        }
                    }

                    if let Err(e) = entry.tag(tag) {
                        log::debug!("Error setting tag for: {}", entry.display());
                        if !opts.quiet {
                            wutag_error!("{} {}", e, bold_entry!(entry));
                        }
                    } else {
                        log::debug!("Setting tag for new entry: {}", entry.display());
                        let entry = EntryData::new(entry)?;
                        let id = self.oregistry.add_or_update_entry(entry);
                        self.oregistry.tag_entry(tag, id);
                        if !self.quiet {
                            print!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                        }
                    }
                }
                if !self.quiet {
                    println!();
                }
            }
        } else {
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
                        if opts.clear {
                            log::debug!(
                                "Using registry in threads: {}",
                                self.oregistry.path.display()
                            );
                            if let Some(id) = self.oregistry.find_entry(entry.path()) {
                                self.oregistry.clear_entry(id);
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
                            log::debug!("Error setting tag for: {}", entry.path().display());
                            if !self.quiet {
                                err!('\t', e, entry);
                            }
                        } else {
                            log::debug!("Setting tag for new entry: {}", entry.path().display());
                            let entry = if let Ok(data) = EntryData::new(entry.path()) {
                                data
                            } else {
                                wutag_fatal!(
                                    "unable to create new entry: {}",
                                    entry.path().display()
                                );
                            };
                            let id = self.oregistry.add_or_update_entry(entry);
                            self.oregistry.tag_entry(tag, id);
                            print!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                        }
                    }
                    if !self.quiet {
                        println!();
                    }
                    // log::debug!("Saving registry...");
                    // self.save_registry();

                    Ok(())
                },
            );
        }
        log::debug!("Saving registry...");
        self.save_registry();

        Ok(())
    }
}
