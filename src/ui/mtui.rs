//! `mtui` = My Tui
//! Meta structure that holds:
//!     * the terminal where the user activity is occuring
//!     * the event signals
//!     * an internal state on whether the application is paused
//!         * This is used for leaving the TUI for the editor

// Credit: idea and outline came from `orhun/gpg-tui`
//  * Using his work to help me learn how to code a TUI

use anyhow::{Context, Result};
use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, sync::atomic::Ordering};
use tui::{backend::Backend, Terminal};

use super::{event::EventHandler, ui_app::UiApp};
use crate::subcommand::App;

/// The meta-tui wrapper, which sets up the user interface
#[derive(Debug)]
pub(crate) struct Tui<B: Backend> {
    /// Terminal interface
    terminal:          Terminal<B>,
    /// Event handler
    pub(crate) events: EventHandler,
    /// Paused state of interface
    pub(crate) paused: bool,
}

impl<B: Backend> Tui<B> {
    /// Creates a new instance of `Tui`
    pub(crate) fn new(terminal: Terminal<B>, events: EventHandler) -> Self {
        Self {
            terminal,
            events,
            paused: false,
        }
    }

    /// Enter TUI mode
    pub(crate) fn enter_tui_mode(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        self.terminal.hide_cursor()?;
        // execute!(stdout, Clear(ClearType::All)).unwrap();
        self.terminal.clear()?;
        self.terminal.resize(self.terminal.size()?)?;
        Ok(())
    }

    /// Leave TUI mode
    pub(crate) fn leave_tui_mode(&mut self) -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Enable mouse capturing
    #[allow(clippy::unused_self)]
    pub(crate) fn enable_mouse_capture(&mut self) -> Result<()> {
        Ok(execute!(io::stdout(), EnableMouseCapture)?)
    }

    /// Disables the mouse capture.
    #[allow(clippy::unused_self)]
    pub(crate) fn disable_mouse_capture(&mut self) -> Result<()> {
        Ok(execute!(io::stdout(), DisableMouseCapture)?)
    }

    /// Hide the cursor
    #[allow(dead_code)]
    #[allow(clippy::unused_self)]
    pub(crate) fn hide_cursor(&self) -> Result<()> {
        // self.terminal.show_cursor()?
        Ok(execute!(io::stdout(), Hide)?)
    }

    /// Show the cursor
    #[allow(dead_code)]
    #[allow(clippy::unused_self)]
    pub(crate) fn show_cursor(&self) -> Result<()> {
        // self.terminal.hide_cursor()?
        Ok(execute!(io::stdout(), Show)?)
    }

    /// Toggle a paused mode when exiting the terminal to edit tags in an
    /// `$EDITOR`
    pub(crate) fn toggle_pause(&mut self) -> Result<()> {
        self.paused = !self.paused;
        if self.paused {
            self.leave_tui_mode()?;
        } else {
            self.enter_tui_mode()?;
        }

        self.events
            .key_input_disabled
            .store(self.paused, Ordering::Relaxed);
        Ok(())
    }

    /// Render the TUI envrionment
    pub(crate) fn render(&mut self, app: &App, uiapp: &mut UiApp) -> Result<()> {
        self.terminal
            .draw(|f| uiapp.draw(app, f))
            .context("failed to draw terminal")?;
        Ok(())
    }
}
