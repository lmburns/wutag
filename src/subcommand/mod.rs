pub(crate) mod clean_cache;
pub(crate) mod clear;
pub(crate) mod cp;
pub(crate) mod edit;
pub(crate) mod list;
pub(crate) mod print_completions;
pub(crate) mod rm;
pub(crate) mod search;
pub(crate) mod set;
pub(crate) mod uses;
pub(crate) mod view;

use crate::{
    opt::{Command, Opts},
    ui::ui_app::UiApp,
};
use uses::{
    env, fs, parse_color, parse_color_cli_table, wutag_error, Color, Colorize, Config, Context,
    Cow, FileTypes, LookupError, PathBuf, RegexSet, RegexSetBuilder, Result, Stream, TagRegistry,
    DEFAULT_BASE_COLOR, DEFAULT_BORDER_COLOR, DEFAULT_COLORS,
};

// TODO: Add --all option to view
// TODO: Add list options for search

#[derive(Clone, Debug)]
pub(crate) struct App {
    pub(crate) base_color:       Color,
    pub(crate) base_dir:         PathBuf,
    pub(crate) border_color:     cli_table::Color,
    pub(crate) case_insensitive: bool,
    pub(crate) case_sensitive:   bool,
    pub(crate) color_when:       String,
    pub(crate) colors:           Vec<Color>,
    pub(crate) exclude:          Vec<String>,
    pub(crate) extension:        Option<RegexSet>,
    pub(crate) file_type:        Option<FileTypes>,
    pub(crate) format:           String,
    pub(crate) global:           bool,
    pub(crate) ignores:          Option<Vec<String>>,
    pub(crate) ls_colors:        bool,
    pub(crate) max_depth:        Option<usize>,
    pub(crate) pat_regex:        bool,
    pub(crate) registry:         TagRegistry,
    pub(crate) ui:               Option<UiApp>,
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
                base_dir.clone()
            }
        } else {
            std::env::current_dir().context("failed to determine current working directory")?
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

        let format = if let Some(format_) = config.format {
            match format_.as_ref() {
                f @ ("toml" | "yaml" | "yml" | "json") => f.to_string(),
                _ => {
                    wutag_error!(
                        "invalid format found as your configuration. Valid values: toml, yaml, \
                         yml, json. Using the default: toml"
                    );
                    "toml".to_string()
                },
            }
        } else {
            "toml".to_string()
        };

        let state_file = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| dirs::home_dir().map(|d| d.join(".cache")))
            .map(|p| p.join("wutag.registry"))
            .unwrap();

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
            } else if registry.display().to_string().ends_with('/') {
                wutag_error!(
                    "{} last error is a directory path. Using default registry: {}",
                    registry.display().to_string().green(),
                    state_file.display().to_string().green(),
                );
                TagRegistry::load(&state_file).unwrap_or_else(|_| TagRegistry::new(&state_file))
            } else {
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

        let excludes = opts.exclude.clone().map_or_else(Vec::new, |v| {
            v.iter().map(|p| String::from("!") + p.as_str()).collect()
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

        let ui_app = if opts.ui {
            Some(UiApp::new(config, registry)?)
        } else {
            None
        };

        Ok(App {
            base_color,
            base_dir,
            border_color,
            case_insensitive: opts.case_insensitive,
            case_sensitive: opts.case_sensitive,
            color_when: color_when.to_string(),
            colors,
            exclude: excludes,
            extension: extensions,
            file_type: file_types,
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
            registry,
            ui: ui_app,
        })
    }

    pub(crate) fn save_registry(&mut self) {
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
            Command::CleanCache => self.clean_cache(),
            Command::Clear(ref opts) => self.clear(opts),
            Command::Cp(ref opts) => self.cp(opts),
            Command::Edit(ref opts) => self.edit(opts),
            Command::List(ref opts) => self.list(opts),
            Command::PrintCompletions(ref opts) => self.print_completions(opts),
            Command::Rm(ref opts) => self.rm(opts),
            Command::Search(ref opts) => self.search(opts),
            Command::Set(opts) => self.set(&opts),
            Command::View(ref opts) => self.view(opts),
        }
    }
}
