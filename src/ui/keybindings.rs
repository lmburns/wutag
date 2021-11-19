#![allow(clippy::non_ascii_literal)]
//! Keybindings for help menu within TUI

use once_cell::sync::Lazy;
use std::fmt;
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::ListItem,
};

use super::ui_app::{GREEN, MAGENTA, PINK, YELLOW};

/// Representation of a keybinding
#[derive(Debug, Clone, Copy)]
pub(crate) struct Keybinding<'a> {
    /// Keybinding
    key:                    &'a str,
    /// Action that the keybinding executes
    action:                 &'a str,
    /// Description of the keybinding that is shown in the help screen
    pub(crate) description: &'a str,
}

impl fmt::Display for Keybinding<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            // └─
            format!(
                "{}\n \u{2514}\u{2500}{}\n ",
                self.key
                    .split(',')
                    .fold(String::new(), |acc, v| format!("{}[{}] ", acc, v)),
                self.action
            )
        )
    }
}

impl<'a> Keybinding<'a> {
    /// Build an instance of [`Keybinding`]
    pub(crate) fn new(key: &'a str, action: &'a str, description: &'a str) -> Self {
        Self {
            key,
            action,
            description,
        }
    }

    /// Return description of a keybinding
    pub(crate) fn get_description_text(&self, command_style: Style) -> Text<'a> {
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
    pub(crate) fn as_list_item(&self, colored: bool, highlighted: bool) -> ListItem<'a> {
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
                    Span::styled(self.action, highlight_style),
                ]),
                Spans::default(),
            ])
        } else {
            Text::raw(self.to_string())
        })
    }
}

/// Collection of all possible keybindings found in the `wutag` TUI
pub(crate) static KEYBINDINGS: Lazy<&[Keybinding]> = Lazy::new(|| {
    &[
        Keybinding {
            key:         "?",
            action:      "show help",
            description: r#"
            Show this help menu
            "#,
        },
        Keybinding {
            key:         "esc,q",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "testing,key",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "testing,again",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "testing,even",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "haha",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "more",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "lol",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "long",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "again",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "abc",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "further",
            action:      "exit",
            description: r#"
            Exit the program
            "#,
        },
        Keybinding {
            key:         "bottom",
            action:      "exit",
            description: r#"
            Exit the program
            :ok test
            "#,
        },
        Keybinding {
            key:         "hjkl,arrows,pgkeys",
            action:      "navigate",
            description: r#"
            Scrolls the current widget or selects the next/previous tab.
            M-<key>: scroll the table rows
            C-<key>,pgup,pgdown: scroll to top/bottom
            :scroll (row) up/down/left/right <amount>
            "#,
        },
    ]
});
