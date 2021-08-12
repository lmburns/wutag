use colored::{ColoredString, Colorize};
use globwalk::{DirEntry, GlobWalker, GlobWalkerBuilder};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use lscolors::{LsColors, Style};

use crate::DEFAULT_MAX_DEPTH;
use anyhow::{Context, Result};
use wutag_core::tag::Tag;

pub fn fmt_err<E: Display>(err: E) -> String {
    format!("{} {}", "ERROR".red().bold(), format!("{}", err).white())
}

pub fn fmt_ok<S: AsRef<str>>(msg: S) -> String {
    format!("{} {}", "OK".green().bold(), msg.as_ref().white())
}

pub fn fmt_path<P: AsRef<Path>>(
    path: P,
    ls_colors: bool,
    color_when: &str,
) -> String {
    // ansi_term always prints colors
    // colored will removed colors when piping automatically
    // Therefore, ls_colors implies forced coloring
    if ls_colors {
        let lscolors = LsColors::from_env().unwrap_or_default();

        let style = lscolors.style_for_path(path.as_ref());
        let style = style
            .map(Style::to_ansi_term_style)
            .unwrap_or_else(|| ansi_term::Color::Blue.bold());

        format!("{}", style.paint(path.as_ref().display().to_string()))
    } else if color_when == "always" {
            format!(
                "{}",
                ansi_term::Color::Blue.bold()
                    .paint(path.as_ref().display().to_string())
            )
    } else {
        format!(
            "{}",
            path.as_ref().display().to_string().bold().blue()
        )
    }
}

/// Format a local path (i.e., remove path components before files local to directory)
pub fn fmt_local_path<P: AsRef<Path>>(
    path: P,
    local_path: P,
    ls_colors: bool,
    color_when: &str
) -> String {
    // let painted = |to_paint

    let mut replaced = local_path.as_ref().display().to_string();
    if !replaced.ends_with('/') {
        replaced.push('/');
    }

    if ls_colors {
        let lscolors = LsColors::from_env().unwrap_or_default();

        let style = lscolors.style_for_path(path.as_ref());
        let style = style
            .map(Style::to_ansi_term_style)
            .unwrap_or_else(|| ansi_term::Color::Blue.bold());

        format!("{}",
            style.paint(
                path.as_ref().display().to_string()
                    .replace(replaced.as_str(), "")
            )
        )
    } else if color_when == "always" {
            format!(
                "{}",
                ansi_term::Color::Blue.bold().paint(
                    path.as_ref().display().to_string()
                        .replace(replaced.as_str(), "")
                    )
            )
    } else {
        format!("{}",
            path.as_ref().display().to_string()
            .replace(replaced.as_str(), "")
            .bold().blue()
        )
    }
}

pub fn fmt_tag(tag: &Tag) -> ColoredString {
    tag.name().color(*tag.color()).bold()
}

pub fn raw_local_path<P: AsRef<Path>>(path: P, local: P) -> String {
    let mut replaced = local.as_ref().display().to_string();
    if !replaced.ends_with('/') {
        replaced.push('/');
    }
    path.as_ref().display().to_string()
        .replace(replaced.as_str(), "")
}

/// Determine whether file (path) contains path and if so, return true
pub fn contained_path<P: AsRef<Path>>(file: P, path: P) -> bool {
    file.as_ref().display().to_string()
        .contains(path.as_ref().to_str().unwrap())
}

/// Returns a GlobWalker instance with base path set to `base_path` and pattern to `pattern`. If
/// max_depth is specified the GlobWalker will have it's max depth set to its value, otherwise max
/// depth will be [DEFAULT_MAX_DEPTH](DEFAULT_MAX_DEPTH).
pub fn glob_walker<S>(
    dir: S,
    pattern: S,
    max_depth: Option<usize>,
    case_insensitive: bool
) -> Result<GlobWalker>
where
    S: AsRef<str>,
{
    let mut builder = GlobWalkerBuilder::new(dir.as_ref(), pattern.as_ref());

    if let Some(max_depth) = max_depth {
        builder = builder.max_depth(max_depth);
    } else {
        builder = builder.max_depth(DEFAULT_MAX_DEPTH);
    }

    if case_insensitive {
        builder = builder.case_insensitive(true);
    } else {
        builder = builder.case_insensitive(false);
    }
    builder.build().context("invalid path")
}

/// Utility function that executes the function `f` on all directory entries that are Ok, by
/// default ignores all errors.
pub fn glob_ok<P, F>(
    pattern: &str,
    base_path: P,
    max_depth: Option<usize>,
    case_insensitive: bool,
    mut f: F
) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&DirEntry),
{
    let base_path = base_path.as_ref().to_string_lossy().to_string();

    for entry in glob_walker(base_path.as_str(), pattern, max_depth, case_insensitive)?.flatten() {
        f(&entry);
    }

    Ok(())
}

/// Helper function to get different directories for `macOS` specifically
/// Example: `dirs::cache_dir()` returns `$HOME/Library/Caches`, when this will return `$HOME/.cache`
/// This can and is used to respect `XDG` defaults for `macOS`
pub fn macos_dirs(dir_func: Option<PathBuf>, joined: &str) -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    if std::env::consts::OS == "macos" {
        Ok(PathBuf::from(env!("HOME")).join(joined))
    } else {
        Ok(dir_func.context(format!("Invalid {} directory", joined))?)
    }
}
