//! History of commands typed within the command prompt. Using `rustyline`
//! instead of another plain frame within the TUI offers tab completion and
//! history scroll-back/scroll-forward functionality between sessions

use crate::config;
use anyhow::{anyhow, Context, Result};
use rustyline::{
    error::ReadlineError,
    history::{History, SearchDirection},
};
use std::{
    borrow::Cow,
    env, fmt,
    fs::{self, File},
    path::{Path, PathBuf},
};

/// Context of `super::UiApp`'s history
pub(crate) struct HistoryContext {
    /// User command history
    history:       History,
    history_index: usize,
    /// Location of configuration file
    config:        PathBuf,
}

// Used to debug the main struct of the `super::ui_app::UiApp`
impl fmt::Debug for HistoryContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HistoryContext")
            .field("history_index", &self.history_index)
            .field("config", &self.config)
            .finish()
    }
}

impl HistoryContext {
    /// Create a new instance of `HistoryContext
    pub(crate) fn new<P: AsRef<str>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let expanded = PathBuf::from(
            shellexpand::full(path)
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
        );

        let parent = expanded
            .parent()
            .context("failed to get parent of history path")?;

        if !parent.exists() {
            fs::create_dir_all(parent).context("unable to create history file directory")?;
        }

        Ok(Self {
            history:       History::new(),
            history_index: 0,
            config:        expanded,
        })
    }

    /// Call to access config field of `HistoryContext`
    pub(crate) fn config(&self) -> &PathBuf {
        &self.config
    }

    /// Either load the history file or save the history file if it is the first
    /// time being ran
    pub(crate) fn load(&mut self) -> Result<()> {
        if self.config.exists() {
            self.history.load(&self.config)?;
        } else {
            self.history.save(&self.config)?;
        }

        Ok(())
    }

    /// Write a history file
    pub(crate) fn write(&mut self) -> Result<()> {
        self.history.save(&self.config)?;
        Ok(())
    }

    /// Call to access history field of `HistoryContext`
    pub(crate) fn history(&self) -> &History {
        &self.history
    }

    /// Call to access history index field of `HistoryContext`
    pub(crate) fn history_index(&self) -> usize {
        self.history_index
    }

    /// Add an item to the history file
    pub(crate) fn add(&mut self, buffer: &str) {
        if self.history.add(buffer) {
            self.history_index = self.history.len() - 1;
        }
    }

    /// Access last item in history
    pub(crate) fn last(&mut self) {
        self.history_index = self.history.len().saturating_sub(1);
    }

    /// Get number of items in history file
    pub(crate) fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Search through history using custom keybindings
    pub(crate) fn history_search(
        &mut self,
        buffer: &str,
        direction: SearchDirection,
    ) -> Option<String> {
        if self.history.is_empty() {
            return None;
        }

        if (self.history_index == self.history.len().saturating_sub(1)
            && direction == SearchDirection::Forward)
            || (self.history_index == 0 && direction == SearchDirection::Reverse)
        {
            return None;
        }

        let history_index = match direction {
            SearchDirection::Forward => self.history_index + 1,
            SearchDirection::Reverse => self.history_index - 1,
        };

        if let Some(item) = self.history.starts_with(buffer, history_index, direction) {
            // item.entry
            self.history_index = item.idx;
            Some(self.history.get(item.idx).unwrap().clone())
        } else if buffer.is_empty() {
            self.history_index = history_index;
            Some(self.history.get(history_index).unwrap().clone())
        } else {
            None
        }
    }
}
