//! Subcommands found within the `wutag` crate

pub(crate) mod clean_cache;
pub(crate) mod clear;
pub(crate) mod cp;
pub(crate) mod edit;
pub(crate) mod info;
pub(crate) mod list;
pub(crate) mod list2;
pub(crate) mod print_completions;
pub(crate) mod repair;
pub(crate) mod rm;
pub(crate) mod search;
pub(crate) mod set;
// pub(crate) mod set2;
pub(crate) mod testing;
pub(crate) mod uses;
pub(crate) mod view;

// TODO: Virtual filesystem
// TODO: Repair database
// TODO: Finish info command
// TODO: imply tags
// TODO: rename tags
// TODO: merge tags
// TODO: tag value attributes

use uses::{
    env, oregistry as registry, parse_color, parse_color_cli_table, ui, wutag_error, wutag_fatal,
    Color, Colorize, Command, Config, Context, EncryptConfig, FileTypes, Opts, PathBuf, RegexSet,
    RegexSetBuilder, Result, Stream, TagRegistry, DEFAULT_BASE_COLOR, DEFAULT_BORDER_COLOR,
    DEFAULT_COLORS,
};

/// A structure that is built from a parsed `Config` and parsed `Opts`
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Clone, Debug)]
pub(crate) struct App {
    pub(crate) base_color:       Color,
    pub(crate) base_dir:         PathBuf,
    pub(crate) case_insensitive: bool,
    pub(crate) case_sensitive:   bool,
    pub(crate) color_when:       String,
    pub(crate) colors:           Vec<Color>,
    pub(crate) exclude:          Vec<String>,
    pub(crate) extension:        Option<RegexSet>,
    pub(crate) file_type:        Option<FileTypes>,
    pub(crate) follow_symlinks:  bool,
    pub(crate) format:           String,
    pub(crate) global:           bool,
    pub(crate) ignores:          Option<Vec<String>>,
    pub(crate) ls_colors:        bool,
    pub(crate) max_depth:        Option<usize>,
    pub(crate) quiet:            bool,
    pub(crate) pat_regex:        bool,
    pub(crate) registry:         TagRegistry,

    #[cfg(feature = "prettify")]
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
        let base_dir = if let Some(base_dir) = &opts.dir {
            if base_dir.display().to_string() == "." {
                std::env::current_dir().context("failed to determine CWD")?
            } else {
                base_dir.clone()
            }
        } else {
            std::env::current_dir().context("failed to determine CWD")?
        };

        let colors = if let Some(colors_) = config.colors {
            let mut colors = Vec::new();
            for color in colors_.iter().map(parse_color) {
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

        #[cfg(feature = "prettify")]
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

        let format = config.format.map_or_else(
            || "toml".to_owned(),
            |format_| {
                {
                    if let f @ ("toml" | "yaml" | "yml" | "json") = format_.as_ref() {
                        f
                    } else {
                        wutag_error!(
                            "invalid format found as your configuration. Valid values: toml, \
                             yaml, yml, json. Using the default: toml"
                        );
                        "toml"
                    }
                }
                .to_owned()
            },
        );

        let registry = registry::load_registry(opts, &config.encryption)?;

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

        let excludes = opts.exclude.clone().map_or_else(Vec::new, |v| {
            v.iter().map(|p| format!("!{}", p.as_str())).collect()
        });

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

        Ok(Self {
            base_color,
            base_dir,
            case_insensitive: opts.case_insensitive,
            case_sensitive: opts.case_sensitive,
            color_when: color_when.to_owned(),
            colors,
            exclude: excludes,
            extension: extensions,
            file_type: file_types,
            follow_symlinks: config.follow_symlinks,
            format,
            global: opts.global,
            ignores: config.ignores,
            ls_colors: opts.ls_colors,
            max_depth: if opts.max_depth.is_some() {
                opts.max_depth
            } else {
                config.max_depth
            },
            pat_regex: opts.regex,
            quiet: opts.quiet,
            registry,

            #[cfg(feature = "prettify")]
            border_color,

            #[cfg(any(feature = "encrypt-gpgme"))]
            encrypt: config.encryption,
        })
    }

    /// Save the `TagRegistry` after modifications
    pub(crate) fn save_registry(&mut self) {
        if let Err(e) = self.registry.save() {
            wutag_error!("failed to save registry - {}", e);
        }
    }

    /// Run the subcommand from the command-line
    pub(crate) fn run_command(&mut self, opts: Opts, config: &Config) -> Result<()> {
        if self.color_when == "never" {
            colored::control::SHOULD_COLORIZE.set_override(false);
        } else if self.color_when == "always" {
            colored::control::SHOULD_COLORIZE.set_override(true);
        }

        match opts.cmd {
            Command::CleanCache => self.clean_cache(),
            Command::Clear(ref opts) => self.clear(opts),
            Command::Cp(ref opts) => self.cp(opts)?,
            Command::Edit(ref opts) => self.edit(opts),
            Command::Info(ref opts) => self.info(opts),
            Command::List(ref opts) => self.list(opts),
            Command::List2(ref opts) => self.list2(opts),
            Command::PrintCompletions(ref opts) => self.print_completions(opts),
            Command::Repair(ref opts) => self.repair(opts)?,
            Command::Rm(ref opts) => self.rm(opts),
            Command::Search(ref opts) => self.search(opts),
            Command::Set(opts) => self.set(&opts)?,
            Command::Testing(opts) => self.testing(&opts)?,
            Command::View(ref opts) => self.view(opts)?,

            #[cfg(feature = "ui")]
            Command::Ui => {
                better_panic::install();
                if let Err(e) = ui::start_ui(
                    &self.clone(),
                    config.clone(),
                    registry::load_registry(&opts, &config.encryption)
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
        if self.encrypt.to_encrypt && !registry::is_encrypted(&self.registry.path) {
            log::debug!("Attempting to encrypt registry");
            if let Err(e) = TagRegistry::crypt_registry(&self.registry.path, &self.encrypt, true) {
                wutag_fatal!("{}", e);
            }
        }
    }
}
