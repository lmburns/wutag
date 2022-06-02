//! Format functions

use crate::{registry::types::Tag, subcommand::App, xattr::tag_old::Tag as WTag};
use colored::{ColoredString, Colorize};
use lscolors::{LsColors, Style};
use std::path::Path;

// /// Format one style of an error message
// #[allow(dead_code)]
// pub(crate) fn err<E: Display>(err: E) -> String {
//     format!("{} {}", "ERROR:".red().bold(), format!("{}", err).white())
// }

/// Format an `OK` message
pub(crate) fn ok<S: AsRef<str>>(msg: S) -> String {
    format!("{} {}", "OK".green().bold(), msg.as_ref().white())
}

/// Format the colored/non-colored output of a [`Path`]
pub(crate) fn path<P: AsRef<Path>>(path: P, app: &App) -> String {
    // ls_colors implies forced coloring
    if app.ls_colors {
        LsColors::from_env()
            .unwrap_or_default()
            .style_for_path_components(path.as_ref())
            .fold(Vec::new(), |mut acc, (component, style)| {
                acc.push(
                    style
                        .map_or_else(|| ansi_term::Color::Blue.bold(), Style::to_ansi_term_style)
                        .paint(component.to_string_lossy())
                        .to_string(),
                );
                acc
            })
            .join("")
    } else {
        format!(
            "{}",
            path.as_ref().to_string_lossy().color(app.base_color).bold()
        )
    }
}

/// Return a path in a format that only shows components from the CWD and
/// any level of depth beneath that
pub(crate) fn local_path<P: AsRef<Path>>(path: P, app: &App) -> String {
    let mut replaced = app.base_dir.to_string_lossy().to_string();
    if !replaced.ends_with('/') {
        replaced.push('/');
    }

    let path = path
        .as_ref()
        .to_string_lossy()
        .to_string()
        .replace(replaced.as_str(), "");

    if app.ls_colors {
        LsColors::from_env()
            .unwrap_or_default()
            .style_for_path_components(path.as_ref())
            .fold(Vec::new(), |mut acc, (component, style)| {
                acc.push(
                    style
                        .map_or_else(|| ansi_term::Color::Blue.bold(), Style::to_ansi_term_style)
                        .paint(component.to_string_lossy())
                        .to_string(),
                );
                acc
            })
            .join("")
    } else {
        format!("{}", path.color(app.base_color).bold())
    }
}

/// Format the tag by coloring it the specified color
// XXX: Remove once ready
pub(crate) fn tag_old(tag: &WTag) -> ColoredString {
    tag.name().color(*tag.color()).bold()
}

/// Format the tag by coloring it the specified color
pub(crate) fn tag<S: AsRef<str>>(tag: &Tag, effects: &[S]) -> ColoredString {
    let mut s = tag.name().color(tag.color());
    for effect in effects {
        match effect.as_ref().to_ascii_lowercase().trim() {
            "underline" | "u" | "ul" => s = s.underline(),
            "italic" | "i" | "it" => s = s.italic(),
            "reverse" | "r" | "rev" => s = s.reversed(),
            "dimmed" | "d" | "dim" => s = s.dimmed(),
            "blink" | "bl" => s = s.blink(),
            "strikethrough" | "s" | "st" => s = s.strikethrough(),
            "none" | "n" => s = s.clear(),
            // Bold is the default
            _ => s = s.bold(),
        }
    }

    s
}

/// Return a local path with no color, i.e., one in which /home/user/... is not
/// used and it is relative to the current directory. The searching of the paths
/// does not go above the folder in which this command is read and only searches
/// recursively
pub(crate) fn raw_local_path<P: AsRef<Path>, A: AsRef<Path>>(path: P, local: A) -> String {
    let mut replaced = local.as_ref().to_string_lossy().to_string();
    if !replaced.ends_with('/') {
        replaced.push('/');
    }
    path.as_ref()
        .to_string_lossy()
        .to_string()
        .replace(replaced.as_str(), "")
}
