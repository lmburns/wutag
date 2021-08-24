use anyhow::{Context, Result};
use atty::Stream;
use clap::IntoApp;
use cli_table::{
    format::{Border, Justify, Separator},
    print_stdout, Cell, ColorChoice, Style, Table,
};
use colored::{Color, Colorize};
use crossbeam_channel as channel;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use regex::bytes::{RegexSet, RegexSetBuilder};
use shellexpand::LookupError;

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    env,
    ffi::OsStr,
    fs, io,
    io::prelude::*,
    iter,
    path::PathBuf,
    process,
    sync::{Arc, Mutex},
};

use crate::{
    comp_helper,
    config::Config,
    err,
    exe::{
        job::{receiver, sender, WorkerResult},
        CommandTemplate,
    },
    filesystem::{contained_path, osstr_to_bytes, FileTypes},
    opt::{
        ClearOpts, Command, CompletionsOpts, CpOpts, EditOpts, ListObject, ListOpts, Opts, RmOpts,
        SearchOpts, SetOpts,
    },
    registry::{EntryData, TagRegistry},
    ternary,
    util::{
        fmt_err, fmt_local_path, fmt_ok, fmt_path, fmt_tag, gen_completions, glob_builder,
        raw_local_path, reg_ok, regex_builder, replace,
    },
    wutag_error, DEFAULT_BASE_COLOR, DEFAULT_BORDER_COLOR, DEFAULT_COLORS,
};

use wutag_core::{
    color::{parse_color, parse_color_cli_table},
    tag::{clear_tags, has_tags, list_tags, DirEntryExt, Tag, DEFAULT_COLOR},
};

#[derive(Clone, Debug)]
pub struct App {
    pub base_dir:         PathBuf,
    pub max_depth:        Option<usize>,
    pub base_color:       Color,
    pub border_color:     cli_table::Color,
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
    pub file_type:        Option<FileTypes>,
}

impl App {
    pub(crate) fn run(opts: Opts, config: Config) -> Result<()> {
        let mut app = Self::new(&opts, config)?;
        log::trace!("CONFIGURATION: {:#?}", app);
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

        let border_color = config
            .border_color
            .map(parse_color_cli_table)
            .transpose()?
            .unwrap_or(DEFAULT_BORDER_COLOR);

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
                                var_name: "Unkown environment variable".into(),
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
                wutag_error!(
                    "{} is not a file. Using default registry: {}",
                    registry.display().to_string().green(),
                    state_file.display().to_string().green(),
                );
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
                wutag_error!(
                    "{} last error is a directory path. Using default registry: {}",
                    registry.display().to_string().green(),
                    state_file.display().to_string().green(),
                );
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

        let file_types = opts.file_type.clone().map(|vals| {
            let mut ftypes = FileTypes::default();
            for v in vals {
                match v.as_str() {
                    "f" | "file" => ftypes.files = true,
                    "d" | "directory" => ftypes.directories = true,
                    "l" | "symlink" => ftypes.symlinks = true,
                    "b" | "block" => ftypes.block_devices = true,
                    "c" | "char" => ftypes.char_devices = true,
                    "x" | "executable" => {
                        ftypes.executables_only = true;
                        ftypes.files = true;
                    },
                    "e" | "empty" => ftypes.empty_only = true,
                    "s" | "socket" => ftypes.sockets = true,
                    "F" | "fifo" => ftypes.fifos = true,
                    _ => unreachable!(),
                }
            }
            if ftypes.empty_only && !(ftypes.files || ftypes.directories) {
                ftypes.files = true;
                ftypes.directories = true;
            }

            ftypes
        });
        log::debug!("FileTypes: {:#?}", file_types);

        Ok(App {
            base_dir,
            max_depth: if opts.max_depth.is_some() {
                opts.max_depth
            } else {
                config.max_depth
            },
            base_color,
            border_color,
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
            file_type: file_types,
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
            wutag_error!("{:?}", e);
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

        let mut table = vec![];
        let colorchoice = match self.color_when {
            ref s if s == "always" => ColorChoice::Always,
            ref s if s == "never" => ColorChoice::Never,
            _ => ColorChoice::Auto,
        };

        match opts.object {
            ListObject::Files {
                with_tags,
                formatted,
                border,
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
                            table.push(vec![
                                ternary!(
                                    self.global,
                                    fmt_path(file.path(), self.base_color, self.ls_colors),
                                    fmt_local_path(
                                        file.path(),
                                        &self.base_dir,
                                        self.base_color,
                                        self.ls_colors,
                                    )
                                )
                                .cell(),
                                tags.cell().justify(Justify::Right),
                            ]);
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
                    print_stdout(if border {
                        table
                            .table()
                            .foreground_color(Some(self.border_color))
                            .color_choice(colorchoice)
                    } else {
                        table
                            .table()
                            .border(Border::builder().build())
                            .separator(Separator::builder().build())
                    })
                    .expect("Unable to print table");
                }
            },
            ListObject::Tags {
                for_completions,
                border,
            } => {
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
                        table.push(vec![
                            tag.cell(),
                            ternary!(
                                opts.raw,
                                count.to_string().white(),
                                count.to_string().green().bold()
                            )
                            .cell()
                            .justify(Justify::Right),
                        ])
                    });

                if for_completions {
                    utags.iter().for_each(|tag| println!("{}", tag));
                } else {
                    print_stdout(if border {
                        table
                            .table()
                            .foreground_color(Some(self.border_color))
                            .color_choice(colorchoice)
                    } else {
                        table
                            .table()
                            .border(Border::builder().build())
                            .separator(Separator::builder().build())
                    })
                    .expect("Unable to print table");
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
                wutag_error!("{}", e);
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
                wutag_error!("{}", e);
            }
        }
    }

    fn search(&self, opts: &SearchOpts) {
        let pat = if self.pat_regex {
            String::from(&opts.pattern)
        } else {
            glob_builder(&opts.pattern)
        };

        let re = regex_builder(&pat, self.case_insensitive);
        log::debug!("Compiled pattern: {}", re);

        #[allow(clippy::manual_map)]
        let command = if let Some(cmd) = &opts.execute {
            Some(CommandTemplate::new(cmd))
        } else if let Some(cmd) = &opts.execute_batch {
            Some(CommandTemplate::new_batch(cmd).expect("Invalid batch command"))
        } else {
            None
        };

        let app = Arc::new(self.clone());
        let opts = Arc::new(opts.clone());
        let re = Arc::new(re);
        let command = command.map(Arc::new);

        let (tx, rx) = channel::unbounded::<WorkerResult>();

        let rec = receiver(&app, &opts, command, rx);
        sender(&app, &opts, re, tx);
        rec.join().unwrap();
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

    fn edit(&mut self, opts: &EditOpts) {
        // TODO: Move this to search and add as a feature
        // TODO: Add tag search
        if opts.view {
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
            let exclude_pattern =
                regex_builder(self.exclude.join("|").as_str(), self.case_insensitive);

            let mut map = BTreeMap::new();
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
                    // if !opts.tags.is_empty() && !app.registry.entry_has_tags(id, &opts.tags) {
                    //     continue;
                    // }

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
            let tag_yaml = serde_yaml::to_string(&map).expect("err");
            let mut rng = thread_rng();
            let mut tmp_path = env::temp_dir();
            let fname = iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .map(char::from)
                .take(12)
                .collect::<String>();

            tmp_path.push(format!("{}-{}", env!("CARGO_PKG_NAME"), fname));
            tmp_path.set_extension("yml");

            let mut tmp_file: fs::File = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&tmp_path)
                .unwrap_or_else(|_| panic!("could not create tmp file: '{}'", tmp_path.display()));

            tmp_file
                .write_all(tag_yaml.as_bytes())
                .unwrap_or_else(|_| panic!("could not create tmp file: '{}'", tmp_path.display()));

            tmp_file
                .flush()
                .unwrap_or_else(|_| panic!("could not create tmp file: '{}'", tmp_path.display()));

            process::Command::new(opts.editor.to_string())
                .arg(&tmp_path)
                .status()
                .expect("could not spawn editor");

            let emap: BTreeMap<String, Vec<String>> =
                serde_yaml::from_slice(&fs::read(&tmp_path).expect("failed to read tagged file"))
                    .expect("failed to deserialize config file");

            let diff = emap
                .iter()
                .find(|(key, val)| map.iter().any(|(k, v)| k == *key && v != *val));

            if let Some(_diff) = diff {
                log::debug!("Diffs: {:#?}", diff);
                let (entry, tags) = _diff;
                let entry = &PathBuf::from(entry);
                let entry = &fs::canonicalize(&entry).unwrap();

                if let Some(id) = self.registry.find_entry(entry) {
                    self.registry.clear_entry(id);
                }

                macro_rules! bold_entry {
                    ($entry:ident) => {
                        $entry.display().to_string().bold()
                    };
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

                let tags = tags
                    .iter()
                    .map(|t| {
                        if let Some(t) = self.registry.get_tag(t) {
                            log::debug!("Got tag: {:?}", t);
                            t.clone()
                        } else {
                            log::debug!("Setting random tag: {:?}", t);
                            Tag::random(t, &self.colors)
                        }
                    })
                    .collect::<Vec<_>>();

                for tag in tags.iter() {
                    if let Err(e) = entry.tag(tag) {
                        wutag_error!("{} {}", e, bold_entry!(entry))
                    } else {
                        let entry = EntryData::new(entry);
                        let id = self.registry.add_or_update_entry(entry);
                        self.registry.tag_entry(tag, id);
                        print!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                    }
                }
            }
            self.save_registry();
        } else if let Some(_color) = &opts.color {
            let color = match parse_color(_color) {
                Ok(color) => color,
                Err(e) => {
                    wutag_error!("{}", e);
                    return;
                },
            };

            // Can unwrap here, as Clap will throw error if color is passed without tag
            let tag = opts.tag.as_ref().unwrap();
            let old_tag = self.registry.get_tag(tag).cloned();
            if self.registry.update_tag_color(tag, color) {
                if let Some(old_tag) = old_tag {
                    let new_tag = self.registry.get_tag(tag);
                    println!("{} ==> {}", fmt_tag(&old_tag), fmt_tag(new_tag.unwrap()))
                }
            }
            log::debug!("Saving registry...");
        }
        self.save_registry();
    }

    fn print_completions(&self, opts: &CompletionsOpts) {
        use clap_generate::{generators::*, Shell};

        let mut app = Opts::into_app();

        let buffer = Vec::new();
        let mut cursor = io::Cursor::new(buffer);

        match opts.shell {
            Shell::Bash => gen_completions::<Bash>(&mut app, &mut cursor),
            Shell::Elvish => gen_completions::<Elvish>(&mut app, &mut cursor),
            Shell::Fish => gen_completions::<Fish>(&mut app, &mut cursor),
            Shell::PowerShell => gen_completions::<PowerShell>(&mut app, &mut cursor),
            Shell::Zsh => gen_completions::<Zsh>(&mut app, &mut cursor),
            _ => (),
        }

        let buffer = cursor.into_inner();
        let mut script = String::from_utf8(buffer).expect("Clap completion not UTF-8");

        // Replace Zsh completion output to make it better
        // I may work on other shells, but am very familiar with Zsh, so that is why it
        // is the only one so far
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
