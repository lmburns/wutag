#![allow(unused)]

use anyhow::{anyhow, Result};
use std::{
    collections::{HashMap, HashSet},
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
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal,
};

use crate::{
    config::Config,
    opt::{Command, Opts},
    registry::{EntryData, TagRegistry},
    subcommand::App,
};
use rustyline::{line_buffer::LineBuffer, At, Editor, Word};
use rustyline_derive::Helper;
use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;
use uuid::Uuid;

use super::{
    event::Key,
    table::{TableSelection, TableState},
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
    pub(crate) registry:               TagRegistry,
    pub(crate) should_quit:            bool,
    pub(crate) dirty:                  bool,
    pub(crate) terminal_width:         u16,
    pub(crate) terminal_height:        u16,
    pub(crate) table_state:            TableState,
    pub(crate) mode:                   AppMode,
    pub(crate) current_selection:      usize,
    pub(crate) current_selection_uuid: Option<Uuid>,
    pub(crate) current_selection_id:   Option<u64>,
    pub(crate) current_selection_path: Option<PathBuf>,
    pub(crate) list_state:             ListState,
    pub(crate) file_details:           HashMap<Uuid, String>, // TODO: Show a stat command
    pub(crate) preview_file:           bool,                  // TODO: Show a file preview
    pub(crate) preview_height:         u16,
    pub(crate) marked:                 HashSet<Uuid>,
    pub(crate) filter:                 LineBuffer,
    pub(crate) last_export:            Option<SystemTime>,
}

/// Mode that application is in
#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum AppMode {
    WutagList,
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
            registry:               reg,
            table_state:            TableState::default(),
            should_quit:            false,
            dirty:                  false,
            terminal_width:         w,
            terminal_height:        h,
            mode:                   AppMode::WutagList,
            current_selection:      0,
            current_selection_uuid: None,
            current_selection_path: None,
            current_selection_id:   None, // state.selected
            list_state:             state,
            file_details:           HashMap::new(),
            preview_file:           false,
            preview_height:         0,
            marked:                 HashSet::new(),
            filter:                 LineBuffer::with_capacity(MAX_LINE),
            last_export:            None,
        };

        Ok(uiapp)
    }

    /// Render the screen on the `Terminal` object
    pub(crate) fn render<B>(&mut self, terminal: &mut Terminal<B>) -> Result<()>
    where
        B: Backend,
    {
        terminal.draw(|f| self.draw(f))?;
        Ok(())
    }

    /// Wrapper function that executes startup screen depending on the `AppMode`
    pub(crate) fn draw(&mut self, f: &mut Frame<impl Backend>) {
        let rect = f.size();
        self.terminal_width = rect.width;
        self.terminal_height = rect.height;
        match self.mode {
            AppMode::WutagList => self.draw_tag(f),
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
    #[allow(clippy::unused_self)]
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
    #[allow(clippy::unused_self)]
    pub(crate) fn draw_tag(&mut self, f: &mut Frame<impl Backend>) {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
            .split(f.size());

        if self.preview_file {
            let split_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(rects[0]);

            self.preview_height = split_layout[0].height;
            self.draw_preview(f, split_layout[0]);
            // self.draw_task_details(f, split_layout[1]);
        } else {
            let full_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(rects[0]);

            self.preview_height = full_layout[0].height;
            self.draw_preview(f, full_layout[0]);
        }

        let selected = self.current_selection;
        let file_id = if self.registry.tags.is_empty() {
            vec![PathBuf::new().display().to_string()]
        } else {
            match self.table_state.mode() {
                TableSelection::Single => {
                    vec![self
                        .registry
                        .get_entry(selected)
                        .and_then(|c| Some(c.path()))
                        .unwrap_or_else(|| &PathBuf::new())
                        .display()
                        .to_string()]
                },
                TableSelection::Multiple => {
                    let mut tag_uuids = vec![];
                    for uuid in self.marked.iter() {
                        if let Some(entry) = self.tag_by_uuid(*uuid) {
                            tag_uuids.push(self.registry.add_or_update_entry(entry).to_string())
                        }
                    }
                    tag_uuids
                },
            }
        };
        match self.mode {
            AppMode::WutagList => self.draw_command(
                f,
                rects[1],
                self.filter.as_str(),
                "Filter Tags".to_string(),
                self.get_position(&self.filter),
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
                    .title(title.into()),
            )
            .scroll((0, ((position + 3) as u16).saturating_sub(rect.width)));
        f.render_widget(p, rect);
    }

    fn draw_preview(&mut self, f: &mut Frame<impl Backend>, rect: Rect) {
        todo!();
    }

    //     let (tasks, headers) = self.get_task_report();
    //     if tasks.is_empty() {
    //         let mut style = Style::default();
    //         match self.mode {
    //             AppMode::WutagList => style = style.add_modifier(Modifier::BOLD),
    //             _ => style = style.add_modifier(Modifier::DIM),
    //         }
    //
    //         let mut title = vec![
    //             Span::styled("Tags", style),
    //             Span::from("|"),
    //             Span::styled("Preview",
    // Style::default().add_modifier(Modifier::DIM)),         ];
    //     }
    // }

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

    pub(crate) fn handle_input(&mut self, input: Key) -> Result<()> {
        match self.mode {
            AppMode::WutagList =>
                if input == self.config.keys.quit || input == Key::Ctrl('c') {
                    self.should_quit = true;
                } else if input == Key::Esc {
                    self.marked.clear()
                    // } else if input == self.config.keys.select {
                    //     self.task_table_state.multiple_selection();
                    //     self.toggle_mark();
                    // } else if input == self.config.keys.select_all {
                    //     self.table_state.multiple_selection();
                    //     self.toggle_mark_all();
                },
        }

        Ok(())
    }

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

    /// Get the `TagRegistry`'s last modification time
    fn get_registry_mtime(&self) -> Result<SystemTime> {
        fs::metadata(self.registry.path)
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

        let i = {
            if self.current_selection == 0 {
                if self.config.ui.report_looping {
                    self.registry.tags.len() - 1
                } else {
                    0
                }
            } else {
                self.current_selection - 1
            }
        };

        self.current_selection = i;
        // self.current_selection_id = None;
        self.current_selection_path = None;
        self.current_selection_uuid = None;
    }

    /// Fix cursor position under any errors that may arise
    pub(crate) fn cursor_fix(&mut self) {
        while !self.registry.tags.is_empty() && self.current_selection >= self.registry.tags.len() {
            self.previous_report();
        }
    }

    /// Fix selection of any errors that may arrise
    pub(crate) fn selection_fix(&mut self) {
        if let (Some(t), Some(id)) = (self.tag_current(), self.current_selection_id) {
            // if t.id() != Some(id) {
            //     if let Some(i) = self.tag_index_by_id(id) {
            //         self.current_selection = i;
            //         self.current_selection_id = None;
            //     }
            // }
        }

        // if let (Some(t), Some(uuid)) = (self.tag_current(),
        // self.current_selection_uuid) {     if t.uuid() != &uuid {
        //         if let Some(i) = self.task_index_by_uuid(uuid) {
        //             self.current_selection = i;
        //             self.current_selection_uuid = None;
        //         }
        //     }
        // }
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

    /// Current selections `Uuid`
    pub(crate) fn selected_task_uuids(&self) -> Vec<Uuid> {
        let selected = match self.table_state.mode() {
            TableSelection::Single => vec![self.current_selection],
            TableSelection::Multiple => self.table_state.marked().cloned().collect::<Vec<usize>>(),
        };

        let mut uuids = vec![];

        for s in selected {
            // let id = self.tasks[s].id().unwrap_or_default();
            let uuid = *self.registry.entries[&s].uuid();
            uuids.push(uuid);
        }

        uuids
    }

    /// Find a tag by an id
    fn tag_by_id(&self, id: u64) -> Option<EntryData> {
        todo!();
    }

    /// Find a tag's `EntryData` by its' `Uuid`
    fn tag_by_uuid(&self, uuid: Uuid) -> Option<EntryData> {
        self.registry
            .list_entries()
            .find(|t| *t.uuid() == uuid)
            .cloned()
    }

    fn tag_index_by_id(&self, id: u64) -> Option<usize> {
        todo!();
    }

    fn tag_index_by_uuid(&self, uuid: Uuid) -> Option<usize> {
        self.registry.list_entries().position(|t| *t.uuid() == uuid)
    }
}
