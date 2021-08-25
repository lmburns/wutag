pub mod clean_cache;
pub mod clear;
pub mod cp;
pub mod edit;
pub mod list;
pub mod print_completions;
pub mod rm;
pub mod search;
pub mod set;
pub mod uses;
pub mod view;

use crate::opt::{Command, Opts};
use uses::*;

// TODO: Add --all option to view
// TODO: Add list options for search

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
            Command::List(ref opts) => self.list(opts),
            Command::Set(opts) => self.set(&opts),
            Command::Rm(ref opts) => self.rm(opts),
            Command::Clear(ref opts) => self.clear(opts),
            Command::Search(ref opts) => self.search(opts),
            Command::Cp(ref opts) => self.cp(opts),
            Command::View(ref opts) => self.view(opts),
            Command::Edit(ref opts) => self.edit(opts),
            Command::PrintCompletions(ref opts) => self.print_completions(opts),
            Command::CleanCache => self.clean_cache(),
        }
    }
}
