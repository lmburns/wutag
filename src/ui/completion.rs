//! Completion menu popup within the command prompt

// Credit: idea and outline came from `kdheepak/taskwarrior-tui`
//  * Using their work to help me learn how to code a TUI

use std::{error::Error, fmt, io};
use tui::{
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};

use rustyline as rl;
use rustyline::{
    completion::{Completer, FilenameCompleter, Pair},
    error::ReadlineError,
    highlight::{Highlighter, MatchingBracketHighlighter},
    hint::Hinter,
    history::History,
    line_buffer::LineBuffer,
    Context,
};
use rustyline_derive::Helper;

use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

// TODO: add help menu here as well?

/// Get the current word the user is typing to start the completion menu
pub(crate) fn get_word_under_cursor(line: &str, cursor_pos: usize) -> usize {
    let mut chars = line[..cursor_pos].chars();
    let mut res = cursor_pos;
    while let Some(c) = chars.next_back() {
        if c == ' ' {
            break;
        }
        res -= c.len_utf8();
    }

    res
}

/// Representation of completions options and the completer
pub(crate) struct CompletionHelper {
    pub(crate) completer:  FilenameCompleter,
    pub(crate) candidates: Vec<String>,
}

impl Completer for CompletionHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        word: &str,
        pos: usize,
        ctx_: &Context,
    ) -> rl::Result<(usize, Vec<Self::Candidate>)> {
        let candidates = self
            .candidates
            .iter()
            .filter_map(|cand| {
                if cand.starts_with(&word[..pos]) {
                    // Options such as --dir <dir>
                    let replacement = if cand.contains(' ') {
                        cand[pos..].split(' ').collect::<Vec<&str>>()[0].to_string()
                    } else {
                        cand[pos..].to_string()
                    };
                    Some(Pair {
                        display: cand.clone(),
                        replacement,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<Pair>>();

        Ok((pos, candidates))
    }
}

/// Extension of `Self::CompletionHelper`, including user input, position, and
/// the state of the list
pub(crate) struct CompletionList {
    pub(crate) input:  String,
    pub(crate) pos:    usize,
    pub(crate) state:  ListState,
    pub(crate) helper: CompletionHelper,
}

// Used to debug the main struct of the `super::ui_app::UiApp`
impl fmt::Debug for CompletionList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompletionList")
            .field("input", &self.input)
            .field("pos", &self.pos)
            .field("state", &self.state)
            .finish()
    }
}

impl CompletionList {
    /// Create a new instance of `CompletionList`
    pub(crate) fn new() -> Self {
        Self {
            input:  String::new(),
            pos:    0,
            state:  ListState::default(),
            helper: CompletionHelper {
                candidates: vec![],
                completer:  FilenameCompleter::new(),
            },
        }
    }

    /// Create an instance of `CompletionList` with candidates
    pub(crate) fn with_items(items: Vec<String>) -> Self {
        let mut candidates = vec![];
        for item in items {
            if !candidates.contains(&item) {
                candidates.push(item);
            }
        }
        Self {
            input:  String::new(),
            pos:    0,
            state:  ListState::default(),
            helper: CompletionHelper {
                candidates,
                completer: FilenameCompleter::new(),
            },
        }
    }

    /// Return a vector of completion candidates
    pub(crate) fn candidates(&self) -> Vec<Pair> {
        let hist = History::new();
        let ctx = Context::new(&hist);
        let (pos, candidates) = self.helper.complete(&self.input, self.pos, &ctx).unwrap();
        candidates
    }

    /// Set the input of the completion list
    pub(crate) fn input(&mut self, input: String) {
        self.input = input;
        self.pos = self.input.len();
    }

    /// Insert an item into the completer
    pub(crate) fn insert(&mut self, item: String) {
        if !self.helper.candidates.contains(&item) {
            self.helper.candidates.push(item);
        }

        self.helper.candidates.sort();
    }

    /// Get the next item in the completion list
    // TODO: fix a crash here while typing
    pub(crate) fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) =>
                if i >= self.candidates().len() - 1 {
                    0
                } else {
                    i + 1
                },
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Get the previous item in the completion list
    pub(crate) fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) =>
                if i == 0 {
                    self.candidates().len() - 1
                } else {
                    i - 1
                },
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Unselect the item in the completion list
    pub(crate) fn unselect(&mut self) {
        self.state.select(None);
    }

    /// Clear the completion menu
    pub(crate) fn clear(&mut self) {
        self.helper.candidates.clear();
        self.state.select(None);
    }

    /// Return the length of the candidates
    pub(crate) fn len(&self) -> usize {
        self.candidates().len()
    }

    /// Get a candidate from the completion list
    pub(crate) fn get(&self, i: usize) -> Option<String> {
        let candidates = self.candidates();
        if i < candidates.len() {
            Some(candidates[i].replacement.clone())
        } else {
            None
        }
    }

    /// Return the selected item in the completion list
    pub(crate) fn selected(&self) -> Option<String> {
        self.state.selected().and_then(|i| self.get(i))
    }

    /// Test whether there are any candidates
    pub(crate) fn is_empty(&self) -> bool {
        self.candidates().is_empty()
    }

    /// Return the max width of the completion menu
    pub(crate) fn max_width(&self) -> Option<usize> {
        self.candidates()
            .iter()
            .map(|p| p.display.width() + 4)
            .max()
    }
}
