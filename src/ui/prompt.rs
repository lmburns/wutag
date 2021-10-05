//! Prompt used within the TUI subcommand of this crate. This allows user input
//! to enter a command instead of just using keybindings

use super::command::Command;
use std::{
    cmp::Ordering,
    fmt,
    time::Instant,
};
