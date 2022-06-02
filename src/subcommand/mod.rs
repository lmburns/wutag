//! Subcommands found within the `wutag` crate

pub(crate) mod clean_cache;
pub(crate) mod clear;
pub(crate) mod cp;
pub(crate) mod edit;
pub(crate) mod info;
pub(crate) mod list;
pub(crate) mod print_completions;
pub(crate) mod repair;
pub(crate) mod rm;
pub(crate) mod search;
pub(crate) mod set;
pub(crate) mod view;

// TODO: Virtual filesystem
// TODO: Repair command (crawl a given directory looking for xattrs)
// TODO: Info command

// TODO: Rename/merge/copy tags

use crate::{
    config::{Config, EncryptConfig},
    consts::{DEFAULT_BASE_COLOR, DEFAULT_BORDER_COLOR, DEFAULT_COLORS},
    fail,
    filesystem::FileTypes,
    opt::{Colorization, Command, Opts},
    oregistry,
    oregistry::TagRegistry,
    registry::{types::tag::Tag, Registry},
    ui,
    utils::{
        color::{parse_color, parse_color_cli_table},
        fmt,
    },
    wutag_error, wutag_fatal,
    xattr::tag::DirEntryExt,
};
use anyhow::{Context, Result};
use colored::{Color, ColoredString, Colorize};
use regex::bytes::{RegexSet, RegexSetBuilder};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex},
};

/// A structure that is built from a parsed `Config` and parsed `Opts`
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug)]
pub(crate) struct App {
    pub(crate) base_color:           Color,
    pub(crate) base_dir:             PathBuf,
    pub(crate) case_insensitive:     bool,
    pub(crate) case_sensitive:       bool,
    pub(crate) color_when:           Colorization,
    pub(crate) colors:               Vec<Color>,
    pub(crate) exclude:              Vec<String>,
    pub(crate) extension:            Option<RegexSet>,
    pub(crate) file_type:            Option<FileTypes>,
    pub(crate) follow_symlinks:      bool,
    pub(crate) show_duplicates:      bool,
    pub(crate) format:               String,
    pub(crate) global:               bool,
    pub(crate) ignores:              Option<Vec<String>>,
    pub(crate) ls_colors:            bool,
    pub(crate) tag_effect:           Vec<String>,
    pub(crate) max_depth:            Option<usize>,
    pub(crate) quiet:                bool,
    pub(crate) wildcard_matches_sep: bool,
    pub(crate) pat_regex:            bool,
    pub(crate) fixed_string:         bool,
    pub(crate) prune:                bool,
    pub(crate) oregistry:            TagRegistry,

    pub(crate) registry:      Arc<Mutex<Registry>>,
    pub(crate) registry_path: PathBuf,

    pub(crate) border_color: cli_table::Color,

    #[cfg(feature = "encrypt-gpgme")]
    pub(crate) encrypt: EncryptConfig,
}

impl App {
    /// Run the application
    pub(crate) fn run(opts: Opts, config: &Config) -> Result<()> {
        let mut app = Self::new(&opts, config.clone())?;
        log::trace!("CONFIGURATION FILE: {:#?}", config);
        log::trace!("CONFIGURATION RESULT: {:#?}", app);
        app.run_command(opts, config)?;

        Ok(())
    }

    /// Create a new instance of the application
    pub(crate) fn new(opts: &Opts, config: Config) -> Result<Self> {
        let follow_links = opts
            .no_follow_links
            .then(|| false)
            .unwrap_or(opts.follow_links || config.follow_symlinks);

        let oregistry = oregistry::load_registry(opts, &config.encryption)?;

        let registry = Registry::new(opts.reg.as_ref().or(config.registry.as_ref()), follow_links)?;
        // Must be called each time to create user-defined functions
        registry.init()?;

        let file_types = opts.file_type.as_ref().map(|vals| {
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

        Ok(Self {
            base_color: config
                .base_color
                .map(parse_color)
                .transpose()?
                .unwrap_or(DEFAULT_BASE_COLOR),
            border_color: config
                .border_color
                .map(parse_color_cli_table)
                .transpose()?
                .unwrap_or(DEFAULT_BORDER_COLOR),
            base_dir: opts.dir.as_ref().map_or_else(
                || env::current_dir().context(fail!("determining CWD")),
                |bd| {
                    if bd.to_string_lossy() == "." {
                        env::current_dir().context(fail!("determining CWD"))
                    } else {
                        Ok(bd.clone())
                    }
                },
            )?,
            case_insensitive: opts.case_insensitive,
            case_sensitive: opts.case_sensitive,
            color_when: opts.color_when.unwrap_or_default(),
            // This ignores invalid colors (i.e., doesn't crash program)
            // Could also be done with .fold()
            colors: config.colors.map_or_else(
                || DEFAULT_COLORS.to_vec(),
                |colors_| {
                    colors_
                        .iter()
                        .filter_map(|color| {
                            let parsed = parse_color(color);
                            if let Err(e) = parsed {
                                wutag_error!("{e}");
                                None
                            } else {
                                parsed.ok()
                            }
                        })
                        .collect::<Vec<_>>()
                },
            ),
            exclude: opts.exclude.as_ref().map_or_else(Vec::new, |v| {
                v.iter().map(|ex| format!("!{}", ex.as_str())).collect()
            }),
            extension: opts
                .extension
                .as_ref()
                .map(|ext| {
                    RegexSetBuilder::new(
                        ext.iter()
                            .map(|e| e.trim_start_matches('.').to_owned())
                            .map(|e| format!(r".\.{}$", regex::escape(e.as_str()))),
                    )
                    .case_insensitive(true)
                    .build()
                })
                .transpose()?,
            file_type: file_types,
            follow_symlinks: follow_links,
            show_duplicates: config.show_duplicates,
            format: config.format.map_or_else(
                || "toml".to_owned(),
                |format_| {
                    {
                        if let f @ ("toml" | "yaml" | "yml" | "json") = format_.as_ref() {
                            f
                        } else {
                            wutag_error!(
                                "invalid format found in your configuration. Valid values: toml, \
                                 yaml, yml, json. Using the default: toml"
                            );
                            "toml"
                        }
                    }
                    .to_owned()
                },
            ),
            global: opts.global,
            ignores: config.ignores,
            ls_colors: opts.ls_colors,
            tag_effect: config
                .tag_effect
                .is_empty()
                .then(|| vec![String::from("bold")])
                .unwrap_or(config.tag_effect),
            max_depth: opts.max_depth.or(config.max_depth),
            wildcard_matches_sep: config.glob_wildcard_match_separator,
            pat_regex: opts.regex,
            fixed_string: opts.fixed_string,
            prune: opts.prune,
            quiet: opts.quiet,
            oregistry,

            registry_path: registry.path().clone(),
            registry: Arc::new(Mutex::new(registry)),

            #[cfg(feature = "encrypt-gpgme")]
            encrypt: config.encryption,
        })
    }

    /// Save the `TagRegistry` after modifications
    pub(crate) fn save_registry(&mut self) {
        if let Err(e) = self.oregistry.save() {
            wutag_error!("failed to save registry - {}", e);
        }
    }

    /// Run the subcommand from the command-line
    pub(crate) fn run_command(&mut self, opts: Opts, config: &Config) -> Result<()> {
        if self.color_when == Colorization::Never {
            colored::control::SHOULD_COLORIZE.set_override(false);
        } else if self.color_when == Colorization::Always {
            colored::control::SHOULD_COLORIZE.set_override(true);
        }

        debug_registry_path(&self.registry);

        match opts.cmd {
            Command::CleanCache => self.clean_cache(),
            Command::Clear(ref opts) => self.clear(opts)?,
            Command::Cp(ref opts) => self.cp(opts)?,
            Command::Edit(ref opts) => self.edit(opts),
            Command::Info(ref opts) => self.info(opts)?,
            Command::List(ref opts) => self.list(opts)?,
            Command::PrintCompletions(ref opts) => self.print_completions(opts),
            Command::Repair(ref opts) => self.repair(opts)?,
            Command::Rm(ref opts) => self.rm(opts)?,
            Command::Search(ref opts) => self.search(opts),
            Command::Set(opts) => self.set(&opts)?,
            Command::View(ref opts) => self.view(opts)?,

            #[cfg(feature = "ui")]
            Command::Ui => {
                better_panic::install();
                if let Err(e) = ui::start_ui(
                    &self.clone(),
                    config.clone(),
                    oregistry::load_registry(&opts, &config.encryption)
                        .expect("unable to get tag registry"),
                ) {
                    ui::destruct_terminal();
                    wutag_fatal!("{}", e);
                }
            },
        };

        #[cfg(feature = "encrypt-gpgme")]
        self.handle_encryption();

        Ok(())
    }

    /// Encryption command to run after every subcommand
    pub(crate) fn handle_encryption(&self) {
        if self.encrypt.to_encrypt && !oregistry::is_encrypted(&self.oregistry.path) {
            log::debug!("Attempting to encrypt registry");
            if let Err(e) = TagRegistry::crypt_registry(&self.oregistry.path, &self.encrypt, true) {
                wutag_fatal!("{}", e);
            }
        }
    }

    /// Format a tag according to the [`Tag`]'s color and an optional effect
    pub(crate) fn fmt_tag(&self, tag: &Tag) -> ColoredString {
        fmt::tag(tag, &self.tag_effect)
    }

    /// Format the colored/non-colored output of a [`Path`]
    pub(crate) fn fmt_path<P: AsRef<Path>>(&self, path: P) -> String {
        fmt::path(path, self)
    }

    /// Return a path in a format that only shows components from the CWD and
    /// any level of depth beneath that
    pub(crate) fn fmt_local_path<P: AsRef<Path>>(&self, path: P) -> String {
        fmt::local_path(path, self)
    }

    /// Return a local path with no color, i.e., one in which /home/user/... is
    /// not used and it is relative to the current directory. The searching
    /// of the paths does not go above the folder in which this command is
    /// read and only searches recursively
    pub(crate) fn fmt_raw_local_path<P: AsRef<Path>>(&self, path: P) -> String {
        fmt::raw_local_path(path, &self.base_dir)
    }

    /// Resolve the given entry if `follow_symlinks` is enabled
    ///
    /// Note that [`ignore::WalkParallel`] has an option to resolve symlinks,
    /// but it does not seem to resolve individual file symlinks. It only
    /// works with directories
    pub(crate) fn resolve_symlink<P: AsRef<Path>>(&self, entry: P) -> Result<PathBuf> {
        let entry = entry.as_ref();
        if self.follow_symlinks
            && fs::symlink_metadata(entry)
                .ok()
                .map_or(false, |f| f.file_type().is_symlink())
        {
            log::debug!("{}: resolving symlink", entry.display());
            return fs::canonicalize(entry)
                .context(format!("{}: failed to canonicalize", entry.display()));
        }

        Ok(entry.to_path_buf())
    }
}

// Implement a custom clone (specifically for `registry`)
impl Clone for App {
    fn clone(&self) -> Self {
        Self {
            base_color:           self.base_color,
            base_dir:             self.base_dir.clone(),
            case_insensitive:     self.case_insensitive,
            case_sensitive:       self.case_sensitive,
            color_when:           self.color_when,
            colors:               self.colors.clone(),
            exclude:              self.exclude.clone(),
            extension:            self.extension.clone(),
            file_type:            self.file_type,
            follow_symlinks:      self.follow_symlinks,
            show_duplicates:      self.show_duplicates,
            format:               self.format.clone(),
            global:               self.global,
            ignores:              self.ignores.clone(),
            ls_colors:            self.ls_colors,
            tag_effect:           self.tag_effect.clone(),
            max_depth:            self.max_depth,
            quiet:                self.quiet,
            wildcard_matches_sep: self.wildcard_matches_sep,
            pat_regex:            self.pat_regex,
            fixed_string:         self.fixed_string,
            prune:                self.prune,
            oregistry:            self.oregistry.clone(),
            registry:             self.registry.clone(),
            registry_path:        self.registry_path.clone(),

            border_color: self.border_color,

            #[cfg(feature = "encrypt-gpgme")]
            encrypt:                                   self.encrypt.clone(),
        }
    }
}

/// Debug the registry path
pub(crate) fn debug_registry_path(p: &Arc<Mutex<Registry>>) {
    log::debug!("{}", p.lock().expect("poisoned lock").path().display());
}

/// Parse a single tag-value pair
pub(crate) fn parse_tag_val<T>(s: &str) -> Result<(T, T), Box<dyn Error + Send + Sync + 'static>>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    use std::io::{Error as IoError, ErrorKind};
    let tagval = format!("expected {}", "tag=value".green());

    let pos = s.find('=').ok_or_else(|| {
        format!(
            "{}: no `{}` found in `{}`",
            tagval,
            "=".yellow(),
            s.magenta()
        )
    })?;

    let tag = &s[..pos];
    let value = &s[pos + 1..];

    let io_err = |s: String| -> Box<IoError> { Box::new(IoError::new(ErrorKind::InvalidInput, s)) };

    match (tag.is_empty(), value.is_empty()) {
        (true, true) => Err(io_err(format!("{}: tag or value cannot be empty", tagval))),
        (true, false) => Err(io_err(format!("{}: tag cannot be empty", tagval))),
        (false, true) => Err(io_err(format!("{}: value cannot be empty", tagval))),
        (false, false) => Ok((tag.parse()?, value.parse()?)),
    }
}
