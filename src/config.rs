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
    pub(crate) tick_rate:           u64,
    pub(crate) looping:             bool,
    pub(crate) selection_indicator: String,
    pub(crate) mark_indicator:      String,
    pub(crate) unmark_indicator:    String,
    pub(crate) selection_bold:      bool,
    pub(crate) selection_italic:    bool,
    pub(crate) selection_dim:       bool,
    pub(crate) selection_blink:     bool,
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
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            tick_rate:           250_u64,
            looping:             true,
            selection_indicator: String::from("\u{2022}"),
            mark_indicator:      String::from("\u{2714}"),
            unmark_indicator:    String::from(" "),
            selection_bold:      true,
            selection_italic:    false,
            selection_dim:       false,
            selection_blink:     false,
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
        Self::load(
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .or_else(|| dirs::home_dir().map(|d| d.join(".config")))
                .context("Invalid configuration directory")?
                .join("wutag"),
        )
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
