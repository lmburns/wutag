//! Keybindings for help menu within TUI.
//!
//! Credit for the idea of how to implement and build this module can be given
//! to `orhun` on GitHub, as I am still learning how to build applications from
//! the ground up.

// Keybinding {
//     key:         "hjkl,arrows,pgkeys",
//     action:      "navigate",
//     description: r#"
//     Scrolls the current widget or selects the next/previous tab.
//     M-<key>: scroll the table rows
//     C-<key>,pgup,pgdown: scroll to top/bottom
//     :scroll (row) up/down/left/right <amount>
//     "#,

#![allow(clippy::non_ascii_literal)]

use crate::config::KeyConfig;
use once_cell::sync::Lazy;
use std::{collections::HashMap, fmt};
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::ListItem,
};

use super::{
    event::Key,
    ui_app::color::{GREEN, MAGENTA, PINK, YELLOW},
};

// Would be string slices, but I haven't figured out how to convert an enum
// value to a string slice. An error is given about temporary variables being
// referenced at the same time they're being dropped at the end of the function.
// It would make it much easier if they could be slices

/// Representation of a keybinding
#[derive(Debug, Clone, Default)]
pub(crate) struct Keybinding {
    /// Keybinding
    pub(crate) key:         String,
    /// Action that the keybinding executes
    pub(crate) action:      String,
    /// Description of the keybinding that is shown in the help screen
    pub(crate) description: String,
}

impl fmt::Display for Keybinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // └─
        write!(
            f,
            "{}\n └─{}\n ",
            self.key
                .split(',')
                .fold(String::new(), |acc, v| format!("{}[{}] ", acc, v)),
            self.action
        )
    }
}

impl Keybinding {
    /// Build an instance of [`Keybinding`]
    pub(crate) const fn new(key: String, action: String, description: String) -> Self {
        Self {
            key,
            action,
            description,
        }
    }

    /// Return description of a keybinding
    pub(crate) fn get_description_text(&self, command_style: Style) -> Text {
        let mut lines = Vec::new();

        for line in self.description.lines().map(|v| format!("{}\n", v.trim())) {
            lines.push(if line.starts_with(':') {
                Spans::from(Span::styled(line, command_style))
            } else {
                Spans::from(line)
            });
        }
        Text::from(lines)
    }

    /// Returns [`Keybinding`] as a [`ListItem`]
    pub(crate) fn as_list_item(&self, colored: bool, highlighted: bool) -> ListItem {
        // .fg(Color::Reset)
        // Current selection

        let highlight_style = if highlighted {
            Style::default()
                .fg(Color::Rgb(PINK[0], PINK[1], PINK[2]))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(MAGENTA[0], MAGENTA[1], MAGENTA[2]))
        };

        ListItem::new(if colored {
            Text::from(vec![
                Spans::from(self.key.split(',').fold(Vec::new(), |mut keys, key| {
                    keys.push(Span::styled("[", highlight_style));
                    keys.push(Span::styled(
                        key,
                        if highlighted {
                            Style::default()
                                .fg(Color::Rgb(YELLOW[0], YELLOW[1], YELLOW[2]))
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::Rgb(YELLOW[0], YELLOW[1], YELLOW[2]))
                                .add_modifier(Modifier::DIM)
                                .add_modifier(Modifier::BOLD)
                        },
                    ));
                    keys.push(Span::styled("] ", highlight_style));
                    keys
                })),
                // └─
                Spans::from(vec![
                    Span::styled(" └─", Style::default().fg(Color::DarkGray)),
                    Span::styled(self.action.clone(), highlight_style),
                ]),
                Spans::default(),
            ])
        } else {
            Text::raw(self.to_string())
        })
    }
}
