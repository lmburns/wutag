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
pub(crate) struct Config {
    pub(crate) max_depth:    Option<usize>,
    pub(crate) base_color:   Option<String>,
    pub(crate) border_color: Option<String>,
    pub(crate) colors:       Option<Vec<String>>,
    pub(crate) ignores:      Option<Vec<String>>,
    pub(crate) format:       Option<String>,
    #[serde(rename = "keys")]
    pub(crate) keys:         KeyConfig,
    #[serde(rename = "ui")]
    pub(crate) ui:           UiConfig,
}

/// UI general configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct UiConfig {
    pub(crate) colored_ui:          bool,
    pub(crate) completion_color:    String,
    pub(crate) looping:             bool,
    pub(crate) mark_indicator:      String,
    pub(crate) paths_bold:          bool,
    pub(crate) paths_color:         String,
    pub(crate) selection_blink:     bool,
    pub(crate) selection_bold:      bool,
    pub(crate) selection_dim:       bool,
    pub(crate) selection_indicator: String,
    pub(crate) selection_italic:    bool,
    pub(crate) startup_cmd:         Option<String>,
    pub(crate) tick_rate:           u64,
    pub(crate) unmark_indicator:    String,
}

/// UI Key configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct KeyConfig {
    pub(crate) quit:         Key,
    // Movement
    pub(crate) up:           Key,
    pub(crate) down:         Key,
    pub(crate) go_to_top:    Key,
    pub(crate) go_to_bottom: Key,
    pub(crate) page_up:      Key,
    pub(crate) page_down:    Key,
    pub(crate) select:       Key,
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
            paths_bold:          true,
            paths_color:         String::from("blue"),
            selection_blink:     false,
            selection_bold:      true,
            selection_dim:       false,
            selection_indicator: String::from("\u{2022}"),
            selection_italic:    false,
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
            let initialization = "---\nmax_depth: 2\n...";

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

        serde_yaml::from_slice(&fs::read(path).context("failed to read config file")?)
            .context("failed to deserialize config file")
    }

    /// Loads config file from home directory of user executing the program
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
