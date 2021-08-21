use anyhow::{anyhow, Context, Result};
use atty::Stream;
use clap::IntoApp;
use colored::{Color, Colorize};
use regex::bytes::{RegexSet, RegexSetBuilder};
use shellexpand::LookupError;

use std::{
    borrow::Cow,
    collections::HashMap,
    env,
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    comp_helper,
    config::Config,
    exe::{job::run_command, CommandTemplate},
    opt::{
        ClearOpts, Command, CompletionsOpts, CpOpts, EditOpts, ListObject, ListOpts, Opts, RmOpts,
        SearchOpts, SetOpts, Shell, APP_NAME,
    },
    registry::{EntryData, TagRegistry},
    util::{
        contained_path, fmt_err, fmt_local_path, fmt_ok, fmt_path, fmt_tag, glob_builder,
        osstr_to_bytes, print_err, raw_local_path, reg_ok, regex_builder,
    },
    DEFAULT_BASE_COLOR, DEFAULT_COLORS,
};

use wutag_core::{
    color::parse_color,
    tag::{clear_tags, has_tags, list_tags, DirEntryExt, Tag, DEFAULT_COLOR},
};

#[derive(Clone, Debug)]
pub struct App {
    pub base_dir:         PathBuf,
    pub max_depth:        Option<usize>,
    pub base_color:       Color,
    pub colors:           Vec<Color>,
    pub ignores:          Option<Vec<String>>,
    pub global:           bool,
    pub registry:         TagRegistry,
    pub case_insensitive: bool,
    pub pat_regex:        bool,
    pub ls_colors:        bool,
    pub color_when:       String,
    pub extension:        Option<RegexSet>,
    pub exclude:          Vec<String>,
}

/// Format errors
macro_rules! err {
    ($err:ident, $entry:ident) => {
        err!("", $err, $entry);
    };
    ($prefix:expr, $err:ident, $entry:ident) => {{
        let err = fmt_err($err);
        eprintln!(
            "{}{} - {}",
            $prefix,
            err,
            $entry.path().to_string_lossy().bold()
        );
    }};
}

/// Makeshift ternary 2 == 2 ? "yes" : "no", mainly used for printing
macro_rules! ternary {
    ($c:expr, $v:expr, $v1:expr) => {
        if $c {
            $v
        } else {
            $v1
        }
    };
}

impl App {
    pub(crate) fn run(opts: Opts, config: Config) -> Result<()> {
        let mut app = Self::new(&opts, config)?;
        log::debug!("CONFIGURATION: {:#?}", app);
        app.run_command(opts.cmd);

        Ok(())
    }

    pub(crate) fn new(opts: &Opts, config: Config) -> Result<App> {
        let base_dir = if let Some(base_dir) = &opts.dir {
            if base_dir.display().to_string() == "." {
                std::env::current_dir().context("failed to determine current working directory")?
            } else {
                base_dir.to_path_buf()
            }
        } else {
            std::env::current_dir().context("failed to determine current working directory")?
        };

        // let ignores = config
        //     .ignores
        //     .clone()
        //     .unwrap_or_else(Vec::new);

        let colors = if let Some(_colors) = config.colors {
            let mut colors = Vec::new();
            for color in _colors.iter().map(parse_color) {
                colors.push(color?);
            }
            colors
        } else {
            DEFAULT_COLORS.to_vec()
        };

        let base_color = config
            .base_color
            .map(parse_color)
            .transpose()?
            .unwrap_or(DEFAULT_BASE_COLOR);

        let color_when = match opts.color_when {
            Some(ref s) if s == "always" => "always",
            Some(ref s) if s == "never" => "never",
            _ =>
                if env::var_os("NO_COLOR").is_none() && atty::is(Stream::Stdout) {
                    "auto"
                } else {
                    "never"
                },
        };

        let cache_dir = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| dirs::home_dir().map(|d| d.join(".cache")));
        let state_file = cache_dir.unwrap().join("wutag.registry");

        let registry = if let Some(registry) = &opts.reg {
            // Expand both tlide '~' and environment variables in 'WUTAG_REGISTRY' env var
            let registry = &PathBuf::from(
                shellexpand::full(&registry.display().to_string())
                    .unwrap_or_else(|_| {
                        Cow::from(
                            LookupError {
                                var_name: "UNKNOWN_ENVIRONMENT_VARIABLE".into(),
                                cause:    env::VarError::NotPresent,
                            }
                            .to_string(),
                        )
                    })
                    .to_string(),
            );

            if registry.is_file() && registry.file_name().is_some() {
                TagRegistry::load(&registry).unwrap_or_else(|_| TagRegistry::new(&registry))
            } else if registry.is_dir() && registry.file_name().is_some() {
                print_err(format!(
                    "{} is not a file. Using default registry: {}",
                    registry.display().to_string().green(),
                    state_file.display().to_string().green(),
                ));
                TagRegistry::load(&state_file).unwrap_or_else(|_| TagRegistry::new(&state_file))
            } else if !registry.display().to_string().ends_with('/') {
                fs::create_dir_all(
                    &registry
                        .parent()
                        .context("Could not get parent of nonexisting path")?,
                )
                .with_context(|| {
                    format!(
                        "unable to create registry directory: '{}'",
                        registry.display()
                    )
                })?;
                TagRegistry::load(&registry).unwrap_or_else(|_| TagRegistry::new(&registry))
            } else {
                print_err(format!(
                    "{} last error is a directory path. Using default registry: {}",
                    registry.display().to_string().green(),
                    state_file.display().to_string().green(),
                ));
                TagRegistry::load(&state_file).unwrap_or_else(|_| TagRegistry::new(&state_file))
            }
        } else {
            TagRegistry::load(&state_file).unwrap_or_else(|_| TagRegistry::new(&state_file))
        };

        let extensions = opts
            .extension
            .clone()
            .map(|ext| {
                RegexSetBuilder::new(
                    ext.into_iter()
                        .map(|e| e.trim_start_matches('.').to_owned())
                        .map(|e| format!(r".\.{}$", regex::escape(e.as_str()))),
                )
                .case_insensitive(true)
                .build()
            })
            .transpose()?;

        let excludes = opts
            .exclude
            .clone()
            .map(|v| v.iter().map(|p| String::from("!") + p.as_str()).collect())
            .unwrap_or_else(Vec::new);

        Ok(App {
            base_dir,
            max_depth: if opts.max_depth.is_some() {
                opts.max_depth
            } else {
                config.max_depth
            },
            base_color,
            colors,
            ignores: config.ignores,
            global: opts.global,
            registry,
            case_insensitive: opts.case_insensitive,
            pat_regex: opts.regex,
            ls_colors: opts.ls_colors,
            color_when: color_when.to_string(),
            extension: extensions,
            exclude: excludes,
        })
    }

    pub fn save_registry(&mut self) {
        if let Err(e) = self.registry.save() {
            eprintln!("failed to save registry - {}", e);
        }
    }

    pub(crate) fn run_command(&mut self, cmd: Command) {
        if self.color_when == "never" {
            colored::control::SHOULD_COLORIZE.set_override(false);
        } else if self.color_when == "always" {
            colored::control::SHOULD_COLORIZE.set_override(true);
        }

        match cmd {
            Command::List(ref opts) => self.list(opts),
            Command::Set(opts) => self.set(&opts),
            Command::Rm(ref opts) => self.rm(opts),
            Command::Clear(ref opts) => self.clear(opts),
            Command::Search(ref opts) => self.search(opts),
            Command::Cp(ref opts) => self.cp(opts),
            Command::Edit(ref opts) => self.edit(opts),
            Command::PrintCompletions(ref opts) => self.print_completions(opts),
            Command::CleanCache => self.clean_cache(),
        }
    }

    fn clean_cache(&mut self) {
        self.registry.clear();
        if let Err(e) = self.registry.save() {
            println!("{:?}", e);
        } else {
            println!(
                "{} {}: {}",
                "✔".green().bold(),
                "CACHE CLEARED".red().bold(),
                self.registry
                    .path
                    .to_path_buf()
                    .display()
                    .to_string()
                    .green()
                    .bold()
            );
        }
    }

    fn list(&self, opts: &ListOpts) {
        log::debug!("Using registry: {}", self.registry.path.display());
        let stdout = io::stdout();
        let std_lock = stdout.lock();
        let handle = io::BufWriter::new(std_lock);
        let mut tab_handle = tabwriter::TabWriter::new(handle);

        match opts.object {
            ListObject::Files {
                with_tags,
                formatted,
                garrulous,
            } => {
                let global_opts = |local: String, global: String| {
                    if garrulous {
                        ternary!(self.global, println!("{}", global), println!("{}", local));
                    } else if self.global {
                        print!("{}", global);
                    } else {
                        print!("{}", local);
                    }
                };

                for (id, file) in self.registry.list_entries_and_ids() {
                    // Skips paths that are not contained within one another to respect the `-d`
                    // flag Global is just another way to specify -d ~ (list files locally
                    // by default)
                    if !self.global && !contained_path(file.path(), &self.base_dir) {
                        continue;
                    }

                    if opts.raw {
                        global_opts(
                            raw_local_path(file.path(), &self.base_dir),
                            file.path().display().to_string(),
                        );
                    } else if !formatted {
                        global_opts(
                            fmt_local_path(
                                file.path(),
                                &self.base_dir,
                                self.base_color,
                                self.ls_colors,
                            ),
                            fmt_path(file.path(), self.base_color, self.ls_colors),
                        );
                    }

                    if with_tags {
                        let tags = self
                            .registry
                            .list_entry_tags(*id)
                            .unwrap_or_default()
                            .iter()
                            .map(|t| {
                                if opts.raw {
                                    t.name().to_owned()
                                } else {
                                    fmt_tag(t).to_string()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" ");

                        if formatted {
                            writeln!(
                                tab_handle,
                                "{}\t{}",
                                ternary!(
                                    self.global,
                                    fmt_path(file.path(), self.base_color, self.ls_colors),
                                    fmt_local_path(
                                        file.path(),
                                        &self.base_dir,
                                        self.base_color,
                                        self.ls_colors,
                                    )
                                ),
                                tags
                            )
                            .expect("Unable to write to tab handler");
                        } else if garrulous {
                            println!("\t{}", tags);
                        } else {
                            println!(": {}", tags);
                        }
                    } else {
                        println!();
                    }
                }
                if formatted {
                    tab_handle.flush().expect("Unable to flush tab handler");
                }
            },
            ListObject::Tags { for_completions } => {
                // I think both id and entry has to be listed here to be able to respect current
                // directory This is really dirty
                let mut utags = Vec::new();
                for (&id, file) in self.registry.list_entries_and_ids() {
                    if !self.global && !contained_path(file.path(), &self.base_dir) {
                        continue;
                    }
                    let tags = self
                        .registry
                        .list_entry_tags(id)
                        .map(|tags| {
                            tags.iter().fold(String::new(), |mut acc, t| {
                                acc.push_str(&format!(
                                    "{} ",
                                    if opts.raw {
                                        t.name().white()
                                    } else {
                                        fmt_tag(t)
                                    }
                                ));
                                acc
                            })
                        })
                        .unwrap_or_default()
                        .clone();
                    utags.push(tags);
                }

                utags
                    .iter()
                    .fold(HashMap::new(), |mut acc, t| {
                        *acc.entry(t.clone()).or_insert(0) += 1;
                        acc
                    })
                    .iter()
                    .for_each(|(tag, count)| {
                        writeln!(
                            tab_handle,
                            "{}\t{}",
                            ternary!(
                                opts.raw,
                                count.to_string().white(),
                                count.to_string().green().bold()
                            ),
                            tag
                        )
                        .expect("Unable to write to tab handler");
                    });

                if for_completions {
                    utags.iter().for_each(|tag| println!("{}", tag));
                } else {
                    tab_handle.flush().expect("Unable to flush tab handler");
                }
            },
        }
    }

    fn set(&mut self, opts: &SetOpts) {
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
                            print_err(format!("{}", e));
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
            print_err(format!("{}", e));
        }
    }

    fn rm(&mut self, opts: &RmOpts) {
        // Global will match a glob only against files that are tagged
        // Could add a fixed string option
        log::debug!("Using registry: {}", self.registry.path.display());
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive);
        log::debug!("Compiled pattern: {}", re);

        if self.global {
            let ctags = opts.tags.iter().collect::<Vec<_>>();
            let exclude_pattern =
                regex_builder(self.exclude.join("|").as_str(), self.case_insensitive);
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
                                    tag.to_owned(),
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
        } else {
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

                    let id = selfu.registry.find_entry(entry.path());
                    let tags = opts
                        .tags
                        .iter()
                        .map(|tag| {
                            if let Some(id) = id {
                                selfu.registry.untag_by_name(tag, id);
                            }
                            entry.get_tag(tag)
                        })
                        .collect::<Vec<_>>();

                    if tags.is_empty() {
                        return;
                    }

                    println!(
                        "{}:",
                        fmt_path(entry.path(), selfu.base_color, selfu.ls_colors)
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
                    selfu.save_registry();
                },
            ) {
                print_err(format!("{}", e));
            }
        }
    }

    fn clear(&mut self, opts: &ClearOpts) {
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive);

        if self.global {
            let exclude_pattern =
                regex_builder(self.exclude.join("|").as_str(), self.case_insensitive);
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
            log::debug!("Saving registry...");
            self.save_registry();
        } else {
            let selfc = Arc::new(Mutex::new(self.clone()));

            if let Err(e) = reg_ok(
                Arc::new(re),
                &Arc::new(self.clone()),
                move |entry: &ignore::DirEntry| {
                    let selfc = Arc::clone(&selfc);
                    let mut selfu = selfc.lock().unwrap();

                    if let Some(id) = selfu.registry.find_entry(entry.path()) {
                        selfu.registry.clear_entry(id);
                    }
                    match entry.has_tags() {
                        Ok(has_tags) =>
                            if has_tags {
                                println!(
                                    "{}:",
                                    fmt_path(entry.path(), selfu.base_color, selfu.ls_colors)
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
                    selfu.save_registry();
                },
            ) {
                print_err(format!("{}", e));
            }
        }
    }

    fn search(&self, opts: &SearchOpts) {
        // FIX: Cannot exclude file paths
        // FIX: Returns all files regardless of tags
        // ADD: option to search by file name

        if opts.any {
            for (&id, entry) in self.registry.list_entries_and_ids() {
                if opts.raw {
                    println!("{}", entry.path().display());
                } else {
                    let tags = self
                        .registry
                        .list_entry_tags(id)
                        .map(|tags| {
                            tags.iter().fold(String::new(), |mut acc, t| {
                                acc.push_str(&format!("{} ", fmt_tag(t)));
                                acc
                            })
                        })
                        .unwrap_or_default();
                    println!(
                        "{}: {}",
                        fmt_path(entry.path(), self.base_color, self.ls_colors),
                        tags
                    )
                }
            }
        } else if opts.execute.is_some() || opts.execute_batch.is_some() {
            let command = if let Some(cmd) = &opts.execute {
                Some(CommandTemplate::new(cmd))
            } else {
                opts.execute_batch
                    .as_ref()
                    .map(|cmd| CommandTemplate::new_batch(cmd).expect("Invalid batch command"))
            };

            let cmd = command.unwrap();
            run_command(
                &(self.clone()),
                &Arc::new(cmd),
                &Arc::new(opts.tags.clone()),
            );
        } else {
            for id in self.registry.list_entries_with_tags(&opts.tags) {
                let path = match self.registry.get_entry(id) {
                    Some(entry) =>
                        if !self.global && !contained_path(entry.path(), &self.base_dir) {
                            continue;
                        } else {
                            entry.path()
                        },
                    None => continue,
                };
                if opts.raw {
                    println!("{}", path.display());
                } else {
                    let tags = self
                        .registry
                        .list_entry_tags(id)
                        .map(|tags| {
                            tags.iter().fold(String::new(), |mut acc, t| {
                                acc.push_str(&format!("{} ", fmt_tag(t)));
                                acc
                            })
                        })
                        .unwrap_or_default();

                    println!(
                        "{}: {}",
                        ternary!(
                            self.global,
                            fmt_path(path, self.base_color, self.ls_colors),
                            fmt_local_path(path, &self.base_dir, self.base_color, self.ls_colors)
                        ),
                        tags
                    )
                }
            }
        }
    }

    fn cp(&mut self, opts: &CpOpts) {
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
                    print_err(format!("{}", e));
                }
                log::debug!("Saving registry...");
                self.save_registry();
            },
            Err(e) => eprintln!(
                "failed to get source tags from `{}` - {}",
                path.display(),
                e
            ),
        }
    }

    fn edit(&mut self, opts: &EditOpts) {
        let color = match parse_color(&opts.color) {
            Ok(color) => color,
            Err(e) => {
                print_err(format!("{}", e));
                return;
            },
        };
        let old_tag = self.registry.get_tag(&opts.tag).cloned();
        if self.registry.update_tag_color(&opts.tag, color) {
            if let Some(old_tag) = old_tag {
                let new_tag = self.registry.get_tag(&opts.tag);
                println!("{} ==> {}", fmt_tag(&old_tag), fmt_tag(new_tag.unwrap()))
            }
        }
        log::debug!("Saving registry...");
        self.save_registry();
    }

    fn print_completions(&self, opts: &CompletionsOpts) {
        fn replace(haystack: &mut String, needle: &str, replacement: &str) -> Result<()> {
            if let Some(index) = haystack.find(needle) {
                haystack.replace_range(index..index + needle.len(), replacement);
                Ok(())
            } else {
                Err(anyhow!(
                    "Failed to find text:\n{}\n…in completion script:\n{}",
                    needle,
                    haystack
                ))
            }
        }

        use clap_generate::{
            generate,
            generators::{Bash, Elvish, Fish, PowerShell, Zsh},
        };

        let mut app = Opts::into_app();

        let buffer = Vec::new();
        let mut cursor = io::Cursor::new(buffer);

        match opts.shell {
            Shell::Bash => generate::<Bash, _>(&mut app, APP_NAME, &mut cursor),
            Shell::Elvish => generate::<Elvish, _>(&mut app, APP_NAME, &mut cursor),
            Shell::Fish => generate::<Fish, _>(&mut app, APP_NAME, &mut cursor),
            Shell::PowerShell => generate::<PowerShell, _>(&mut app, APP_NAME, &mut cursor),
            Shell::Zsh => generate::<Zsh, _>(&mut app, APP_NAME, &mut cursor),
        }

        let buffer = cursor.into_inner();
        let mut script = String::from_utf8(buffer).expect("Clap completion not UTF-8");

        match opts.shell {
            Shell::Zsh =>
                for (needle, replacement) in comp_helper::ZSH_COMPLETION_REP {
                    replace(&mut script, needle, replacement)
                        .expect("Failed to replace completion script");
                },
            _ => println!(),
        }

        println!("{}", script.trim())
    }
}
