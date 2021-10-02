//! Style of the entire UI. That is, colored or not
use std::{
    fmt::{self, Display},
    str::FromStr,
};
use tui::{
    style::{Color, Style},
    text::{Span, Spans, Text},
};

/// UI style
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TuiStyle {
    /// Plain style with no colors
    Plain,
    /// Colored paths and colored tags
    Colored,
}

impl Default for TuiStyle {
    fn default() -> Self {
        Self::Colored
    }
}

impl Display for TuiStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

impl FromStr for TuiStyle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().trim() {
            "plain" | "false" | "0" | "no" => Ok(Self::Plain),
            "colored" | "true" | "1" | "yes" => Ok(Self::Colored),
            _ => Err(String::from("could not parse TuiStyle style")),
        }
    }
}

impl TuiStyle {
    /// Returns `true` if the style is [`Self::Colored`].
    ///
    /// [`Colored`](Self::Colored)
    pub(crate) fn is_colored(self) -> bool {
        self == Self::Colored
    }

    /// Returns the next style
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Plain => Self::Colored,
            Self::Colored => Self::Plain,
        }
    }
}
