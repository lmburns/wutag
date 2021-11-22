#![allow(unused)]
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    env,
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};
use tui::layout::Alignment;

use crate::ui::event::Key;
use wutag_core::color::TuiColor;

const CONFIG_FILE: &str = "wutag.yml";

// INFO: Could redo the structure of Config
// INFO: Create a global sub-struct, and put keys in UI

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "snake_case", default)]
pub(crate) struct Config {
    #[serde(alias = "max-depth")]
    pub(crate) max_depth:    Option<usize>,
    #[serde(alias = "base-color")]
    pub(crate) base_color:   Option<String>,
    #[serde(alias = "border-color")]
    pub(crate) border_color: Option<String>,
    pub(crate) colors:       Option<Vec<String>>,
    #[serde(alias = "ignore")]
    pub(crate) ignores:      Option<Vec<String>>,
    pub(crate) format:       Option<String>,

    #[cfg(feature = "ui")]
    #[serde(rename = "keys", alias = "Keys")]
    pub(crate) keys: KeyConfig,
    #[cfg(feature = "ui")]
    #[serde(rename = "tui", alias = "ui", alias = "UI", alias = "TUI")]
    pub(crate) ui:   UiConfig,

    #[cfg(feature = "encrypt-gpgme")]
    #[serde(rename = "encryption", alias = "Encryption")]
    pub(crate) encryption: EncryptConfig,
}

/// Encryption section of configuration file
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "snake_case", default)]
pub(crate) struct EncryptConfig {
    #[serde(alias = "public-key")]
    pub(crate) public_key: Option<String>,
    #[serde(alias = "to-encrypt")]
    pub(crate) to_encrypt: bool,
    #[serde(alias = "TTY")]
    pub(crate) tty:        bool,
}

/// UI general configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case", default)]
pub(crate) struct UiConfig {
    /// Whether the UI is colored
    #[serde(alias = "colored-ui")]
    pub(crate) colored_ui:    bool,
    /// TODO: ??
    pub(crate) looping:       bool,
    /// Command to run on startup to display files
    #[serde(alias = "startup-cmd", alias = "startup-command")]
    pub(crate) startup_cmd:   Option<String>,
    /// Refresh rate of application
    #[serde(alias = "tick-rate")]
    pub(crate) tick_rate:     u64,
    /// Whether some colors should flash
    #[serde(alias = "flash")]
    pub(crate) flashy:        bool,
    /// Map /home/user to $HOME
    #[serde(alias = "default-shorten")]
    pub(crate) default_alias: bool,
    /// Hash of these mappings /home/user to $HOME
    #[serde(alias = "shorten-hash")]
    pub(crate) alias_hash:    IndexMap<String, String>,

    // #[serde(skip)]
    // pub(crate) default_alias_hash: IndexMap<String, String>,
    /// Whether tags should be displayed as bold
    #[serde(alias = "tags-bold", alias = "bold-tags")]
    pub(crate) tags_bold:        bool,
    /// Whether paths should be displayed as bold
    #[serde(alias = "paths-bold", alias = "bold-paths")]
    pub(crate) paths_bold:       bool,
    /// Color the paths should be displayed
    #[serde(alias = "paths-color", alias = "color-paths")]
    pub(crate) paths_color:      String,
    /// TODO: Background color of completions
    #[serde(alias = "completion-color")]
    pub(crate) completion_color: String,

    /// What symbol should indicate item isn't selected
    #[serde(alias = "unmark-indicator")]
    pub(crate) unmark_indicator:    String,
    /// What symbol should indicate item is selected
    #[serde(alias = "selection-indicator")]
    pub(crate) selection_indicator: String,
    /// What symbol should indicate item is marked
    #[serde(alias = "mark-indicator")]
    pub(crate) mark_indicator:      String,

    /// Whether tags should change color when selected
    #[serde(alias = "selection-tags", alias = "tag-selections")]
    pub(crate) selection_tags:   bool,
    /// Whether selection style should blink
    #[serde(alias = "selection-blink")]
    pub(crate) selection_blink:  bool,
    /// Whether selection style should be bold
    #[serde(alias = "selection-bold")]
    pub(crate) selection_bold:   bool,
    /// Whether selection style should be dim
    #[serde(alias = "selection-dim")]
    pub(crate) selection_dim:    bool,
    /// Whether selection style should be italic
    #[serde(alias = "selection-italic")]
    pub(crate) selection_italic: bool,

    /// Alignment of header
    #[serde(alias = "header-alignment")]
    pub(crate) header_alignment: String,
}

/// UI Key configuration
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case", default)]
pub(crate) struct KeyConfig {
    pub(crate) quit:         Key,
    // Movement
    pub(crate) up:           Key,
    pub(crate) down:         Key,
    #[serde(alias = "go-to-top", alias = "goto-top")]
    pub(crate) go_to_top:    Key,
    #[serde(alias = "go-to-bottom", alias = "goto-bottom")]
    pub(crate) go_to_bottom: Key,
    #[serde(alias = "page-up")]
    pub(crate) page_up:      Key,
    #[serde(alias = "page-down")]
    pub(crate) page_down:    Key,
    #[serde(alias = "select-all")]
    pub(crate) select_all:   Key,
    pub(crate) select:       Key,
    pub(crate) refresh:      Key,
    pub(crate) help:         Key,

    // Actions to tags
    pub(crate) add:     Key,
    pub(crate) clear:   Key,
    pub(crate) remove:  Key,
    pub(crate) edit:    Key,
    pub(crate) search:  Key,
    pub(crate) copy:    Key,
    pub(crate) preview: Key,
    /* pub(crate) modify:       Key,
     * pub(crate) undo:         Key,
     * pub(crate) done:         Key,
     * pub(crate) refresh:      Key, */
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            quit:         Key::Char('q'),
            up:           Key::Char('k'),
            down:         Key::Char('j'),
            go_to_bottom: Key::Char('G'),
            go_to_top:    Key::Char('g'),
            page_up:      Key::Char('K'),
            page_down:    Key::Char('J'),
            select:       Key::Char('v'),
            select_all:   Key::Char('V'),
            refresh:      Key::Char('r'),
            help:         Key::Char('?'),
            add:          Key::Char('a'),
            clear:        Key::Char('D'),
            edit:         Key::Char('e'),
            remove:       Key::Char('x'),
            search:       Key::Char('/'),
            copy:         Key::Char('y'),
            preview:      Key::Char('P'),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            colored_ui:          true,
            looping:             true,
            flashy:              true,
            default_alias:       true,
            alias_hash:          IndexMap::new(),
            tick_rate:           250_u64,
            startup_cmd:         Some(String::from("--global list files --with-tags")),
            tags_bold:           true,
            paths_bold:          true,
            paths_color:         String::from("blue"),
            completion_color:    String::from("dark"),
            selection_tags:      false,
            selection_blink:     false,
            selection_bold:      false,
            selection_dim:       false,
            selection_italic:    true,
            mark_indicator:      String::from("\u{2714}"),
            unmark_indicator:    String::from(" "),
            selection_indicator: String::from("\u{2022}"),
            header_alignment:    String::from("center"),
        }
    }
}

impl Config {
    /// Loads Config from provided `path` by appending
    /// [CONFIG_FILE](CONFIG_FILE) name to it and reading the file.
    pub(crate) fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            fs::create_dir_all(path).context("unable to create config directory")?;
        }

        let path = path.join(CONFIG_FILE);

        if !path.is_file() {
            let initialization = include_str!("../example/wutag.yml");

            let mut config_file: fs::File = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&path)
                .with_context(|| format!("could not create wutag config: '{}'", path.display()))?;

            config_file
                .write_all(initialization.as_bytes())
                .with_context(|| format!("could not create wutag config: '{}'", path.display()))?;
            config_file.flush()?;
        }

        // TODO: Need specific line errors when deserializing configuration file
        // Until then, don't use the "deny_unknown_fields" for serde
        serde_yaml::from_slice(&fs::read(path).context("failed to read config file")?)
            .context("failed to deserialize config file")
    }

    /// Loads config file from configuration directory
    pub(crate) fn load_default_location() -> Result<Self> {
        Self::load(get_config_path()?)
    }
}

impl KeyConfig {
    // TODO: Remove if unnecessary
    /// Check for duplicate keys within configuration file
    pub(crate) fn check_dupes(&self) -> Result<()> {
        let opts = vec![
            &self.quit,
            &self.add,
            &self.edit,
            &self.go_to_bottom,
            &self.go_to_top,
            &self.down,
            &self.up,
            &self.page_down,
            &self.page_up,
            &self.remove,
            &self.select,
            &self.select_all,
            &self.search,
            &self.copy,
            &self.clear,
        ];
        let mut cloned = opts.clone();
        cloned.sort_unstable();
        cloned.dedup();

        if opts.len() == cloned.len() {
            Ok(())
        } else {
            Err(anyhow!(
                "{:#?}",
                crate::wutag_error!(
                    "configuration contains duplicate keys: {:#?}",
                    cloned
                        .into_iter()
                        .filter(|v| !opts.contains(v))
                        .collect::<Vec<_>>()
                )
            ))
        }
    }

    /// Return the field name as a string for generating keybindings help within
    /// the TUI
    pub(crate) fn fieldname(&self, other: Key) -> String {
        match other {
            s if s == self.quit => "quit",
            s if s == self.up => "up",
            s if s == self.down => "down",
            s if s == self.go_to_top => "go to top",
            s if s == self.go_to_bottom => "go to bottom",
            s if s == self.page_up => "page up",
            s if s == self.page_down => "page down",
            s if s == self.select_all => "select all",
            s if s == self.select => "select",
            s if s == self.refresh => "refresh",
            s if s == self.help => "help",
            s if s == self.add => "add",
            s if s == self.clear => "clear",
            s if s == self.remove => "remove",
            s if s == self.edit => "edit",
            s if s == self.search => "search",
            s if s == self.copy => "copy",
            s if s == self.preview => "preview",
            _ => unreachable!(),
        }
        .to_string()
    }
}

impl UiConfig {
    /// Create the default alias hash. `IndexMap` is needed to keep track of the
    /// order the user adds the variables. If one variable is `$XDG_CONFIG_HOME`
    /// which is `$HOME/.config`, and `$HOME` is also a variable, the longer and
    /// more specific variable should replace parts of the path first.
    ///
    /// This also adds the user's custom aliases from the configuration file
    pub(crate) fn build_alias_hash(&mut self) -> IndexMap<String, String> {
        if self.alias_hash.is_empty() && !self.default_alias {
            return IndexMap::new();
        }

        let mut alias_hash = IndexMap::new();

        for var in self.alias_hash.keys() {
            alias_hash.insert(
                PathBuf::from(
                    shellexpand::full(self.alias_hash.get(var).unwrap())
                        .unwrap_or_else(|_| {
                            Cow::from(
                                shellexpand::LookupError {
                                    var_name: "UNKNOWN_ENVIRONMENT_VARIABLE".into(),
                                    cause:    env::VarError::NotPresent,
                                }
                                .to_string(),
                            )
                        })
                        .to_string(),
                )
                .display()
                .to_string(),
                format!("%{}", var),
            );
        }

        // The unwrap INVALID_ is used here since these will get inserted into the hash
        // anyway, if for whatever reason a distribution does not have this directory,
        // therefore it should never get registered because the path will never
        // register. It would be better to have this than an error be thrown crashing
        // the program
        if self.default_alias {
            // Used to insert the default directory given by `dirs`
            let insert_default =
                |hash: &mut IndexMap<String, String>, dir: Option<PathBuf>, name: &str| {
                    hash.insert(
                        dir.unwrap_or_else(|| {
                            PathBuf::from(format!("INVALID_{}_DIR", name.replace("DIR", "")))
                        })
                        .display()
                        .to_string(),
                        format!("%{}", name),
                    )
                };

            // Used for alternative folders for `macOS`. Use XDG specs instead
            let alt_dirs = |path: Option<PathBuf>, join: &str, var: &str| -> String {
                // Test whether the XDG variable is set. If not join with the `join`
                #[cfg(target_os = "macos")]
                let dir_og = std::env::var_os(format!("XDG_{}", var))
                    .map(PathBuf::from)
                    .filter(|p| p.is_absolute())
                    .or_else(|| dirs::home_dir().map(|d| d.join(join)))
                    .context(format!("Invalid {} directory", var));

                #[cfg(not(target_os = "macos"))]
                let dir_og = path;

                dir_og
                    .unwrap_or_else(|| {
                        PathBuf::from(format!("INVALID_{}_DIR", join.to_uppercase()))
                    })
                    .display()
                    .to_string()
            };

            let insert_alt = |hash: &mut IndexMap<String, String>,
                              dir: Option<PathBuf>,
                              join: &str,
                              name: &str| {
                hash.insert(alt_dirs(dir, join, name), format!("%{}", name));
            };

            // For example:
            //      - linux: XDG_MUSIC_DIR - /home/alice/Music
            //      - macos: $HOME/Music   - /Users/alice/Music
            // They're in the same spot so `insert_default` is used
            insert_default(&mut alias_hash, dirs::audio_dir(), "MUSIC_DIR");

            // For example:
            //      - linux: XDG_CACHE_DIR          - /home/alice/.cache
            //      - macos: $HOME/Library/Caches   - /Users/alice/Library/Caches
            // They're not in the same spot, so join `$HOME` with `.cache` on `macOS`
            insert_alt(&mut alias_hash, dirs::cache_dir(), ".cache", "CACHE_HOME");

            insert_alt(
                &mut alias_hash,
                dirs::config_dir(),
                ".config",
                "CONFIG_HOME",
            );

            insert_alt(
                &mut alias_hash,
                dirs::data_dir(),
                ".local/share",
                "DATA_HOME",
            );

            insert_default(&mut alias_hash, dirs::desktop_dir(), "DESKTOP");
            insert_default(&mut alias_hash, dirs::document_dir(), "DOCUMENTS");
            insert_default(&mut alias_hash, dirs::download_dir(), "DOWNLOADS");

            // Not set on `macOS` at all
            insert_alt(
                &mut alias_hash,
                dirs::executable_dir(),
                ".local/bin",
                "BIN_HOME",
            );

            insert_default(&mut alias_hash, dirs::font_dir(), "FONTS_DIR");
            insert_default(&mut alias_hash, dirs::picture_dir(), "PICTURES");
            insert_default(&mut alias_hash, dirs::public_dir(), "PUBLIC_DIR");

            // Not set on `macOS` at all
            insert_alt(
                &mut alias_hash,
                dirs::template_dir(),
                "Templates",
                "TEMPLATE_DIR",
            );

            insert_default(&mut alias_hash, dirs::video_dir(), "VIDEO_DIR");

            // Lastly, do `$HOME` so all others will be replaced first
            insert_default(&mut alias_hash, dirs::home_dir(), "HOME");

            // Closure needs to be altered to fit something not in $HOME
            // directory insert_alt(
            //     &mut alias_hash,
            //     dirs::runtime_dir(),
            //     env::var_os("TMPDIR")
            //         .unwrap_or_else(|| OsString::from("/tmp"))
            //         .to_str()
            //         .unwrap()
            //         .to_string(),
            //     "RUNTIME_DIR",
            // );
        }

        alias_hash

        // self.default_alias_hash = alias_hash;
    }
}

/// Wrapper around [`Alignment`](tui::layout::Alignment) to provide
/// serialization
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum HeaderAlignment {
    Left,
    Center,
    Right,
}

impl From<HeaderAlignment> for Alignment {
    fn from(other: HeaderAlignment) -> Alignment {
        match other {
            HeaderAlignment::Left => Alignment::Left,
            HeaderAlignment::Center => Alignment::Center,
            HeaderAlignment::Right => Alignment::Right,
        }
    }
}

impl FromStr for HeaderAlignment {
    type Err = ();

    fn from_str(s: &str) -> Result<HeaderAlignment, Self::Err> {
        match s.to_ascii_lowercase().trim() {
            "left" => Ok(HeaderAlignment::Left),
            "right" => Ok(HeaderAlignment::Right),
            _ => Ok(HeaderAlignment::Center),
        }
    }
}

/// Get configuration file path
pub(crate) fn get_config_path() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    let conf_dir_og = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| dirs::home_dir().map(|d| d.join(".config")))
        .context("Invalid configuration directory");

    #[cfg(not(target_os = "macos"))]
    let conf_dir_og = dirs::config_dir();

    conf_dir_og
        .map(|p| p.join("wutag"))
        .context("unable to join config path")
}
