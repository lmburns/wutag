//! Configuration file options for the user

#![allow(clippy::use_self)]
#![allow(unused)]

// TODO: Check for duplicate keys in configuration file

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

use crate::{
    directories::PROJECT_DIRS, fail, failt, inner_immute, ui::event::Key, utils::color::TuiColor,
    wutag_fatal,
};

/// Configuration file name
const CONFIG_FILE: &str = "wutag.yml";

/// Configuration file options
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case", default)]
pub(crate) struct Config {
    /// Location of where tags are stored
    #[serde(alias = "database")]
    pub(crate) registry: Option<PathBuf>,

    /// Follow a symlink to real file
    #[serde(alias = "follow-symlinks")]
    pub(crate) follow_symlinks: bool,

    /// Max depth a regex/glob with traverse
    #[serde(alias = "max-depth")]
    pub(crate) max_depth: Option<usize>,

    /// Font effect of tags:
    ///   - bold, b
    ///   - underline, ul
    ///   - italic, it
    ///   - reverse, r
    ///   - dimmed, d
    ///   - blink, bl
    ///   - strikethrough, st
    ///   - background, bg
    ///   - none, n
    #[serde(alias = "tag-effect")]
    pub(crate) tag_effect: Vec<String>,

    /// Base color that paths are displayed
    #[serde(alias = "base-color")]
    pub(crate) base_color: Option<String>,

    /// Border color used to display tags with border option
    #[serde(alias = "border-color")]
    pub(crate) border_color: Option<String>,

    /// Array of colors to use as tags
    pub(crate) colors: Option<Vec<String>>,

    #[serde(alias = "ignore")]
    /// Array of file patterns to ignore tagging
    pub(crate) ignores: Option<Vec<String>>,

    /// The format the file is in when using `view` subcommand
    pub(crate) format: Option<String>,

    /// Should duplicate file hashes be reported?
    #[serde(alias = "show-duplicates")]
    pub(crate) show_duplicates: bool,

    /// If true, `*` in a glob will match any number of directories
    /// The default behavior is for `**/<glob>` to match path separators
    #[serde(alias = "glob-wildcard-match-separator")]
    pub(crate) glob_wildcard_match_separator: bool,

    // pub(crate) root_path: PathBuf,
    /// Configuration dealing with keys
    #[cfg(feature = "ui")]
    #[serde(rename = "keys", alias = "Keys")]
    pub(crate) keys: KeyConfig,

    /// Configuration dealing with UI settings
    #[cfg(feature = "ui")]
    #[serde(rename = "tui", alias = "ui", alias = "UI", alias = "TUI")]
    pub(crate) ui: UiConfig,

    /// Configuration dealing with encryption
    #[cfg(feature = "encrypt-gpgme")]
    #[serde(rename = "encryption", alias = "Encryption")]
    pub(crate) encryption: EncryptConfig,
}

/// Encryption section of configuration file
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "snake_case", default)]
pub(crate) struct EncryptConfig {
    /// Public key/email to use `gpg` with
    #[serde(alias = "public-key")]
    pub(crate) public_key: Option<String>,
    /// Whether the database/yaml file should actually be encrypted
    #[serde(alias = "to-encrypt")]
    pub(crate) to_encrypt: bool,
    // TODO: Check and make sure works
    /// Use a `TTY` to ask for password input
    #[serde(alias = "TTY")]
    pub(crate) tty:        bool,
}

/// UI general configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case", default)]
pub(crate) struct UiConfig {
    /// Whether the UI is colored
    #[serde(alias = "colored-ui")]
    pub(crate) colored_ui:           bool,
    /// Whether the list should wrap back around to opposite side when reaching
    /// end
    pub(crate) looping:              bool,
    /// Refresh rate of application
    #[serde(alias = "tick-rate")]
    pub(crate) tick_rate:            u64,
    /// Command to run on startup to display files
    #[serde(alias = "startup-cmd", alias = "startup-command")]
    pub(crate) startup_cmd:          Option<String>,
    /// Number of lines preview_scroll_{up,down} should move
    #[serde(alias = "preview-scroll-lines")]
    pub(crate) preview_scroll_lines: u16,
    /// Height of preview window (out of 100)
    #[serde(alias = "preview-height")]
    pub(crate) preview_height:       u16,
    /// Whether history should be enabled
    pub(crate) history:              bool,
    #[serde(alias = "history-filepath")]
    /// Path to history file
    pub(crate) history_filepath:     String,
    /// Whether some colors should flash
    #[serde(alias = "flash")]
    pub(crate) flashy:               bool,
    /// Map /home/user to $HOME
    #[serde(alias = "default-shorten")]
    pub(crate) default_alias:        bool,
    /// Hash of these mappings /home/user to $HOME
    #[serde(alias = "shorten-hash")]
    pub(crate) alias_hash:           IndexMap<String, String>,

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
    /// Underline header
    #[serde(alias = "header-underline")]
    pub(crate) header_underline: bool,
}

/// UI Key configuration
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case", default)]
pub(crate) struct KeyConfig {
    // == General ==
    /// Quit the application
    pub(crate) quit:    Key,
    /// Get overall help
    pub(crate) help:    Key,
    /// Refresh application
    pub(crate) refresh: Key,
    /// Preview a file
    pub(crate) preview: Key,

    // == Movement ==
    /// Move up
    pub(crate) up:           Key,
    /// Move down
    pub(crate) down:         Key,
    /// Go to the top
    #[serde(alias = "go-to-top", alias = "goto-top")]
    pub(crate) go_to_top:    Key,
    /// Go to the bottom
    #[serde(alias = "go-to-bottom", alias = "goto-bottom")]
    pub(crate) go_to_bottom: Key,
    /// Scroll a page up
    #[serde(alias = "page-up")]
    pub(crate) page_up:      Key,
    /// Scroll a page down
    #[serde(alias = "page-down")]
    pub(crate) page_down:    Key,
    /// Select all items in the list
    #[serde(alias = "select-all")]
    pub(crate) select_all:   Key,
    /// Select one item in the list
    pub(crate) select:       Key,
    #[serde(alias = "preview-down")]
    /// Move the preview window down
    pub(crate) preview_down: Key,
    #[serde(alias = "preview-down")]
    /// Move the preview window up
    pub(crate) preview_up:   Key,

    // == Actions to tags ==
    /// Add a tag to the database
    pub(crate) add:    Key,
    /// Set tag(s) on a file
    pub(crate) set:    Key,
    /// Clear all tags from a file
    pub(crate) clear:  Key,
    /// Remove tag(s) from a file
    pub(crate) remove: Key,
    /// Edit tag attributes
    pub(crate) edit:   Key,
    /// View the tag in an `$EDITOR`
    pub(crate) view:   Key,
    /// Search for tags
    pub(crate) search: Key,
    /// Copy attributes of one tag to another
    pub(crate) copy:   Key,
    // pub(crate) modify:  Key,
    // pub(crate) undo:    Key,
    // pub(crate) done:    Key,
}

impl Config {
    inner_immute!(follow_symlinks, bool, false);

    inner_immute!(max_depth, Option<usize>, false);

    inner_immute!(base_color, Option<String>);

    inner_immute!(colors, Option<Vec<String>>);

    inner_immute!(ignores, Option<Vec<String>>);

    inner_immute!(format, Option<String>);

    inner_immute!(border_color, Option<String>);

    #[cfg(feature = "ui")]
    inner_immute!(keys, KeyConfig);

    #[cfg(feature = "ui")]
    inner_immute!(ui, UiConfig);

    #[cfg(feature = "encrypt-gpgme")]
    inner_immute!(encryption, EncryptConfig);
}

impl Default for Config {
    fn default() -> Self {
        Self {
            follow_symlinks: true,
            show_duplicates: true,
            glob_wildcard_match_separator: false,
            format: None,
            ignores: None,
            colors: None,
            registry: None,
            max_depth: None,
            tag_effect: vec![String::from("bold")],
            base_color: None,
            border_color: None,

            #[cfg(feature = "ui")]
            keys:                                         KeyConfig::default(),
            #[cfg(feature = "ui")]
            ui:                                           UiConfig::default(),
            #[cfg(feature = "encrypt-gpgme")]
            encryption:                                   EncryptConfig::default(),
        }
    }
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            quit:    Key::Char('q'),
            help:    Key::Char('?'),
            refresh: Key::Char('r'),
            preview: Key::Char('P'),

            up:           Key::Char('k'),
            down:         Key::Char('j'),
            go_to_top:    Key::Char('g'),
            go_to_bottom: Key::Char('G'),
            page_up:      Key::Char('K'),
            page_down:    Key::Char('J'),
            preview_up:   Key::Ctrl('u'),
            preview_down: Key::Ctrl('d'),
            select:       Key::Char('v'),
            select_all:   Key::Char('V'),

            add:    Key::Char('a'),
            set:    Key::Char('s'),
            clear:  Key::Char('D'),
            remove: Key::Char('x'),
            edit:   Key::Char('e'),
            view:   Key::Char('o'),
            search: Key::Char('/'),
            copy:   Key::Char('y'),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            colored_ui:           true,
            looping:              true,
            flashy:               true,
            history:              true,
            history_filepath:     PROJECT_DIRS
                .config_dir()
                .join("command.history")
                .to_string_lossy()
                .to_string(),
            preview_scroll_lines: 1_u16,
            preview_height:       60_u16,
            default_alias:        true,
            alias_hash:           IndexMap::new(),
            tick_rate:            250_u64,
            startup_cmd:          Some(String::from("--global list files --with-tags")),
            tags_bold:            true,
            paths_bold:           true,
            paths_color:          String::from("blue"),
            completion_color:     String::from("dark"),
            selection_tags:       false,
            selection_blink:      false,
            selection_bold:       false,
            selection_dim:        false,
            selection_italic:     true,
            mark_indicator:       String::from("\u{2714}"),
            unmark_indicator:     String::from(" "),
            selection_indicator:  String::from("\u{2022}"),
            header_alignment:     String::from("center"),
            header_underline:     true,
        }
    }
}

// TODO: allow specifying configuration file

impl Config {
    /// Loads Config from provided `path` by appending
    /// [`CONFIG_FILE`]'s name to it and serializing the file into `Config`
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

        let file = fs::read(&path).context(fail!("reading config file"))?;
        // let attempt = serde_yaml::Deserializer::from_slice(&file);
        //
        // let res: Result<Self, _> = serde_path_to_error::deserialize(attempt);
        //
        // if let Err(e) = res {
        //     return Err(anyhow!(
        //         "there was an error parsing configuration file: {}",
        //         e.path().to_string()
        //     ));
        // }

        let attempt: Self = serde_yaml::from_slice(&file).context(fail!("deserializing config file"))?;

        if attempt.ui.preview_height > 100 {
            wutag_fatal!(
                "preview height ({}) cannot be above 100",
                attempt.ui.preview_height
            );
        }

        Ok(attempt)
    }

    /// Loads configuration file from the default location
    pub(crate) fn load_default_location() -> Result<Self> {
        Self::load(PROJECT_DIRS.config_dir())
    }
}

impl KeyConfig {
    // TODO: Use with ui::command
    /// Return the field name as a string for the generation of keybindings
    /// within the help menu in the TUI
    pub(crate) fn fieldname(&self, other: Key) -> String {
        match other {
            s if s == self.quit => "quit",
            s if s == self.help => "help",
            s if s == self.refresh => "refresh",
            s if s == self.preview => "preview",
            //
            s if s == self.up => "up",
            s if s == self.down => "down",
            s if s == self.go_to_top => "go to top",
            s if s == self.go_to_bottom => "go to bottom",
            s if s == self.page_up => "page up",
            s if s == self.page_down => "page down",
            s if s == self.preview_up => "preview up",
            s if s == self.preview_down => "preview down",
            s if s == self.select_all => "select all",
            s if s == self.select => "select",
            //
            s if s == self.add => "add",
            s if s == self.set => "set",
            s if s == self.clear => "clear",
            s if s == self.remove => "remove",
            s if s == self.edit => "edit",
            s if s == self.view => "view",
            s if s == self.search => "search",
            s if s == self.copy => "copy",
            _ => unreachable!(),
        }
        .to_owned()
    }
}

impl UiConfig {
    /// Create the default alias hash. `IndexMap` is needed to keep track of the
    /// order the user adds the variables. If one variable is `$XDG_CONFIG_HOME`
    /// which is `$HOME/.config`, and `$HOME` is also a variable, the longer and
    /// more specific variable should replace parts of the path first.
    ///
    /// This also adds the user's custom aliases from the configuration file
    ///
    /// See [`alias_replace`] for more
    /// information on what this does and why I'm doing it
    ///
    /// [`alias_replace`]: ./ui/ui_app/struct.UiApp#method.alias_replace
    pub(crate) fn build_alias_hash(&mut self) -> IndexMap<String, String> {
        let mut alias_hash = IndexMap::new();

        if self.alias_hash.is_empty() && !self.default_alias {
            return alias_hash;
        }

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

        if self.default_alias {
            let mut insert = |dir: PathBuf, name: &str| {
                alias_hash.insert(dir.to_string_lossy().to_string(), format!("%{}", name));
            };

            insert(PROJECT_DIRS.hash_audio_dir().to_path_buf(), "MUSIC_DIR");
            insert(PROJECT_DIRS.hash_cache_dir().to_path_buf(), "CACHE_HOME");
            insert(PROJECT_DIRS.hash_config_dir().to_path_buf(), "CONFIG_HOME");
            insert(PROJECT_DIRS.hash_data_dir().to_path_buf(), "DATA_HOME");
            insert(PROJECT_DIRS.hash_desktop_dir().to_path_buf(), "DESKTOP");
            insert(PROJECT_DIRS.hash_document_dir().to_path_buf(), "DOCUMENTS");
            insert(PROJECT_DIRS.hash_download_dir().to_path_buf(), "DOWNLOADS");
            insert(PROJECT_DIRS.hash_executable_dir().to_path_buf(), "BIN_HOME");
            insert(PROJECT_DIRS.hash_font_dir().to_path_buf(), "FONTS_DIR");
            insert(PROJECT_DIRS.hash_picture_dir().to_path_buf(), "PICTURES");
            insert(PROJECT_DIRS.hash_public_dir().to_path_buf(), "PUBLIC_DIR");
            insert(PROJECT_DIRS.hash_template_dir().to_path_buf(), "TEMPLATES");
            insert(PROJECT_DIRS.hash_video_dir().to_path_buf(), "VIDEO_DIR");

            // Lastly, do `$HOME` so all others will be replaced first
            insert(PROJECT_DIRS.home_dir().to_path_buf(), "HOME");
        }

        alias_hash
    }
}

/// Wrapper around [`Alignment`](tui::layout::Alignment) to provide
/// serialization for the user configuration
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum HeaderAlignment {
    Left,
    Center,
    Right,
}

impl From<HeaderAlignment> for Alignment {
    fn from(other: HeaderAlignment) -> Self {
        match other {
            HeaderAlignment::Left => Self::Left,
            HeaderAlignment::Center => Self::Center,
            HeaderAlignment::Right => Self::Right,
        }
    }
}

impl FromStr for HeaderAlignment {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().trim() {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Ok(Self::Center),
        }
    }
}
