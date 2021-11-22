//! Current app's context
#![allow(unused)]

use crate::registry::TagRegistry;
use std::{env, fmt, path::PathBuf};

/// Context of the current application (only used within the TUI)
#[derive(Debug, Clone)]
pub(crate) struct Context {
    current_dir:      PathBuf,
    current_registry: PathBuf,
}

// impl fmt::Display for Context {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", )
//     }
// }

impl Context {
    /// Create a new instance of `Context`
    pub(crate) fn new(registry: TagRegistry) -> Self {
        Self {
            current_dir:      env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            current_registry: registry.path,
        }
    }

    /// Access to current directory
    pub(crate) fn current_dir(self) -> PathBuf {
        self.current_dir
    }

    /// Access to current registry path
    pub(crate) fn current_registry(self) -> PathBuf {
        self.current_registry
    }

    /// Get information as a string
    pub(crate) fn get_info(self) -> String {
        format!(
            r#"
            Current directory: {}
            Current registry: {}
            "#,
            self.current_dir.display(),
            self.current_registry.display()
        )
    }
}
