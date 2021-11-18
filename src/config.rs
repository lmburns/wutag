#![allow(unused)]
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use crate::ui::event::Key;
use wutag_core::color::TuiColor;

const CONFIG_FILE: &str = "wutag.yml";

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
    #[serde(alias = "colored-ui")]
    pub(crate) colored_ui:          bool,
    #[serde(alias = "completion-color")]
    pub(crate) completion_color:    String,
    pub(crate) looping:             bool,
    #[serde(alias = "mark-indicator")]
    pub(crate) mark_indicator:      String,
    #[serde(alias = "tags-bold", alias = "bold-tags")]
    pub(crate) tags_bold:           bool,
    #[serde(alias = "paths-bold", alias = "bold-paths")]
    pub(crate) paths_bold:          bool,
    #[serde(alias = "paths-color", alias = "color-paths")]
    pub(crate) paths_color:         String,
    #[serde(alias = "selection-blink")]
    pub(crate) selection_blink:     bool,
    #[serde(alias = "selection-bold")]
    pub(crate) selection_bold:      bool,
    #[serde(alias = "selection-dim")]
    pub(crate) selection_dim:       bool,
    #[serde(alias = "selection-indicator")]
    pub(crate) selection_indicator: String,
    #[serde(alias = "selection-italic")]
    pub(crate) selection_italic:    bool,
    #[serde(alias = "startup-cmd", alias = "startup-command")]
    pub(crate) startup_cmd:         Option<String>,
    #[serde(alias = "tick-rate")]
    pub(crate) tick_rate:           u64,
    #[serde(alias = "unmark-indicator")]
    pub(crate) unmark_indicator:    String,
}

/// UI Key configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
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
    pub(crate) select:       Key,
    #[serde(alias = "select-all")]
    pub(crate) select_all:   Key,
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
            add:          Key::Char('a'),
            edit:         Key::Char('e'),
            go_to_bottom: Key::Char('G'),
            go_to_top:    Key::Char('g'),
            down:         Key::Char('j'),
            up:           Key::Char('k'),
            page_down:    Key::Char('J'),
            page_up:      Key::Char('K'),
            remove:       Key::Char('x'),
            select:       Key::Char('v'),
            select_all:   Key::Char('V'),
            refresh:      Key::Char('r'),
            search:       Key::Char('/'),
            copy:         Key::Char('y'),
            clear:        Key::Char('D'),
            preview:      Key::Char('P'),
            help:         Key::Char('?'),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            colored_ui:          true,
            completion_color:    String::from("dark"),
            looping:             true,
            mark_indicator:      String::from("\u{2714}"),
            tags_bold:           true,
            paths_bold:          true,
            paths_color:         String::from("blue"),
            selection_blink:     false,
            selection_bold:      true,
            selection_dim:       false,
            selection_italic:    false,
            selection_indicator: String::from("\u{2022}"),
            startup_cmd:         Some(String::from("--global list files --with-tags")),
            tick_rate:           250_u64,
            unmark_indicator:    String::from(" "),
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
