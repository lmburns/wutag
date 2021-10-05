//! Commands available throughout the TUI part of this crate.
//!
//! Other structures are that relate only to the execution of a command within
//! the TUI are also defined within this file

use std::{fmt, str::FromStr};

/// Specifies the `wutag` subcommand to run within the TUI
///
/// [`UiApp`](super::ui_app::UiApp)
#[derive(Clone, Debug, PartialEq)]
#[allow(variant_size_differences)]
pub(crate) enum Command {
    /// Confirm the execution of a command.
    Confirm(Box<Command>),
    /// Show help.
    ShowHelp,
    /// Quit application
    Quit,
    /// Guess? Do nothing
    None,
    /// Select next tab
    NextTab,
    /// Select previous tab
    PreviousTab,
    /// Refresh application
    Refresh,
    /// List files and/or tags
    List(ListType),
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
            Self::FilesTags => "files/tags",
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
