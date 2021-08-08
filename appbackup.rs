use anyhow::{Context, Result};
use clap::IntoApp;
use colored::{Color, Colorize};
use globwalk::DirEntry;
use std::{io, fs};
use std::path::PathBuf;
use std::collections::HashMap;

use tabular::{Table, Row};

use crate::config::Config;
use crate::opt::{
    ClearOpts, Command, CompletionsOpts, CpOpts, EditOpts, ListObject, ListOpts, Opts, RmOpts,
    SearchOpts, SetOpts, Shell, APP_NAME,
};
use crate::registry::{EntryData, TagRegistry};
use crate::util::{contained_path, fmt_err, fmt_ok, fmt_path, fmt_local_path, raw_local_path, fmt_tag, glob_ok, macos_dirs};
use crate::DEFAULT_COLORS;
use wutag_core::color::parse_color;
use wutag_core::tag::{DirEntryExt, Tag, clear_tags, list_tags, has_tags};

pub struct App {
    pub base_dir: PathBuf,
    pub max_depth: Option<usize>,
    pub colors: Vec<Color>,
    pub no_color: bool,
    pub registry: TagRegistry,
}

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

impl App {
    pub fn run(opts: Opts, config: Config) -> Result<()> {
        let mut app = Self::new(&opts, config)?;
        app.run_command(opts.cmd);

        Ok(())
    }
    pub fn new(opts: &Opts, config: Config) -> Result<App> {
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


        let cache_dir = macos_dirs(dirs::cache_dir(), ".cache");
        let state_file = cache_dir.unwrap().join("wutag.registry");

        let registry = if let Some(registry) = &opts.reg {
            let meta = fs::metadata(&registry);

            match meta {
                Ok(file) => {
                    if file.is_file() {
                        TagRegistry::load(&registry).unwrap_or_else(|_| TagRegistry::new(&registry))
                    } else {
                        eprintln!("{}",
                            fmt_err(format!("{} is not a file. Using default", registry.display().to_string()))
                        );
                        TagRegistry::load(&state_file).unwrap_or_else(|_| TagRegistry::new(&state_file))
                    }
                },
                _ => {
                    if !registry.display().to_string().ends_with("/") {
                        fs::create_dir_all(&registry.parent().expect("Could not get parent of nonexisting path"))
                            .with_context(|| {
                                format!(
                                    "unable to create registry directory: '{}'",
                                    registry.display()
                                )
                        })?;
                        TagRegistry::load(&registry).unwrap_or_else(|_| TagRegistry::new(&registry))
                    } else {
                        eprintln!("{}",
                            fmt_err(format!("{} is not a file. Using default", registry.display().to_string()))
                        );
                        TagRegistry::load(&state_file).unwrap_or_else(|_| TagRegistry::new(&state_file))
                    }
                }
            }
        } else {
            TagRegistry::load(&state_file).unwrap_or_else(|_| TagRegistry::new(&state_file))
        };

        Ok(App {
            base_dir,
            max_depth: if opts.max_depth.is_some() {
                opts.max_depth
            } else {
                config.max_depth
            },
            colors,
            no_color: opts.no_color,
            registry,
        })
    }

    fn save_registry(&mut self) {
        if let Err(e) = self.registry.save() {
            eprintln!("failed to save registry - {}", e);
        }
    }

    pub fn run_command(&mut self, cmd: Command) {
        if self.no_color {
            colored::control::SHOULD_COLORIZE.set_override(false);
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
        }
    }

    // TODO: List files locally without full path
    fn list(&self, opts: &ListOpts) {
        match opts.object {
            ListObject::Files { with_tags, formatted, garrulous } => {

                let global_opts = |local: String, global: String| {
                    if garrulous {
                        if !opts.global {
                            println!("{}", local);
                        } else {
                            println!("{}", global);
                        }
                    } else {
                        if !opts.global {
                            print!("{}", local);
                        } else {
                            print!("{}", global);
                        }
                    }
                };

                let mut table = Table::new("{:<}   {:<}");

                for (id, file) in self.registry.list_entries_and_ids() {
                    // Skips paths that are not contained within one another to respect the `-d` flag
                    // Global is just another way to specify -d ~ (list files locally by default)
                    if !opts.global && !contained_path(file.path(), &self.base_dir) {
                        continue;
                    }

                    if opts.raw {
                        global_opts(raw_local_path(file.path(), &self.base_dir),
                            file.path().display().to_string()
                        );
                    } else if !formatted {
                        global_opts(fmt_local_path(file.path(), &self.base_dir),
                            fmt_path(file.path())
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
                            table.add_row(Row::new()
                                .with_cell(fmt_path(file.path()))
                                .with_cell(tags)
                            );
                        } else {
                            if garrulous {
                                println!("\t{}", tags);
                            } else {
                                println!(": {}", tags);
                            }
                        }
                    } else {
                        println!();
                    }
                }
                if formatted { println!("{}", table); }
            }
            ListObject::Tags => {
                // TODO: Respect raw
                // I think both id and entry has to be listed here to be able to respect current directory
                // This is really dirty
                let mut utags = Vec::new();
                let mut table = Table::new("{:<}  |  {:<}");
                for (&id, file) in self.registry.list_entries_and_ids() {
                    if !opts.global && !contained_path(file.path(), &self.base_dir) {
                        continue;
                    }
                    let tags = self
                        .registry
                        .list_entry_tags(id)
                        .map(|tags| {
                            tags.iter().fold(String::new(), |mut acc, t| {
                                acc.push_str(&format!("{} ", fmt_tag(t)));
                                acc
                            })
                        })
                        .unwrap_or_default()
                        .clone();
                    utags.push(tags);
                }

                utags.iter()
                    .fold(HashMap::new(), |mut acc, t| {
                        *acc.entry(t.clone()).or_insert(0) += 1;
                        acc
                })
                .iter()
                .for_each(|(tag, count)| {
                    table.add_row(Row::new()
                        .with_cell(count.to_string().green().bold())
                        .with_cell(tag)
                    );
                });
                // Can't get this to work if cells are reversed
                println!("{}", table);
            }
        }
    }

    fn set(&mut self, opts: &SetOpts) {
        let tags = opts
            .tags
            .iter()
            .map(|t| {
                if let Some(t) = self.registry.get_tag(t) {
                    t.clone()
                } else {
                    Tag::random(t, &self.colors)
                }
            })
            .collect::<Vec<_>>();

        if let Err(e) = glob_ok(
            &opts.pattern,
            &self.base_dir.clone(),
            self.max_depth,
            |entry: &DirEntry| {
                println!("{}:", fmt_path(entry.path()));
                tags.iter().for_each(|tag| {
                    if let Err(e) = entry.tag(tag) {
                        err!('\t', e, entry);
                    } else {
                        let entry = EntryData::new(entry.path());
                        let id = self.registry.add_or_update_entry(entry);
                        self.registry.tag_entry(tag, id);
                        print!("\t{} {}", "+".bold().green(), fmt_tag(tag));
                    }
                });
                println!();
            },
        ) {
            eprintln!("{}", fmt_err(e));
        }

        self.save_registry();
    }

    fn rm(&mut self, opts: &RmOpts) {
        // Global will match a glob against only files that are tagged
        // There may be a better way to do this. Lot's of similar code here
        if opts.global {
            let pat = glob::Pattern::new(&opts.pattern).unwrap();
            let ctags = opts.tags.iter().collect::<Vec<_>>();
            for (&id, entry) in self.registry.clone().list_entries_and_ids() {
                if pat.matches(entry.path().to_str().unwrap()) {
                    list_tags(entry.path())
                        .map(|tags| {
                            tags.iter().fold(Vec::new(), |mut acc, tag| {
                                acc.push(
                                    (
                                        ctags.iter().find(|c| **c == &tag.to_string()),
                                        tag.to_owned()
                                    )
                                );
                                acc
                            })
                        })
                        .unwrap_or_default()
                        .iter()
                        .for_each(|(search, realtag)| {
                            if search.is_some() {
                                // println!("SEARCH: {:?} REAL: {:?}", search, realtag);
                                self.registry.untag_by_name(search.unwrap(), id);
                                println!("{}:", fmt_path(entry.path()));

                                if let Err(e) = realtag.remove_from(entry.path()) {
                                    err!('\t', e, entry);
                                } else {
                                    print!("\t{} {}", "X".bold().red(), fmt_tag(realtag));
                                }

                                println!();
                            }
                        });
                }
            }
        } else {
            if let Err(e) = glob_ok(
                &opts.pattern,
                &self.base_dir.clone(),
                self.max_depth,
                |entry: &DirEntry| {
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

                    println!("{}:", fmt_path(entry.path()));
                    tags.iter().for_each(|tag| {
                        let tag = match tag {
                            Ok(tag) => tag,
                            Err(e) => {
                                err!('\t', e, entry);
                                return;
                            }
                        };
                        if let Err(e) = entry.untag(tag) {
                            err!('\t', e, entry);
                        } else {
                            print!("\t{} {}", "X".bold().red(), fmt_tag(tag));
                        }
                    });
                    println!();
                },
            ) {
                eprintln!("{}", fmt_err(e));
            }
        }
        self.save_registry();
    }

    fn clear(&mut self, opts: &ClearOpts) {
        if opts.global {
            let pat = glob::Pattern::new(&opts.pattern).unwrap();
            for (&id, entry) in self.registry.clone().list_entries_and_ids() {
                if pat.matches(entry.path().to_str().unwrap()) {
                    self.registry.clear_entry(id);
                    match has_tags(entry.path()) {
                        Ok(has_tags) => {
                            if has_tags {
                                println!("{}:", fmt_path(entry.path()));
                                if let Err(e) = clear_tags(entry.path()) {
                                    err!('\t', e, entry);
                                } else {
                                    println!("\t{}", fmt_ok("cleared"));
                                }
                            }
                        }
                        Err(e) => {
                            err!(e, entry);
                        }
                    }
                }
            }
        } else {
            if let Err(e) = glob_ok(
                &opts.pattern,
                &self.base_dir.clone(),
                self.max_depth,
                |entry: &DirEntry| {
                    if let Some(id) = self.registry.find_entry(entry.path()) {
                        self.registry.clear_entry(id);
                    }
                    match entry.has_tags() {
                        Ok(has_tags) => {
                            if has_tags {
                                println!("{}:", fmt_path(entry.path()));
                                if let Err(e) = entry.clear_tags() {
                                    err!('\t', e, entry);
                                } else {
                                    println!("\t{}", fmt_ok("cleared"));
                                }
                            }
                        }
                        Err(e) => {
                            err!(e, entry);
                        }
                    }
                },
            ) {
                eprintln!("{}", fmt_err(e));
            }
        }
        self.save_registry();
    }

    fn search(&self, opts: &SearchOpts) {
        // FIX: Returns all files regardless of tags
        // The else in this statement does 'any' automatically
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
                    println!("{}: {}", fmt_path(entry.path()), tags)
                }
            }
        } else {
            for id in self.registry.list_entries_with_tags(&opts.tags) {
                let path = match self.registry.get_entry(id) {
                    Some(entry) => {
                        if !contained_path(entry.path(), &self.base_dir) {
                            continue
                        } else {
                            entry.path()
                        }
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
                    println!("{}: {}", fmt_path(path), tags)
                }
            }
        }
    }

    fn cp(&mut self, opts: &CpOpts) {
        let path = opts.input_path.as_path();
        match list_tags(path) {
            Ok(tags) => {
                if let Err(e) = glob_ok(
                    &opts.pattern,
                    &self.base_dir.clone(),
                    self.max_depth,
                    |entry: &DirEntry| {
                        println!("{}:", fmt_path(entry.path()));
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
                    eprintln!("{}", fmt_err(e));
                }

                self.save_registry();
            }
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
                eprintln!("{}", fmt_err(e));
                return;
            }
        };
        let old_tag = self.registry.get_tag(&opts.tag).cloned();
        if self.registry.update_tag_color(&opts.tag, color) {
            if let Some(old_tag) = old_tag {
                let new_tag = self.registry.get_tag(&opts.tag);
                println!("{} ==> {}", fmt_tag(&old_tag), fmt_tag(new_tag.unwrap()))
            }
        }

        self.save_registry();
    }

    fn print_completions(&self, opts: &CompletionsOpts) {
        use clap_generate::{
            generate,
            generators::{Bash, Elvish, Fish, PowerShell, Zsh},
        };

        let mut app = Opts::into_app();

        match opts.shell {
            Shell::Bash => generate::<Bash, _>(&mut app, APP_NAME, &mut io::stdout()),
            Shell::Elvish => generate::<Elvish, _>(&mut app, APP_NAME, &mut io::stdout()),
            Shell::Fish => generate::<Fish, _>(&mut app, APP_NAME, &mut io::stdout()),
            Shell::PowerShell => generate::<PowerShell, _>(&mut app, APP_NAME, &mut io::stdout()),
            Shell::Zsh => generate::<Zsh, _>(&mut app, APP_NAME, &mut io::stdout()),
        }
    }
}
