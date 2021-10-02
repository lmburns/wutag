#![allow(unused)]
#![allow(clippy::unused_self)]

// TODO: ListItem
// TODO: Dim selection

use anyhow::{anyhow, Context, Result};
use colored::{ColoredString, Colorize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    convert::TryInto,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use thiserror::Error;
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    terminal::Frame,
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};

use rustyline::{line_buffer::LineBuffer, At, Editor, Word};
use rustyline_derive::Helper;
use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;
use wutag_core::{
    color::{color_tui_from_fg_str, parse_color_tui, TuiColor},
    tag::Tag,
};

use super::{
    event::Key,
    style::TuiStyle,
    table::{Row, Table, TableSelection, TableState},
};

use crate::{
    config::Config,
    opt::{Command, Opts},
    registry::{EntryData, EntryId, TagRegistry},
    subcommand::App,
};

const MAX_LINE: usize = 4096;

/// Errors used within the UI module of this crate
#[derive(Debug, Error)]
pub(crate) enum Error {
    /// IO errors
    #[error("IO Error: {0}")]
    IOError(#[source] io::Error),
}

// Helper functions
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
    pub(crate) config:                 Config,
    pub(crate) current_context:        String,
    pub(crate) current_context_filter: String,
    pub(crate) current_selection:      usize,
    pub(crate) current_selection_id:   Option<EntryId>,
    pub(crate) current_selection_path: Option<PathBuf>,
    pub(crate) dirty:                  bool,
    pub(crate) error:                  String,
    pub(crate) file_details:           HashMap<EntryId, String>, // TODO: Show a stat command
    pub(crate) filter:                 LineBuffer,
    pub(crate) colored:             bool,
    pub(crate) last_export:            Option<SystemTime>,
    pub(crate) list_height:            u16,
    pub(crate) list_state:             ListState,
    pub(crate) marked:                 HashSet<EntryId>,
    pub(crate) mode:                   AppMode,
    pub(crate) preview_file:           bool, // TODO: Show a file preview
    pub(crate) preview_height:         u16,
    pub(crate) registry:               TagRegistry,
    pub(crate) should_quit:            bool,
    pub(crate) table_state:            TableState,
    pub(crate) terminal_height:        u16,
    pub(crate) terminal_width:         u16,
}

/// Mode that application is in
#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum AppMode {
    WutagList,
    WutagError,
    // WutagRemove,
}

impl UiApp {
    /// Create a new instance of the `UiApp`
    pub(crate) fn new(c: Config, reg: TagRegistry) -> Result<Self> {
        let (w, h) = crossterm::terminal::size()?;
        let mut state = ListState::default();
        if !reg.entries.is_empty() {
            state.select(Some(0));
        }

        let mut uiapp = Self {
            config:                 c,
            current_context:        String::from(""),
            current_context_filter: String::from(""),
            current_selection:      state.selected().unwrap_or(0),
            current_selection_id:   None,
            current_selection_path: None,
            dirty:                  false,
            error:                  String::from(""),
            file_details:           HashMap::new(),
            filter:                 LineBuffer::with_capacity(MAX_LINE),
            colored:             true,
            last_export:            None,
            list_height:            0,
            list_state:             state,
            marked:                 HashSet::new(),
            mode:                   AppMode::WutagList,
            preview_file:           false,
            preview_height:         0,
            registry:               reg,
            should_quit:            false,
            table_state:            TableState::default(),
            terminal_height:        h,
            terminal_width:         w,
        };

        Ok(uiapp)
    }

    /// Render the screen on the `Terminal` object
    pub(crate) fn render<B>(&mut self, app: &App, terminal: &mut Terminal<B>) -> Result<()>
    where
        B: Backend,
    {
        terminal
            .draw(|f| self.draw(app, f))
            .context("failed to draw terminal")?;
        Ok(())
    }

    /// Wrapper function that executes startup screen depending on the `AppMode`
    pub(crate) fn draw(&mut self, app: &App, f: &mut Frame<impl Backend>) {
        let rect = f.size();
        self.terminal_width = rect.width;
        self.terminal_height = rect.height;
        match self.mode {
            AppMode::WutagList | AppMode::WutagError => self.draw_tag(app, f),
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

    /// Draw the startup screen
    pub(crate) fn draw_tag(&mut self, app: &App, f: &mut Frame<impl Backend>) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
            .split(f.size());

        if self.preview_file {
            let split_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(chunks[0]);

            self.preview_height = split_layout[1].height;
            self.draw_table(app, f, split_layout[0]);
            // self.draw_preview(f, split_layout[1]);
        } else {
            let full_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(chunks[0]);

            self.preview_height = full_layout[0].height;
            self.draw_table(app, f, full_layout[0]);
        }

        let empty_path = PathBuf::new();
        let selected = self.current_selection;
        let file_id = if self.registry.entries.is_empty() {
            vec!["OKKKKK".to_string()]
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
            AppMode::WutagList => self.draw_command(
                f,
                chunks[1],
                self.filter.as_str(),
                "Filter Tags".to_string(),
                self.get_position(&self.filter),
                false,
            ),
            AppMode::WutagError => self.draw_command(
                f,
                chunks[1],
                self.error.as_str(),
                Span::styled("Error", Style::default().add_modifier(Modifier::BOLD)),
                0,
                false,
            ),
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
                    .border_style(Style::default().fg(Color::Red))
                    .title(title.into()),
            )
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .scroll((0, ((position + 3) as u16).saturating_sub(rect.width)));
        f.render_widget(p, rect);
    }

    /// Draw the tag table
    fn draw_table(&mut self, app: &App, f: &mut Frame<impl Backend>, rect: Rect) {
        let entries = self.get_full_tag_hash();
        let headers = vec!["Filename", "Tag(s)"]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        if entries.is_empty() {
            let mut style = Style::default();
            match self.mode {
                AppMode::WutagList => style = style.add_modifier(Modifier::BOLD),
                AppMode::WutagError => style = style.add_modifier(Modifier::DIM),
            }

            let mut title = vec![
                Span::styled("Overview", style),
                Span::from("|"),
                Span::styled("Preview", Style::default().add_modifier(Modifier::DIM)),
            ];

            if !self.current_context.is_empty() {
                let context_style = Style::default();
                context_style.add_modifier(Modifier::ITALIC);
                title.insert(title.len(), Span::from(" ("));
                title.insert(
                    title.len(),
                    Span::styled(&self.current_context, context_style),
                );
                title.insert(title.len(), Span::from(")"));
            }

            f.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Spans::from(title)),
                rect,
            );
            return;
        }

        let maximum_column_width = rect.width;

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

        let widths = self.calculate_widths(&entries_name, &headers, maximum_column_width);

        // for (idx, header) in headers.iter().enumerate() {
        //     if header == "Tag(s)" {
        //         self.tag_description_width = widths[idx] - 1;
        //         break;
        //     }
        // }

        // let selected = self.current_selection;
        let header = headers.iter();
        let mut rows = vec![];
        let mut hl_style = Style::default();

        for (idx, entry) in entries_name.iter().enumerate() {
            rows.push(Row::new(vec![
                Text::from(Spans::from(vec![Span::styled(
                    entry[0].clone(),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )])),
                self.styled_text_for_tags(entry),
            ]));

            // Spans::from(vec![Span::styled(line.to_string(),
            // highlight_style)])

            // let style = Style::default();
            // let mut mods = Modifier::empty();
            // if idx == self.selected() {
            //     hl_style = style;
            //     if self.config.ui.selection_bold {
            //         // hl_style = hl_style.add_modifier(Modifier::BOLD);
            //         mods |= Modifier::BOLD;
            //     }
            //     if self.config.ui.selection_italic {
            //         // hl_style = hl_style.add_modifier(Modifier::ITALIC);
            //         mods |= Modifier::ITALIC;
            //     }
            //     if self.config.ui.selection_dim {
            //         // hl_style = hl_style.add_modifier(Modifier::DIM);
            //         mods |= Modifier::DIM;
            //     }
            //     if self.config.ui.selection_blink {
            //         // hl_style =
            // hl_style.add_modifier(Modifier::SLOW_BLINK);
            //         mods |= Modifier::SLOW_BLINK;
            //     }
            // }
            // hl_style = hl_style.add_modifier(mods);

            // rows.push(Row::StyledData(entry.iter(), style));
        }

        let constraints: Vec<Constraint> = widths
            .iter()
            .map(|i| Constraint::Length((*i).try_into().unwrap_or(maximum_column_width)))
            .collect();

        let mut style = Style::default();

        match self.mode {
            AppMode::WutagList => style = style.add_modifier(Modifier::BOLD),
            AppMode::WutagError => style = style.add_modifier(Modifier::DIM),
        }

        let mut title = vec![
            // Span::styled("Tag", style),
            // Span::from("  |  "),
            Span::styled(
                "Wutag",
                if self.colored {
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Rgb(239, 29, 85))
                } else {
                    Style::default()
                },
            ),
        ];

        if !self.current_context.is_empty() {
            let context_style = Style::default();
            context_style.add_modifier(Modifier::BOLD);
            title.insert(title.len(), Span::from(" ("));
            title.insert(
                title.len(),
                Span::styled(&self.current_context, context_style),
            );
            title.insert(title.len(), Span::from(")"));
        }

        let table = Table::new(header, rows)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Spans::from(title))
                    .title_alignment(Alignment::Left),
            )
            .header_style(
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_style(hl_style)
            .highlight_symbol(&self.config.ui.selection_indicator)
            .mark_symbol(&self.config.ui.mark_indicator)
            .unmark_symbol(&self.config.ui.unmark_indicator)
            .widths(&constraints);

        f.render_stateful_widget(table, rect, &mut self.table_state);
    }

    /// Get the rows of `Tag`s' to build the `Table`
    fn get_full_tag_hash(&self) -> BTreeMap<PathBuf, Vec<Tag>> {
        self.registry.list_all_paths_and_tags()
    }

    /// Get the rows of `Tag`s' to build the `Table` with tags as strings
    fn get_full_tag_hash_str(&mut self) -> BTreeMap<PathBuf, Vec<String>> {
        self.registry.list_all_paths_and_tags_as_strings()
    }

    /// Draw a file preview
    fn draw_preview(&mut self, f: &mut Frame<impl Backend>, rect: Rect) {
        todo!();
    }

    pub(crate) fn toggle_mark(&mut self) {
        if !self.registry.tags.is_empty() {
            let selected = self.current_selection;
            // let id = self.registry.tags.get(selected);
            // let task_uuid = *self.tasks[selected].uuid();
            //
            // if !self.marked.insert(task_uuid) {
            //     self.marked.remove(&task_uuid);
            // }
        }
    }

    // pub(crate) fn toggle_mark_all(&mut self) {
    //     for task in &self.tasks {
    //         if !self.marked.insert(*task.uuid()) {
    //             self.marked.remove(task.uuid());
    //         }
    //     }
    // }

    // } else if input == self.config.keys.select {
    //     self.task_table_state.multiple_selection();
    //     self.toggle_mark();
    // } else if input == self.config.keys.select_all {
    //     self.table_state.multiple_selection();
    //     self.toggle_mark_all();

    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn handle_input(&mut self, input: Key) -> Result<()> {
        match self.mode {
            AppMode::WutagList =>
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
                },
            AppMode::WutagError => self.mode = AppMode::WutagList,
        }

        self.update_table_state();
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn update(&mut self, force: bool) -> Result<()> {
        if force || self.changed_since(self.last_export).unwrap_or(true) {
            self.last_export = Some(SystemTime::now());
            //     self.task_report_table.export_headers(None, &self.report)?;
            //     let _ = self.export_tasks();
            self.dirty = false;
        }

        //     self.export_contexts()?;
        //     self.update_tags();
        //     self.task_details.clear();
        //     self.save_history()?;
        // }

        self.cursor_fix();
        // self.update_task_table_state();
        // if self.task_report_show_info {
        //     task::block_on(self.update_task_details())?;
        // }
        // self.selection_fix();
        Ok(())
    }

    /// Update the state the table is in
    pub(crate) fn update_table_state(&mut self) {
        self.table_state.select(Some(self.current_selection));

        // for uuid in self.marked.clone() {
        //     if self.tag_by_uuid(uuid).is_none() {
        //         self.marked.remove(&uuid);
        //     }
        // }

        if self.marked.is_empty() {
            self.table_state.single_selection();
        }

        self.table_state.clear();

        // for uuid in &self.marked {
        //     self.table_state.mark(self.tag_index_by_uuid(*uuid));
        // }
    }

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

    // /// Find a tag's `EntryData` by its' `Uuid`
    // fn tag_by_uuid(&self, uuid: Uuid) -> Option<EntryData> {
    //     self.registry
    //         .list_entries()
    //         .find(|t| *t.uuid() == uuid)
    //         .cloned()
    // }
    //
    // fn tag_index_by_uuid(&self, uuid: Uuid) -> Option<usize> {
    //     self.registry.list_entries().position(|t| *t.uuid() == uuid)
    // }

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

            if let Some(color) = color_tui_from_fg_str(&tag.color().to_fg_str()) {
                style = style.fg(color);
            }

            modifiers |= Modifier::BOLD;
            style = style.add_modifier(modifiers);

            colored.push(Span::styled(tag.clone().name().to_string(), style));
        }

        row.push(Spans::from(colored));

        Text::from(row)
    }

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
}
