//! Commands available throughout the TUI part of this crate. This is a superset
//! of the [`Command`](crate::opt::Command) found in the command-line only
//! section
//!
//! Other structures are that relate only to the execution of a command within
//! the TUI are also defined within this file

use super::ui_app::UiApp;
use anyhow::Result;
use std::{fmt, str::FromStr};

/// Specifies the commands available within the TUI
///
/// Used to display a menu of commands that are available. Commands can be ran
/// through the menu, some keybindings, or the command prompt
#[derive(Clone, Debug, PartialEq)]
#[allow(variant_size_differences)]
pub(crate) enum Command {
    // #### Builtin ####
    /// ...do nothing
    None,
    /// Quit application
    Quit,
    /// Refresh application
    Refresh,
    /// Show help menu
    ShowHelp,
    /* #### Movement ####
     * /// Select next tab
     * NextTab,
     * /// Select previous tab
     * PrevTab,
     *
     * // #### Actions on Tags ####
     * /// Add tag to a file                (pattern, tag(s))
     * Add(String, String),
     * /// Set tag(s) on a file             (pattern, tag(s))
     * Set(String, String),
     * /// Clear tags on file               (pattern)
     * Clear(String),
     * /// Remove tags on file              (pattern, tag(s))
     * Remove(String, String),
     * /// Edit the files tags in an editor (pattern)
     * Edit(String),
     * /// Search for files that are tagged (pattern)
     * Search(Option<String>),
     * /// Copy tag(s) to another file      (path,pattern)
     * Cp(String, String),
     *
     * // #### Meta/Other Actions ####
     * /// List files and/or tags
     * List(ListType),
     * /// Preview file
     * PreviewFile(bool),
     *
     * /// Confirm the execution of a command.
     * Confirm(Box<Command>), */
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Command::None => String::from("close menu"),
            Command::Quit => String::from("quit application"),
            Command::Refresh => String::from("refresh application"),
            Command::ShowHelp => String::from("show help"),
            // Command::NextTab => String::from("go to next tab"),
            // Command::PrevTab => String::from("go to previous tab"),
            // Command::Edit(path) => format!("edit the file {}", path),
            // Command::List(list) => format!("list {}", list),
            // Command::PreviewFile(yes) =>
            //     format!("file preview ({})", if *yes { "on" } else { "off" }),
            // Command::Confirm(command) => (*command).to_string(),
        })
    }
}

// Implement the parsing of the command prompt
impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut full_cmd = s
            .to_ascii_lowercase()
            .split_whitespace()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        let command = full_cmd.first().cloned().unwrap_or_default();
        let args = full_cmd.drain(1..).collect::<Vec<String>>();

        match command.as_str() {
            "@help" => Ok(Command::ShowHelp),
            "@quit" => Ok(Command::Quit),
            "@refresh" => Ok(Command::Refresh),
            "none" | "@none" => Ok(Command::None),
            _ => Err(()),
        }
    }
}

/// List subcommand type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ListType {
    /// Only list files
    Files,
    /// Only list tags
    Tags,
    /// List files and tags (default)
    FilesTags,
}

impl fmt::Display for ListType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Files => "files",
            Self::Tags => "tags",
            Self::FilesTags => "files and tags",
        })
    }
}

impl FromStr for ListType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().trim() {
            "file" | "files" | "f" => Ok(Self::Files),
            "tag" | "tags" | "t" => Ok(Self::Tags),
            "file/tag" | "files/tags" | "ft" => Ok(Self::FilesTags),
            _ => Err(String::from("Unable to parse 'list' type")),
        }
    }
}

// #[allow(clippy::unnecessary_wraps)]
// pub(crate) fn handle_command(app: &UiApp) -> Result<()> {
//     let command = Command::from_str(app.command_buffer);
//     Ok(())
// }
