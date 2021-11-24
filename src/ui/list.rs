//! Provides a more detailed list to preview the keybindings in the `Help`
//! (Other) tab of the TUI

// Credit: idea and outline came from `orhun/gpg-tui`
//  * Using his work to help me learn how to code a TUI

use tui::widgets::ListState;

/// List widget with an internally controlled state
#[derive(Debug, Default, Clone)]
pub(crate) struct StatefulList<T> {
    /// Items that make up the `StatefulList`
    pub(crate) items: Vec<T>,
    /// Modifiable state
    pub(crate) state: ListState,
}

impl<T> StatefulList<T> {
    /// Build an instance of `StatefulList`
    pub(crate) fn new(items: Vec<T>, state: ListState) -> StatefulList<T> {
        Self { items, state }
    }

    /// Build a new `StatefulList` with specified items
    pub(crate) fn with_items(items: Vec<T>) -> StatefulList<T> {
        Self::new(items, ListState::default())
    }

    /// Returns the selected item
    pub(crate) fn selected(&self) -> Option<&T> {
        self.items.get(self.state.selected()?)
    }

    /// Selects the next item in the list
    pub(crate) fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) =>
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                },
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Selects the previous item in the list
    pub(crate) fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) =>
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                },
            None => 0,
        };
        self.state.select(Some(i));
    }
}
