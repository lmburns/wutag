#![allow(unused)]
#![allow(clippy::unused_self)]
#![allow(clippy::non_ascii_literal)]

// Note: This is heavily based off of taskwarrior-tui
// github.com/kdheepak/taskwarrior-tui
//
// I used their outline to help me learn how to code a TUI

// RUN_ONCE.get_or_init(|| super::notify("made it", None));

// TODO: ListItem
// TODO: Local path
// TODO: Keybindings read from configuration file

// TODO: Command prompt in HelpMenu
// TODO: :exit :help

use anyhow::{anyhow, Context, Result};
use clap::IntoApp;
use colored::{ColoredString, Colorize};
use lexiclean::Lexiclean;
use rand::seq::SliceRandom;
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
    convert::{TryFrom, TryInto},
    env, fmt, fs, io,
    path::{Path, PathBuf},
    process,
    str::FromStr,
    time::{Duration, SystemTime},
};
use thiserror::Error;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    terminal::Frame,
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};

use once_cell::sync::{Lazy, OnceCell};
use regex::{Captures, Regex};
use rustyline::{
    history::SearchDirection as HistoryDirection, line_buffer::LineBuffer, At, Editor, Word,
};
use rustyline_derive::Helper;
use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;
use wutag_core::{
    color::{color_tui_from_fg_str, parse_color_tui, TuiColor},
    tag::Tag,
};

use super::{
    completion::{self, CompletionList},
    event::Key,
    history::HistoryContext,
    keybindings::Keybinding,
    list::StatefulList,
    table::{Row, Table, TableSelection, TableState},
};

use crate::{
    config::{Config, HeaderAlignment},
    opt::{Command, Opts},
    registry::{EntryData, EntryId, TagRegistry},
    subcommand::App,
    wutag_fatal,
};

static RUN_ONCE: OnceCell<Result<()>> = OnceCell::new();

const MAX_LINE: usize = 4096;

pub(crate) const FG: [u8; 3] = [232, 192, 151];
pub(crate) const FG2: [u8; 3] = [217, 174, 128];
pub(crate) const PINK: [u8; 3] = [239, 29, 85];
pub(crate) const DARK_PINK: [u8; 3] = [152, 103, 106];
pub(crate) const DARK_PURPLE: [u8; 3] = [115, 62, 139];
pub(crate) const MAGENTA: [u8; 3] = [160, 100, 105];
pub(crate) const BLUE: [u8; 3] = [126, 178, 177];
pub(crate) const DARK_BLUE: [u8; 3] = [76, 150, 168];
pub(crate) const YELLOW: [u8; 3] = [255, 149, 0];
pub(crate) const ORANGE: [u8; 3] = [255, 88, 19];
pub(crate) const GREEN: [u8; 3] = [129, 156, 59];
pub(crate) const BRIGHT_GREEN: [u8; 3] = [163, 185, 90];

/// Errors used within the UI module of this crate
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// IO errors
    #[error("IO Error: {0}")]
    IOError(#[source] io::Error),

    #[error("failure to edit the current file: {0}")]
    Edit(#[from] anyhow::Error),
}

// === Helper functions ===

/// Draw a popup rectangle in the center of the screen
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

/// UI aspect of this App
#[derive(Debug)]
pub(crate) struct UiApp {
    pub(crate) command:                 LineBuffer,
    pub(crate) command_history_context: HistoryContext,
    pub(crate) completion_list:         CompletionList,
    pub(crate) completion_show:         bool,
    pub(crate) config:                  Config,
    pub(crate) current_context:         String,
    pub(crate) current_context_command: String,
    pub(crate) current_directory:       String,
    pub(crate) current_selection:       usize,
    pub(crate) current_selection_id:    Option<EntryId>,
    pub(crate) current_selection_path:  Option<PathBuf>,
    pub(crate) dirty:                   bool,
    pub(crate) error:                   String,
    pub(crate) file_details:            HashMap<EntryId, String>, // TODO: Show a stat command
    pub(crate) history_status:          Option<String>,
    pub(crate) keybindings:             StatefulList<Keybinding>,
    pub(crate) last_export:             Option<SystemTime>,
    pub(crate) list_height:             u16,
    pub(crate) list_state:              ListState,
    pub(crate) marked:                  HashSet<EntryId>,
    pub(crate) mode:                    AppMode,
    pub(crate) paths_color:             Color,
    pub(crate) preview_file:            bool, // TODO: Show a file preview
    pub(crate) preview_height:          u16,
    pub(crate) preview_scroll:          u16,
    pub(crate) registry:                TagRegistry,
    pub(crate) registry_paths:          Vec<PathBuf>,
    pub(crate) should_quit:             bool,
    pub(crate) table_state:             TableState,
    pub(crate) terminal_height:         u16,
    pub(crate) terminal_width:          u16,
}

/// Mode that application is in
#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(single_use_lifetimes)]
pub(crate) enum AppMode {
    List,
    Error,
    Help,
    Command,
    /* Remove,
     * Set,
     * Clear,
     * Search,
     * Cp,
     * Edit,
     * View,
     * Clear */
}

impl fmt::Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AppMode::List => write!(f, "List"),
            AppMode::Error => write!(f, "Error"),
            AppMode::Help => write!(f, "Help"),
            AppMode::Command => write!(f, "Command"),
        }
    }
}

impl UiApp {
    /// Create a new instance of the `UiApp`
    pub(crate) fn new(c: Config, reg: TagRegistry) -> Result<Self> {
        let (w, h) = crossterm::terminal::size()?;
        let mut state = ListState::default();
        if !reg.entries.is_empty() {
            state.select(Some(0));
        }

        let parsed_color = parse_color_tui(c.clone().ui.paths_color).unwrap_or_else(|_| {
            if let Some(color) = c.clone().base_color {
                parse_color_tui(color).unwrap_or(Color::Blue)
            } else {
                Color::Blue
            }
        });

        let cwd = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from(".").lexiclean())
            .display()
            .to_string();

        let mut uiapp = Self {
            command:                 LineBuffer::with_capacity(MAX_LINE),
            command_history_context: HistoryContext::new("command.history")?,
            completion_list:         CompletionList::with_items(vec![]),
            completion_show:         false,
            config:                  c.clone(),
            current_context:         String::from(""),
            current_context_command: String::from(""),
            current_directory:       cwd,
            current_selection:       state.selected().unwrap_or(0),
            current_selection_id:    None,
            current_selection_path:  None,
            dirty:                   false,
            error:                   String::from(""),
            file_details:            HashMap::new(),
            history_status:          None,
            keybindings:             StatefulList::default(),
            last_export:             None,
            list_height:             0,
            list_state:              state,
            marked:                  HashSet::new(),
            mode:                    AppMode::List,
            paths_color:             parsed_color,
            preview_file:            false,
            preview_height:          0,
            preview_scroll:          0,
            registry:                reg,
            registry_paths:          Vec::new(),
            should_quit:             false,
            table_state:             TableState::default(),
            terminal_height:         h,
            terminal_width:          w,
        };

        for ch in c.ui.startup_cmd.unwrap_or_default().chars() {
            uiapp.command.insert(ch, 1);
        }

        uiapp.get_context();
        uiapp.import_paths();
        uiapp.get_keybindings();
        uiapp.config.ui.build_alias_hash();
        uiapp.update(true)?;
        uiapp.command_history_context.load()?;

        // uiapp.get_keybindings();

        Ok(uiapp)
    }

    // ####################### GET INFO #######################
    //

    /// Generate the keybindings help display
    pub(crate) fn get_keybindings(&mut self) {
        let keys = self.config.keys;
        let gen_key = |key: Key, alt: Option<&str>, desc: &str| -> Keybinding {
            if let Some(alt) = alt {
                Keybinding::new(
                    format!("{},{}", key.name(), alt),
                    keys.fieldname(key),
                    desc.to_string(),
                )
            } else {
                Keybinding::new(key.name(), keys.fieldname(key), desc.to_string())
            }
        };

        let keybindings = vec![
            gen_key(
                keys.help,
                None,
                "Show the help menu/Return to main screen\n:help",
            ),
            Keybinding::new(
                ":".to_string(),
                "command prompt".to_string(),
                "Enter a command in the prompt".to_string(),
            ),
            gen_key(keys.quit, Some("C-c"), "Exit the program\n:exit"),
            gen_key(keys.up, Some("Up"), "Move up"),
            gen_key(keys.down, Some("Down"), "Move down"),
            gen_key(keys.go_to_top, Some("Home"), "Go to the top of the list"),
            gen_key(
                keys.go_to_bottom,
                Some("End"),
                "Go to the bottom of the list",
            ),
            gen_key(keys.page_up, Some("PageUp"), "Move a page up"),
            gen_key(keys.page_down, Some("PageDown"), "Move a page down"),
            gen_key(keys.select_all, None, "Select all items"),
            gen_key(keys.select, None, "Select one item"),
            gen_key(keys.refresh, None, "Refresh the application\n:refresh"),
            gen_key(keys.add, None, "Add tag(s) to file(s)\n:add"),
            gen_key(keys.clear, None, "Clear tag(s) from file(s)\n:clear"),
            gen_key(keys.remove, None, "Remove tag(s) from file(s)\n:remove"),
            gen_key(keys.edit, None, "Edit tag(s) on file(s)\n:edit"),
            gen_key(keys.search, None, "Search for tag(s) or file(s)\n:search"),
            gen_key(
                keys.copy,
                None,
                "Copy tag(s) from one file to another\n:copy",
            ),
            // TODO:
            gen_key(keys.preview, None, "Preview a file in $PAGER\n:preview"),
        ];

        self.keybindings = StatefulList::with_items(keybindings);
    }

    /// Get current context as a string for displaying purposes
    pub(crate) fn get_context(&mut self) {
        self.current_context = format!(
            r#"
            Current directory: {}
            Current registry: {}
            "#,
            self.alias_replace(&self.current_directory),
            self.alias_replace(&self.registry.path.display().to_string())
        );
    }

    /// Get the rows of `Tag`s' to build the `Table`
    fn get_full_tag_hash(&self) -> BTreeMap<PathBuf, Vec<Tag>> {
        self.registry.list_all_paths_and_tags()
    }

    /// Get the rows of `Tag`s' to build the `Table` with tags as strings
    fn get_full_tag_hash_str(&mut self) -> BTreeMap<PathBuf, Vec<String>> {
        self.registry.list_all_paths_and_tags_as_strings()
    }

    // ####################### DRAWING #######################
    //

    /// Wrapper function that executes startup screen depending on the `AppMode`
    pub(crate) fn draw(&mut self, app: &App, f: &mut Frame<impl Backend>) {
        let rect = f.size();
        self.terminal_width = rect.width;
        self.terminal_height = rect.height;
        // Use for whenever (if ever) a new mode is added
        match self.mode {
            AppMode::List | AppMode::Error | AppMode::Help | AppMode::Command =>
                self.draw_tag(app, f),
        }
    }

    /// Draw startup screen to debug
    pub(crate) fn draw_debug(&mut self, f: &mut Frame<impl Backend>) {
        let area = centered_rect(50, 50, f.size());
        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(Text::from(format!("{}", self.current_selection))).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            area,
        );
    }

    /// Draw help menu showing user-defined/default keybindings
    pub(crate) fn draw_help(&mut self, f: &mut Frame<impl Backend>, rect: Rect, title: Vec<Span>) {
        f.render_widget(Clear, rect);
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Spans::from(title))
                .title_alignment(Alignment::Left),
            rect,
        );

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(rect);

        let description = self
            .keybindings
            .selected()
            .map(|s| {
                s.get_description_text(
                    Style::default()
                        .fg(Color::Rgb(DARK_BLUE[0], DARK_BLUE[1], DARK_BLUE[2]))
                        .add_modifier(Modifier::ITALIC),
                )
            })
            .unwrap_or_default();

        let description_height = u16::try_from(
            self.keybindings
                .selected()
                .map(|s| s.description.lines().count())
                .unwrap_or_default(),
        )
        .unwrap_or(1)
            + 2;

        {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Min(
                            chunks[0]
                                .height
                                .checked_sub(description_height)
                                .unwrap_or_default(),
                        ),
                        Constraint::Min(description_height),
                    ]
                    .as_ref(),
                )
                .split(chunks[0]);

            let list = List::new(
                self.keybindings
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        v.as_list_item(
                            self.is_colored(),
                            self.keybindings.state.selected() == Some(i),
                        )
                    })
                    .collect::<Vec<ListItem>>(),
            )
            .block(
                Block::default()
                    .borders(Borders::RIGHT)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(self.paths_color))
            .highlight_style(if self.is_colored() {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Reset)
                    .add_modifier(Modifier::BOLD)
            });

            // Clone is necessary to now borrow self as mutable and immutable
            f.render_stateful_widget(list, chunks[0], &mut self.keybindings.state.clone());

            f.render_widget(
                Paragraph::new(description.clone())
                    .block(
                        Block::default()
                            .borders(Borders::RIGHT)
                            .border_style(Style::default().fg(Color::DarkGray)),
                    )
                    .style(
                        Style::default()
                            .fg(if self.config.ui.flashy {
                                self.gen_random_color()
                            } else {
                                Color::Magenta
                            })
                            .add_modifier(Modifier::BOLD),
                    )
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: true }),
                chunks[1],
            );
        }
        {
            let context_height =
                u16::try_from(self.current_context.lines().count()).unwrap_or(1) + 1;

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Min(
                            chunks[1]
                                .height
                                .checked_sub(context_height)
                                .unwrap_or_default(),
                        ),
                        Constraint::Min(context_height),
                    ]
                    .as_ref(),
                )
                .split(chunks[1]);

            f.render_widget(
                Paragraph::new(if self.is_colored() {
                    self.styled_context(Color::Cyan)
                } else {
                    Text::raw(&self.current_context)
                })
                .block(
                    Block::default()
                        .borders(Borders::NONE)
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .style(Style::default().fg(self.paths_color))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true }),
                chunks[1],
            );
        }
    }

    /// Draw the startup screen
    pub(crate) fn draw_tag(&mut self, app: &App, f: &mut Frame<impl Backend>) {
        let rect = f.size();

        // Split screen
        // .constraints([Constraint::Percentage(80),
        // Constraint::Percentage(20)].as_ref())

        // Old help
        // .constraints([Constraint::Min(rect.height - 1), Constraint::Min(1)].as_ref())

        let set_title = |app: &UiApp, mode: String| -> Vec<Span> {
            let match_mode = |mode: AppMode| -> Modifier {
                if app.mode == mode {
                    Modifier::BOLD
                } else {
                    Modifier::DIM
                }
            };

            vec![
                app.set_header_style::<PINK>("Wutag", match_mode(AppMode::List)),
                app.set_header_style::<FG>("|", Modifier::SLOW_BLINK),
                app.set_header_style::<PINK>("Other", match_mode(AppMode::Help)),
                Span::from("──("),
                app.set_header_style::<FG>("Mode: ", Modifier::DIM),
                // FIX: Issues of returning value referencing function
                // Would be nice to use function above for this
                Span::styled(
                    mode,
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Rgb(ORANGE[0], ORANGE[1], ORANGE[2])),
                ),
                Span::from(")"),
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
            .split(rect);

        // TODO: switch logic (implement file preview)
        if self.preview_file {
            let split_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(chunks[0]);

            self.preview_height = split_layout[1].height;
            self.draw_table(
                app,
                f,
                split_layout[0],
                set_title(self, self.mode.to_string()),
            );
            self.draw_preview(f, split_layout[1]);
        } else {
            let full_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(chunks[0]);

            self.preview_height = full_layout[0].height;
            self.draw_table(
                app,
                f,
                full_layout[0],
                set_title(self, self.mode.to_string()),
            );
        };

        let empty_path = PathBuf::new();
        let selected = self.current_selection;

        // TODO: use
        // When selecting files, file_id is shown as files to modify
        let file_id = if self.registry.entries.is_empty() {
            vec!["0".to_string()]
        } else {
            match self.table_state.mode() {
                TableSelection::Single => {
                    vec!["SINGLE".to_string()]
                    // vec![self
                    //     .registry
                    //     .get_entry(selected)
                    //     .map_or(empty_path, |c| c.path().to_path_buf())
                    //     .display()
                    //     .to_string()]
                },
                TableSelection::Multiple => {
                    vec!["MULTIPLE".to_string()]
                    // let mut tag_uuids = vec![];
                    // for uuid in &self.marked {
                    //     if let Some(entry) = self.tag_by_uuid(*uuid) {
                    //         tag_uuids.push(self.registry.
                    // add_or_update_entry(entry).to_string());
                    //     }
                    // }
                    // tag_uuids
                },
            }
        };

        match self.mode {
            AppMode::List => self.draw_command(
                f,
                chunks[1],
                self.command.as_str(),
                self.set_header_style::<PINK>("Command Prompt", Modifier::DIM),
                self.get_position(&self.command),
                false,
            ),
            AppMode::Command => {
                let position = self.get_position(&self.command);
                if self.completion_show {
                    self.draw_completion_popup(f, chunks[1], position);
                }
                self.draw_command(
                    f,
                    chunks[1],
                    self.command.as_str(),
                    self.set_header_style::<PINK>("Command Prompt", Modifier::BOLD),
                    position,
                    true,
                );
            },
            AppMode::Error =>
                self.draw_command(f, chunks[1], self.error.as_str(), "Error", 0, false),
            AppMode::Help => {
                self.draw_command(
                    f,
                    chunks[1],
                    self.command.as_str(),
                    self.set_header_style::<PINK>("Command Prompt", Modifier::BOLD),
                    self.get_position(&self.command),
                    false,
                );
                self.draw_help(f, chunks[0], set_title(self, self.mode.to_string()));
            },
        }
    }

    #[allow(single_use_lifetimes)]
    fn draw_command<'a, T>(
        &self,
        f: &mut Frame<impl Backend>,
        rect: Rect,
        text: &str,
        title: T,
        position: usize,
        cursor: bool,
    ) where
        T: Into<Spans<'a>>,
    {
        f.render_widget(Clear, rect);
        if cursor {
            f.set_cursor(
                std::cmp::min(
                    rect.x + position as u16 + 1,
                    rect.x + rect.width.saturating_sub(2),
                ),
                rect.y + 1,
            );
        }

        let p = Paragraph::new(Text::from(text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(Color::Rgb(FG[0], FG[1], FG[2])))
                    .title(title.into()),
            )
            .scroll((0, ((position + 3) as u16).saturating_sub(rect.width)));
        f.render_widget(p, rect);
    }

    // .alignment(Alignment::Left)
    // .wrap(Wrap { trim: false })

    /// TODO: Draw a file preview
    fn draw_preview(&mut self, f: &mut Frame<impl Backend>, rect: Rect) {
        if self.registry.entries.is_empty() {
            f.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("No tagged file found"),
                rect,
            );
            return;
        }

        let selected = self.selected();
        // TODO: fix
        let path = self
            .current_selection_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        // let data = match self.task_details.get(&task_uuid) {
        //     Some(s) => s.clone(),
        //     None => "Loading task details ...".to_string(),
        // };
        // self.task_details_scroll = std::cmp::min(
        //     (data.lines().count() as u16)
        //         .saturating_sub(rect.height)
        //         .saturating_add(2),
        //     self.task_details_scroll,
        // );

        let p = Paragraph::new(Text::from("File Preview"))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(format!("Entry: {}", path.display())),
            )
            .scroll((self.preview_scroll, 0));
        f.render_widget(p, rect);
    }

    /// Draw the tag table (filepaths tags)
    fn draw_table(&mut self, app: &App, f: &mut Frame<impl Backend>, rect: Rect, title: Vec<Span>) {
        let entries = self.get_full_tag_hash();
        let headers = vec!["Filename", "Tag(s)"]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        if entries.is_empty() {
            // TODO: test this

            f.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Spans::from(title)),
                rect,
            );
            return;
        }

        let entries_name = entries.iter().fold(Vec::new(), |mut acc, (k, v)| {
            acc.push(vec![
                k.display().to_string(),
                v.iter()
                    .map(|tag| tag.name().to_string())
                    .collect::<Vec<String>>()
                    .join(" "),
            ]);
            acc
        });

        let maximum_column_width = rect.width;
        let widths = self.calculate_widths(&entries_name, &headers, maximum_column_width);

        // for (idx, header) in headers.iter().enumerate() {
        //     if header == "Tag(s)" {
        //         self.tag_widths = widths[idx] - 1;
        //         break;
        //     }
        // }

        let header = headers.iter();
        let mut rows = vec![];
        let mut hl_style = Style::default();
        let mut mods = Modifier::empty();

        for (idx, entry) in entries_name.iter().enumerate() {
            let style = if self.is_colored() {
                if self.config.ui.paths_bold {
                    Style::default()
                        .fg(self.paths_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(self.paths_color)
                }
            } else if self.config.ui.paths_bold {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            if idx == self.selected() {
                hl_style = style;
                if self.config.ui.selection_bold {
                    mods |= Modifier::BOLD;
                }
                if self.config.ui.selection_italic {
                    mods |= Modifier::ITALIC;
                }
                if self.config.ui.selection_dim {
                    mods |= Modifier::DIM;
                }
                if self.config.ui.selection_blink {
                    mods |= Modifier::SLOW_BLINK;
                }
                hl_style = hl_style.add_modifier(mods);
            }
            rows.push(Row::new(vec![
                Text::from(Spans::from(vec![Span::styled(
                    self.alias_replace(&entry[0]),
                    style,
                )])),
                self.styled_text_for_tags(entry),
            ]));
        }

        let constraints: Vec<Constraint> = widths
            .iter()
            .map(|i| Constraint::Length((*i).try_into().unwrap_or(maximum_column_width)))
            .collect();

        let table = Table::new(header, rows)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(Color::Rgb(FG[0], FG[1], FG[2])))
                    .title(Spans::from(title))
                    .title_alignment(Alignment::Left),
            )
            .header_style(if self.is_colored() {
                Style::default()
                    .fg(Color::Rgb(DARK_PINK[0], DARK_PINK[1], DARK_PINK[2]))
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            })
            .header_alignment(
                // Seems unncessary to have to convert from string
                Alignment::from(
                    HeaderAlignment::from_str(&self.config.ui.header_alignment).unwrap(),
                ),
            )
            .highlight_style(hl_style)
            .highlight_tags(self.config.ui.selection_tags)
            .highlight_symbol(&self.config.ui.selection_indicator)
            .mark_symbol(&self.config.ui.mark_indicator)
            .unmark_symbol(&self.config.ui.unmark_indicator)
            .widths(&constraints);

        f.render_stateful_widget(table, rect, &mut self.table_state);
    }

    /// Draw the completion list pop-up
    fn draw_completion_popup(
        &mut self,
        f: &mut Frame<impl Backend>,
        rect: Rect,
        cursor_position: usize,
    ) {
        if self.completion_list.candidates().is_empty() {
            self.completion_show = false;
            return;
        }

        // Iterate through all elements in the `items` app and append some debug text to
        // it.
        let items: Vec<ListItem> = self
            .completion_list
            .candidates()
            .iter()
            .map(|p| {
                let lines = vec![Spans::from(p.display.clone())];
                ListItem::new(lines).style(Style::default().fg(Color::Rgb(FG[0], FG[1], FG[2])))
            })
            .collect();

        // self.config.ui.completion_color

        // Create a List from all list items and highlight the currently selected one
        let items = List::new(items)
            .block(Block::default().borders(Borders::NONE).title(""))
            .style(Style::default().fg(Color::Red))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Rgb(ORANGE[0], ORANGE[1], ORANGE[2])),
            )
            .highlight_symbol(&self.config.ui.selection_indicator);

        let area = f.size();

        let mut rect = rect;
        rect.height = std::cmp::min(area.height / 2, self.completion_list.len() as u16 + 2);
        rect.width = std::cmp::min(
            area.width / 2,
            self.completion_list
                .max_width()
                .unwrap_or(40)
                .try_into()
                .unwrap_or(area.width / 2),
        );
        rect.y = rect.y.saturating_sub(rect.height);
        if cursor_position as u16 + rect.width >= area.width {
            rect.x = area.width - rect.width;
        } else {
            rect.x = cursor_position as u16;
        }

        f.render_widget(Clear, rect);
        f.render_stateful_widget(items, rect, &mut self.completion_list.state);
    }

    // ####################### HELPER FUNCTIONS #######################
    //

    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn handle_input(&mut self, input: Key) -> Result<()> {
        match self.mode {
            AppMode::List =>
                if input == self.config.keys.quit || input == Key::Ctrl('c') {
                    self.should_quit = true;
                } else if input == Key::Esc {
                    self.marked.clear();
                } else if input == self.config.keys.refresh {
                    self.update(true)?;
                } else if input == self.config.keys.go_to_bottom || input == Key::End {
                    self.move_to_bottom();
                } else if input == self.config.keys.go_to_top || input == Key::Home {
                    self.move_to_top();
                } else if input == Key::Down || input == self.config.keys.down {
                    self.move_to_next();
                } else if input == Key::Up || input == self.config.keys.up {
                    self.move_to_previous();
                } else if input == Key::PageDown || input == self.config.keys.page_down {
                    self.move_to_next_page();
                } else if input == Key::PageUp || input == self.config.keys.page_up {
                    self.move_to_previous_page();
                } else if input == self.config.keys.select {
                    self.table_state.multiple_selection();
                    self.toggle_mark();
                } else if input == self.config.keys.select_all {
                    self.table_state.multiple_selection();
                    self.toggle_mark_all();
                } else if input == self.config.keys.help {
                    self.mode = AppMode::Help;
                } else if input == Key::Char(':') {
                    self.mode = AppMode::Command;
                    self.command_history_context.last();
                    self.update_completion_list();
                } else if input == self.config.keys.edit {
                    match self.tag_edit2() {
                        Ok(_) => self.update(true)?,
                        Err(e) => {
                            self.mode = AppMode::Error;
                            self.error = e;
                        },
                    }
                },
            AppMode::Help =>
                if input == Key::Ctrl('c') {
                    self.should_quit = true;
                } else if input == self.config.keys.quit
                    || input == self.config.keys.help
                    || input == Key::Esc
                {
                    self.mode = AppMode::List;
                } else if input == Key::Down || input == self.config.keys.down {
                    self.keybindings.next();
                } else if input == Key::Up || input == self.config.keys.up {
                    self.keybindings.previous();
                },
            AppMode::Command => match input {
                Key::Esc =>
                    if self.completion_show {
                        self.completion_show = false;
                        self.completion_list.unselect();
                    } else {
                        self.mode = AppMode::List;
                        self.command_history_context.add(self.command.as_str());
                        self.update(true)?;
                    },
                Key::Char('\n') => {
                    if self.completion_show {
                        self.completion_show = false;
                        if let Some(sel) = self.completion_list.selected() {
                            let (before, after) =
                                self.command.as_str().split_at(self.command.pos());
                            let f = format!("{}{}{}", before, sel, after);
                            self.command.update(&f, self.command.pos() + sel.len());
                        }
                        self.completion_list.unselect();
                        self.dirty = true;
                    } else {
                        // TODO: add error
                        self.mode = AppMode::List;
                        self.command_history_context.add(self.command.as_str());
                        self.update(true)?;
                    }
                },
                Key::Up =>
                    if self.completion_show && !self.completion_list.is_empty() {
                        self.completion_list.previous();
                    } else if let Some(s) = self.command_history_context.history_search(
                        &self.command.as_str()[..self.command.pos()],
                        HistoryDirection::Reverse,
                    ) {
                        let p = self.command.pos();
                        self.command.update("", 0);
                        self.command.update(&s, std::cmp::min(p, s.len()));
                        self.dirty = true;
                    },
                Key::Down =>
                    if self.completion_show && !self.completion_list.is_empty() {
                        self.completion_list.next();
                    } else if let Some(s) = self.command_history_context.history_search(
                        &self.command.as_str()[..self.command.pos()],
                        HistoryDirection::Forward,
                    ) {
                        let p = self.command.pos();
                        self.command.update("", 0);
                        self.command.update(&s, std::cmp::min(p, s.len()));
                        self.dirty = true;
                    },
                Key::Tab | Key::Ctrl('n') =>
                    if !self.completion_list.is_empty() {
                        self.update_completion_matching();
                        if !self.completion_show {
                            self.completion_show = true;
                        }
                        self.completion_list.next();
                    },
                Key::BackTab | Key::Ctrl('p') => {
                    if self.completion_show && !self.completion_list.is_empty() {
                        self.completion_list.previous();
                    }
                },
                Key::Ctrl('r') => {
                    self.command.update("", 0);
                    for c in self
                        .config
                        .clone()
                        .ui
                        .startup_cmd
                        .unwrap_or_default()
                        .chars()
                    {
                        self.command.insert(c, 1);
                    }
                    self.update_completion_matching();
                    self.dirty = true;
                },
                _ => {
                    handle_movement(&mut self.command, input);
                    // self.check_command_status()?;
                    // self.update_completion_list();
                    self.complist();
                    self.update_completion_matching();
                    self.dirty = true;
                },
            },
            AppMode::Error => self.mode = AppMode::List,
            /* } else if input == self.config.keys.go_to_bottom || input == Key::End {
             *     self.move_to_bottom();
             * } else if input == self.config.keys.go_to_top || input == Key::Home {
             *     self.move_to_top();
             * } else if input == Key::PageDown || input == self.config.keys.page_down {
             *     self.move_to_next_page();
             * } else if input == Key::PageUp || input == self.config.keys.page_up {
             *     self.move_to_previous_page();
             * }, */
        }

        self.update_table_state();
        Ok(())
    }

    /// Use this `alias_hash` from the configuration file to replace the
    /// mappings in the strings
    pub(crate) fn alias_replace(&self, replace: &str) -> String {
        let alias_hash = self.config.ui.clone().build_alias_hash();
        let re = Regex::new(
            format!(
                r"({})",
                alias_hash
                    .keys()
                    .map(Clone::clone)
                    .collect::<Vec<String>>()
                    .join("|")
            )
            .as_str(),
        )
        .unwrap();

        let new_path = if re.is_match(replace) {
            re.replace(replace, |caps: &Captures| {
                alias_hash.get(caps.get(1).unwrap().as_str()).unwrap()
            })
        } else {
            Cow::from(replace)
        };

        new_path.to_string()
    }

    /// Calculate the widths of the chunks for displaying
    pub(crate) fn calculate_widths(
        &self,
        entries: &[Vec<String>],
        headers: &[String],
        maximum_column_width: u16,
    ) -> Vec<usize> {
        let mut widths = headers.iter().map(String::len).collect::<Vec<usize>>();

        // super::destruct_terminal();

        for entry in entries.iter() {
            for (idx, cell) in entry.iter().enumerate() {
                // println!("IDX: {:#?}, CELL: {:#?}", idx, cell);
                widths[idx] = std::cmp::max(cell.len(), widths[idx]);
            }
        }

        // println!("WIDTH: {:#?}", widths);

        for (idx, header) in headers.iter().enumerate() {
            if header == "Tag(s)" {
                // Give Tag(s) the maximum room to breath as it is the most variable (usually)
                widths[idx] = maximum_column_width as usize;
                break;
            }
        }

        // std::process::exit(1);

        for (idx, header) in headers.iter().enumerate() {
            // TODO: What's this do?
            if header == "Filename" {
                // Filename is first column, so add width of selection indicator
                widths[idx] += self.config.ui.selection_indicator.as_str().width();
            }
        }

        // now start trimming
        while (widths.iter().sum::<usize>() as u16) >= maximum_column_width - (headers.len()) as u16
        {
            let index = widths
                .iter()
                .position(|i| i == widths.iter().max().unwrap_or(&0))
                .unwrap_or_default();
            if widths[index] == 1 {
                break;
            }
            widths[index] -= 1;
        }

        widths
    }

    /// Refresh the application state
    pub(crate) fn update(&mut self, force: bool) -> Result<()> {
        if force || self.dirty || self.changed_since(self.last_export).unwrap_or(true) {
            self.last_export = Some(SystemTime::now());
            self.import_paths();
            self.get_context();
            self.dirty = false;
            self.save_history()?;
        }

        self.cursor_fix();
        self.update_table_state();
        self.selection_fix();
        Ok(())
    }

    /// Update the state the table is in
    pub(crate) fn update_table_state(&mut self) {
        self.table_state.select(Some(self.current_selection));

        for id in self.marked.clone() {
            if self.path_by_id(id).is_none() {
                self.marked.remove(&id);
            }
        }

        if self.marked.is_empty() {
            self.table_state.single_selection();
        }

        self.table_state.clear();

        for id in &self.marked {
            self.table_state.mark(self.path_idx_by_id(*id));
        }
    }

    /// Save command history to a file
    pub(crate) fn save_history(&mut self) -> Result<()> {
        self.command_history_context.write()?;
        Ok(())
    }

    /// Whether the TUI is in a colored state
    pub(crate) fn is_colored(&self) -> bool {
        self.config.ui.colored_ui
    }

    // ####################### MOVEMENT #######################
    //

    pub(crate) fn previous_report(&mut self) {
        if self.registry.tags.is_empty() {
            return;
        }
        let selected = self.selected();
        let i = {
            if selected == 0 {
                if self.config.ui.looping {
                    self.registry.entries.len() - 1
                } else {
                    0
                }
            } else {
                selected - 1
            }
        };

        self.select(i);
        self.current_selection = i;
        self.current_selection_path = None;
        self.current_selection_id = None;
    }

    /// Go to the bottom of the screen
    pub(crate) fn move_to_bottom(&mut self) {
        if self.registry.entries.is_empty() {
            return;
        }
        self.select(self.registry.entries.len() - 1);
        self.current_selection = self.registry.entries.len() - 1;
        self.current_selection_id = None;
    }

    /// Go to the top of the screen
    pub(crate) fn move_to_top(&mut self) {
        if self.registry.entries.is_empty() {
            return;
        }
        self.select(0);
        self.current_selection = 0;
        self.current_selection_id = None;
    }

    /// Move to next item in list
    pub(crate) fn move_to_next(&mut self) {
        if self.registry.entries.is_empty() {
            return;
        }
        let selected = self.selected();
        let i = {
            if selected >= self.registry.entries.len() - 1 {
                if self.config.ui.looping {
                    0
                } else {
                    selected
                }
            } else {
                selected + 1
            }
        };
        self.select(i);
        self.current_selection = i;
        self.current_selection_id = None;
    }

    /// Move to previous item in list
    pub(crate) fn move_to_previous(&mut self) {
        if self.registry.entries.is_empty() {
            return;
        }
        let selected = self.selected();
        let i = {
            if selected == 0 {
                if self.config.ui.looping {
                    self.registry.entries.len() - 1
                } else {
                    0
                }
            } else {
                selected - 1
            }
        };
        self.select(i);
        self.current_selection = i;
        self.current_selection_id = None;
    }

    /// Move to next page
    pub(crate) fn move_to_next_page(&mut self) {
        if self.registry.entries.is_empty() {
            return;
        }
        let selected = self.selected();
        let i = {
            if selected == self.registry.entries.len() - 1 {
                if self.config.ui.looping {
                    0
                } else {
                    self.registry.entries.len() - 1
                }
            } else {
                std::cmp::min(
                    selected
                        .checked_add(self.list_height as usize)
                        .unwrap_or_else(|| self.registry.entries.len() - 1),
                    self.registry.entries.len() - 1,
                )
            }
        };
        self.select(i);
        self.current_selection = i;
        self.current_selection_id = None;
    }

    /// Move to previous page
    pub(crate) fn move_to_previous_page(&mut self) {
        if self.registry.entries.is_empty() {
            return;
        }
        let selected = self.selected();
        let i = {
            if selected == 0 {
                if self.config.ui.looping {
                    self.registry.entries.len() - 1
                } else {
                    0
                }
            } else {
                self.current_selection
                    .saturating_sub(self.list_height as usize)
            }
        };
        self.select(i);
        self.current_selection = i;
        self.current_selection_id = None;
    }

    /// Fix cursor position under any errors that may arise
    pub(crate) fn cursor_fix(&mut self) {
        while !self.registry.tags.is_empty() && self.current_selection >= self.registry.tags.len() {
            self.previous_report();
        }
    }

    // ####################### SELECTION #######################
    //

    /// Get position of cursor on screen
    pub(crate) fn get_position(&self, buf: &LineBuffer) -> usize {
        let mut position = 0;
        for (i, (i_, g)) in buf.as_str().grapheme_indices(true).enumerate() {
            if i_ == buf.pos() {
                break;
            }
            position += g.width();
        }
        position
    }

    /// Toggle mark on current selection
    pub(crate) fn toggle_mark(&mut self) {
        if !self.registry.tags.is_empty() {
            let selected = self.current_selection;
            if let Some(id) = self.registry.find_entry(&self.registry_paths[selected]) {
                if !self.marked.insert(id) {
                    self.marked.remove(&id);
                }
            }
        }
    }

    /// Toggle mark on every item in registry
    pub(crate) fn toggle_mark_all(&mut self) {
        for path in &self.registry_paths {
            if let Some(id) = self.registry.find_entry(&path) {
                if !self.marked.insert(id) {
                    self.marked.remove(&id);
                }
            }
        }
    }

    /// Fix selection of any errors that may arrise
    pub(crate) fn selection_fix(&mut self) {
        // if let (Some(t), Some(uuid)) = (self.tag_current(),
        // self.current_selection_id) {     if t.uuid() != &uuid {
        //         if let Some(i) = self.tag_index_by_uuid(uuid) {
        //             self.current_selection = i;
        //             self.current_selection_id = None;
        //         }
        //     }
        // }
    }

    /// Make the selection
    pub(crate) fn select(&mut self, selection: usize) {
        self.list_state.select(Some(selection));
    }

    /// Current selection
    pub(crate) fn selected(&mut self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    /// Currently selected task
    pub(crate) fn tag_current(&self) -> Option<EntryData> {
        if self.registry.tags.is_empty() {
            return None;
        }
        let selected = self.current_selection;

        Some(
            self.registry
                .get_entry(*(self.registry.list_entries_ids().collect::<Vec<_>>()[selected]))
                .expect("failed to get EntryData tag")
                .clone(),
        )
    }

    // /// Current selections `EntryId`
    // pub(crate) fn selected_task_ids(&self) -> Vec<EntryId> {
    //     let selected = match self.table_state.mode() {
    //         TableSelection::Single => vec![self.current_selection],
    //         TableSelection::Multiple =>
    // self.table_state.marked().copied().collect::<Vec<usize>>(),     };
    //
    //     let mut ids = vec![];
    //
    //     for s in selected {
    //         // let id = self.registry.find_entry()
    //         // let uuid = *self.registry.entries[&s].uuid();
    //         // uuids.push(uuid);
    //     }
    //
    //     ids
    // }

    // ####################### STYLE #######################
    //

    // This actually wasn't the intent, I meant to choose a single color from the
    // list of colors in the configuration file
    /// Creates a flashing color
    fn gen_random_color(&self) -> Color {
        let mut rng = rand::thread_rng();
        self.config
            .colors
            .clone()
            .map(|colors| {
                parse_color_tui(colors.choose(&mut rng).unwrap_or(&"blue".to_string()))
                    .unwrap_or_else(|_| {
                        [
                            Color::Red,
                            Color::LightRed,
                            Color::Green,
                            Color::LightGreen,
                            Color::Blue,
                            Color::LightBlue,
                            Color::Yellow,
                            Color::LightYellow,
                            Color::Magenta,
                            Color::LightMagenta,
                            Color::Cyan,
                            Color::LightCyan,
                            Color::Gray,
                            Color::DarkGray,
                            Color::Reset,
                        ]
                        .choose(&mut rng)
                        .copied()
                        .unwrap_or(Color::Blue)
                    })
            })
            .unwrap()
    }

    /// Return styled text for the context in the help menu
    fn styled_context(&self, color: Color) -> Text<'_> {
        Text::from(
            self.current_context
                .lines()
                .map(|v| {
                    let mut values = v.trim().split(':').collect::<Vec<&str>>();
                    Spans::from(if values.len() >= 2 {
                        vec![
                            Span::styled(values[0], Style::default().fg(Color::Reset)),
                            Span::styled(":", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                values.drain(1..).collect::<Vec<&str>>().join(":"),
                                Style::default()
                                    .fg(if self.config.ui.flashy {
                                        self.gen_random_color()
                                    } else {
                                        color
                                    })
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]
                    } else {
                        vec![Span::styled(v, Style::default().fg(Color::Reset))]
                    })
                })
                .collect::<Vec<Spans>>(),
        )
    }

    /// Returns a `Text` object of every styled `Tag`
    fn styled_text_for_tags<'a>(&self, entry: &[String]) -> Text<'a> {
        let mut row = vec![];

        let path = entry[0].clone();
        let id = self.registry.find_entry(&path).unwrap_or_default();
        let tags = self.registry.list_entry_tags(id).unwrap_or_default();

        // let mut colored = vec![Span::styled(path, Style::default())];
        let mut colored = vec![];

        for tag in tags {
            let mut style = Style::default();
            let mut modifiers = Modifier::empty();

            if self.is_colored() {
                if let Some(color) = color_tui_from_fg_str(&tag.color().to_fg_str()) {
                    style = style.fg(color);
                }
            }

            if self.config.ui.tags_bold {
                modifiers |= Modifier::BOLD;
            }

            style = style.add_modifier(modifiers);
            colored.push(Span::styled(tag.clone().name().to_string(), style));
        }

        row.push(Spans::from(colored));

        Text::from(row)
    }

    // TODO: use or delete
    /// Returns a vector of `Style` for a vector of (`PathBuf`, ...`Tag`)
    fn style_for_tags(&self, entry: &[String]) -> Vec<Style> {
        let mut styles = vec![];

        let id = self
            .registry
            .find_entry(entry[0].clone())
            .unwrap_or_default();

        let tags = self.registry.list_entry_tags(id).unwrap_or_default();

        // if let Ok(color) = parse_color_tui(tag.color().to_string()) {
        //     println!("PARSED FIRST");
        //     style = style.fg(color);

        for tag in tags {
            let mut style = Style::default();
            let mut modifiers = Modifier::empty();

            if let Some(color) = color_tui_from_fg_str(&tag.color().to_fg_str()) {
                style = style.fg(color);
            }

            modifiers |= Modifier::BOLD;
            style = style.add_modifier(modifiers);
            styles.push(style);
        }

        styles
    }

    // TODO: use or delete
    /// Return style for individual `Tag`
    fn style_for_tag(&self, tag: &Tag) -> Style {
        let mut style = Style::default();
        let mut modifiers = Modifier::empty();

        // if let Ok(color) = parse_color_tui(tag.color().to_string()) {
        //     println!("PARSED FIRST");
        //     style = style.fg(color);

        if let Some(color) = color_tui_from_fg_str(&tag.color().to_fg_str()) {
            style = style.fg(color);
        }

        modifiers |= Modifier::BOLD;
        style = style.add_modifier(modifiers);
        style
    }

    /// Return a styled `Span` based on user configuration
    fn set_header_style<'a, const COLOR: [u8; 3]>(
        &self,
        text: &'a str,
        modif: Modifier,
    ) -> Span<'a> {
        Span::styled(text, self.colored_style::<COLOR>(modif))
    }

    /// Return a `Style` depending on user configuration
    fn colored_style<const COLOR: [u8; 3]>(&self, modif: Modifier) -> Style {
        if self.is_colored() {
            Style::default()
                .add_modifier(modif)
                .fg(Color::Rgb(COLOR[0], COLOR[1], COLOR[2]))
        } else {
            Style::default()
        }
    }

    // INFO: Double const. Not used because can't use match statements
    // fn set_header_style<'a, const COLOR: [u8; 3], const MOD: Modifier>(
    //     &self,
    //     text: &'a str,
    // ) -> Span<'a> {
    //     Span::styled(text, self.colored_style::<COLOR, MOD>())
    // }
    //
    // /// Return a `Style` depending on user configuration
    // fn colored_style<const COLOR: [u8; 3], const MOD: Modifier>(&self) -> Style {
    //     if self.is_colored() {
    //         Style::default()
    //             .add_modifier(MOD)
    //             .fg(Color::Rgb(COLOR[0], COLOR[1], COLOR[2]))
    //     } else {
    //         Style::default()
    //     }
    // }

    // #################### REGISTRY ####################
    //

    /// Get the `TagRegistry`'s last modification time
    fn get_registry_mtime(&self) -> Result<SystemTime> {
        fs::metadata(self.registry.path.clone())
            .map(|m| m.modified().ok())?
            .ok_or_else(|| anyhow!("Unable to get tag registry modified time"))
    }

    /// Determine whether the `TagRegistry` has been modified since the screen
    /// was drawn
    pub(crate) fn changed_since(&mut self, prev: Option<SystemTime>) -> Result<bool> {
        if let Some(prev) = prev {
            let mtime = self.get_registry_mtime()?;
            if mtime > prev {
                Ok(true)
            } else {
                let now = SystemTime::now();
                let max_delta = Duration::from_secs(60);
                Ok(now.duration_since(prev)? > max_delta)
            }
        } else {
            Ok(true)
        }
    }

    /// Import the paths from the registry
    pub(crate) fn import_paths(&mut self) {
        let entries = self.get_full_tag_hash();
        let mut paths = vec![];

        for entry in entries {
            paths.push(entry.0);
        }

        self.registry_paths = paths;
    }

    fn path_by_id(&self, id: EntryId) -> Option<&EntryData> {
        self.registry.get_entry(id)

        // let paths = &self.registry_paths;
        // let m = tasks.iter().find(|t| *t.uuid() == uuid);
        // m.cloned()
    }

    fn path_idx_by_id(&self, id: EntryId) -> Option<usize> {
        let paths = &self.registry_paths;

        paths
            .iter()
            .position(|p| self.registry.find_entry(p).unwrap_or_default() == id)
    }

    // #################### COMPLETIONS ####################
    //

    /// Update items in the completion list
    pub(crate) fn update_completion_list(&mut self) {
        self.completion_list.clear();

        // let i = completion::get_word_under_cursor(self.command.as_str(),
        // self.command.pos()); let input =
        // self.command.as_str()[i..self.command.pos()].to_string();

        if self.mode == AppMode::Command {
            let app = Opts::into_app();

            for item in app.get_subcommands() {
                match item.to_string().as_str() {
                    "print-completions" | "ui" => {},
                    _ => {
                        // if input == item.to_string() {
                        //     self.completion_list.clear();
                        // }
                        self.completion_list.insert(format!("{}", item));
                    },
                }
            }
        }
    }

    /// Generate the completion list
    pub(crate) fn complist(&mut self) {
        let i = completion::get_word_under_cursor(self.command.as_str(), self.command.pos());
        let input = self.command.as_str()[i..self.command.pos()].to_string();
        #[allow(clippy::needless_collect)] // ???
        let full_cmd = self.command.as_str().split(' ').collect::<Vec<_>>();

        if self.mode == AppMode::Command {
            let app = Opts::into_app();

            // Length equaling one here represents an empty command prompt
            if full_cmd.len() == 1 {
                self.completion_list.clear();
                for item in app.get_subcommands() {
                    match item.to_string().as_str() {
                        "print-completions" | "ui" => {},
                        _ => {
                            // if input == item.to_string() {
                            //     self.completion_list.clear();
                            // }
                            self.completion_list.insert(format!("{}", item));
                        },
                    }
                }
                for item in app.get_arguments() {
                    match item.to_string().as_str() {
                        "--help" | "--version" | "--verbose" | "--ls-colors" => {},
                        _ => self.completion_list.insert(format!("{}", item)),
                    }
                }
            } else {
                self.completion_list
                    .insert(format!("len2: {}", full_cmd.len()));
                for item in app.get_subcommands() {
                    // if item.to_string() == input {
                    if self.command.as_str().contains(&item.to_string()) {
                        for flag in item.get_opts() {
                            self.completion_list.insert(format!("{}", flag));
                        }
                    }
                }
            }

            // if app.get_subcommands().any(|sub| sub.to_string() == input) {
            //     self.completion_list.clear();
            //     self.completion_list.insert(format!("sub: {}", input));
            // }
        }
    }

    /// Update input being fed into the completion list. This function will
    /// refresh the completion menu, narrowing down the matches or removing them
    /// completely if no match is found.
    pub(crate) fn update_completion_matching(&mut self) {
        if self.mode == AppMode::Command {
            let i = completion::get_word_under_cursor(self.command.as_str(), self.command.pos());
            let input = self.command.as_str()[i..self.command.pos()].to_string();

            // if input == item.to_string() {
            //     // self.completion_list.clear();
            //     for flag in item.get_arguments() {
            //         self.completion_list.insert(format!("{} zz", flag));
            //     }
            // },
            //
            self.completion_list.input(input);
        }
    }

    /// Check for the status of currently typed command
    pub(crate) fn check_command_status(&mut self) -> Result<()> {
        let i = completion::get_word_under_cursor(self.command.as_str(), self.command.pos());
        let input = self.command.as_str()[i..self.command.pos()].to_string();

        let full_cmd = self.command.as_str().split(' ').collect::<Vec<&str>>();
        self.completion_list
            .insert(format!("cmd: {}", self.command.as_str()));

        let cmd = process::Command::new("wutag")
            .args(full_cmd)
            .output()
            .expect("failed to test wutag command");

        if !cmd.status.success() {
            self.completion_list
                .insert("Fundamental error taking place".to_string());

            let patt = Regex::new(r"error")?;
            #[allow(clippy::needless_collect)]
            let output = String::from_utf8(cmd.stdout.clone())?
                .lines()
                .filter(|line| patt.is_match(line))
                .map(ToString::to_string)
                .collect::<Vec<String>>();

            self.completion_list.insert(format!(
                "output: {}",
                String::from_utf8(cmd.stdout)?
                    .lines()
                    .map(ToString::to_string)
                    .collect::<String>()
            ));

            if !output.is_empty() {
                self.completion_list
                    .insert("Your command contains an error".to_string());
            }
        }

        // TODO: switch to error mode

        Ok(())
    }

    // #################### ACTIONS ####################
    //

    pub(crate) fn tag_edit2(&mut self) -> Result<(), String> {
        if self.registry.entries.is_empty() {
            return Ok(());
        }
        let selected = self.current_selection;
        let id = self
            .registry
            .find_entry(&self.registry_paths[selected])
            .unwrap_or_default();
        let tags = self.registry.list_entry_tags(id).unwrap_or_default();

        let res = process::Command::new("wutag")
            .arg("-gr")
            .arg("view")
            .arg("-p")
            .arg(&self.registry_paths[selected].display().to_string())
            .spawn();

        let res = match res {
            Ok(child) => {
                let output = child.wait_with_output();
                match output {
                    Ok(output) =>
                        if output.status.success() {
                            String::from_utf8_lossy(&output.stdout);
                            String::from_utf8_lossy(&output.stderr);
                            Ok(())
                        } else {
                            Err(format!(
                                "viewing file {} failed. {}{}",
                                self.registry_paths[selected].display(),
                                String::from_utf8_lossy(&output.stdout),
                                String::from_utf8_lossy(&output.stderr),
                            ))
                        },
                    Err(err) => Err(format!(
                        "Cannot run view for {}. {}",
                        self.registry_paths[selected].display(),
                        err
                    )),
                }
            },
            _ => Err(format!(
                "Cannot start `view` for `{}`",
                self.registry_paths[selected].display()
            )),
        };

        self.current_selection_id = Some(id);
        self.current_selection_path = Some(self.registry_paths[selected].clone());

        res
    }

    /// Edit the current tag(s) in an editor
    pub(crate) fn tag_edit(&mut self) -> Result<()> {
        if self.registry.entries.is_empty() {
            return Ok(());
        }
        let selected = self.current_selection;
        let id = self
            .registry
            .find_entry(&self.registry_paths[selected])
            .unwrap_or_default();
        let tags = self.registry.list_entry_tags(id).unwrap_or_default();

        let viewargs = Opts::view_args(&self.registry_paths[selected].display().to_string());
        App::run(viewargs, &self.config).map_err(Error::Edit)?;

        self.current_selection_id = Some(id);
        self.current_selection_path = Some(self.registry_paths[selected].clone());

        Ok(())
    }
}

// Handle cursor movement of the command prompt (`LineBuffer`)
pub(crate) fn handle_movement(linebuffer: &mut LineBuffer, input: Key) {
    match input {
        Key::Ctrl('f') | Key::Right => {
            linebuffer.move_forward(1);
        },
        Key::Ctrl('b') | Key::Left => {
            linebuffer.move_backward(1);
        },
        Key::Ctrl('h') | Key::Backspace => {
            linebuffer.backspace(1);
        },
        Key::Ctrl('d') | Key::Delete => {
            linebuffer.delete(1);
        },
        Key::Ctrl('a') | Key::Home => {
            linebuffer.move_home();
        },
        Key::Ctrl('e') | Key::End => {
            linebuffer.move_end();
        },
        Key::Ctrl('k') => {
            linebuffer.kill_line();
        },
        Key::Ctrl('u') => {
            linebuffer.discard_line();
        },
        Key::Ctrl('w') | Key::AltBackspace | Key::CtrlBackspace => {
            linebuffer.delete_prev_word(Word::Emacs, 1);
        },
        Key::Alt('d') | Key::AltDelete | Key::CtrlDelete => {
            linebuffer.delete_word(At::AfterEnd, Word::Emacs, 1);
        },
        Key::Alt('f') => {
            linebuffer.move_to_next_word(At::AfterEnd, Word::Emacs, 1);
        },
        Key::Alt('b') => {
            linebuffer.move_to_prev_word(Word::Emacs, 1);
        },
        Key::Alt('t') => {
            linebuffer.transpose_words(1);
        },
        Key::Char(c) => {
            linebuffer.insert(c, 1);
        },
        _ => {},
    }
}
