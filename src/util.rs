use anyhow::anyhow;
use colored::{ColoredString, Colorize};
use ignore::{overrides::OverrideBuilder, WalkBuilder};
use lazy_static::lazy_static;
use lscolors::{LsColors, Style};
use regex::bytes::{Regex, RegexBuilder};
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    fmt::Display,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

use crossbeam_channel as channel;

use crate::{app::App, DEFAULT_MAX_DEPTH};
use anyhow::Result;
use wutag_core::tag::Tag;

pub fn fmt_err<E: Display>(err: E) -> String {
    format!("{} {}", "ERROR".red().bold(), format!("{}", err).white())
}

pub fn print_err(err: impl Into<String>) {
    eprintln!("{} {}", "[wutag error]".red().bold(), err.into());
}

pub(crate) fn fmt_ok<S: AsRef<str>>(msg: S) -> String {
    format!("{} {}", "OK".green().bold(), msg.as_ref().white())
}

pub(crate) fn fmt_path<P: AsRef<Path>>(path: P, ls_colors: bool) -> String {
    // ls_colors implies forced coloring
    if ls_colors {
        let lscolors = LsColors::from_env().unwrap_or_default();

        let style = lscolors.style_for_path(path.as_ref());
        let style = style
            .map(Style::to_ansi_term_style)
            .unwrap_or_else(|| ansi_term::Color::Blue.bold());

        format!("{}", style.paint(path.as_ref().display().to_string()))
    } else {
        format!("{}", path.as_ref().display().to_string().bold().blue())
    }
}

/// Format a local path (i.e., remove path components before files local to
/// directory)
pub(crate) fn fmt_local_path<P: AsRef<Path>>(path: P, local_path: P, ls_colors: bool) -> String {
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

        format!(
            "{}",
            style.paint(
                path.as_ref()
                    .display()
                    .to_string()
                    .replace(replaced.as_str(), "")
            )
        )
    } else {
        format!(
            "{}",
            path.as_ref()
                .display()
                .to_string()
                .replace(replaced.as_str(), "")
                .bold()
                .blue()
        )
    }
}

pub(crate) fn fmt_tag(tag: &Tag) -> ColoredString {
    tag.name().color(*tag.color()).bold()
}

pub(crate) fn raw_local_path<P: AsRef<Path>>(path: P, local: P) -> String {
    let mut replaced = local.as_ref().display().to_string();
    if !replaced.ends_with('/') {
        replaced.push('/');
    }
    path.as_ref()
        .display()
        .to_string()
        .replace(replaced.as_str(), "")
}

/// Determine whether file (path) contains path and if so, return true
pub fn contained_path<P: AsRef<Path>>(file: P, path: P) -> bool {
    file.as_ref()
        .display()
        .to_string()
        .contains(path.as_ref().to_str().unwrap())
}

/// Convert an OsStr to bytes for RegexBuilder
pub(crate) fn osstr_to_bytes(input: &OsStr) -> Cow<[u8]> {
    use std::os::unix::ffi::OsStrExt;
    Cow::Borrowed(input.as_bytes())
}

/// Build a glob from GlobBuilder and return a regex
pub(crate) fn glob_builder(pattern: &str) -> String {
    let builder = globset::GlobBuilder::new(pattern);
    builder
        .build()
        .expect("Invalid glob sequence")
        .regex()
        .to_owned()
}

/// Build a regular expression with RegexBuilder (bytes)
pub(crate) fn regex_builder(pattern: &str, case_insensitive: bool) -> Regex {
    lazy_static! {
        static ref UPPER_REG: Regex = Regex::new(r"[[:upper:]]").unwrap();
    };

    let cow_pat: Cow<OsStr> = Cow::Owned(OsString::from(pattern));
    let upper_char = !UPPER_REG.is_match(&osstr_to_bytes(cow_pat.as_ref()));

    RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive || upper_char)
        .build()
        .map_err(|e| {
            anyhow!(
                "{}\n\nInvalid pattern. You can use --regex to use a regular expression instead \
                 of a glob.",
                e.to_string()
            )
        })
        .expect("Invalid pattern")
}

/// Returns an ignore::WalkParallel instance that uses `base_path`, and a
/// pattern (both glob and regex) does not follow symlinks, respects hidden
/// files, and uses max CPU's. If a `max_depth` is specified, the parallel
/// walker will not traverse deeper than that, else if no `max_depth` is
/// specified, it will use [DEFAULT_MAX_DEPTH](DEFAULT_MAX_DEPTH).
pub(crate) fn reg_walker(app: &Arc<App>) -> Result<ignore::WalkParallel> {
    let mut override_builder = OverrideBuilder::new(&app.base_dir);
    for excluded in &app.exclude {
        override_builder
            .add(excluded.as_str())
            .map_err(|e| anyhow!("Malformed exclude pattern: {}", e))?;
    }

    let overrides = override_builder
        .build()
        .map_err(|_| anyhow!("Mismatch in exclude patterns"))?;

    Ok(WalkBuilder::new(&app.base_dir)
        .threads(num_cpus::get())
        .follow_links(false)
        .hidden(true)
        .ignore(false)
        .overrides(overrides)
        .git_global(false)
        .git_ignore(false)
        .git_exclude(false)
        .parents(false)
        .max_depth(app.max_depth)
        .build_parallel())
}

/// Type to execute a closure across multiple threads when wrapped in `Mutex`
type FnThread = Box<dyn FnMut(&ignore::DirEntry) + Send>;
lazy_static::lazy_static! {
    static ref FNIN: Mutex<Option<FnThread>> = Mutex::new(None);
}

/// Traverses directories using `ignore::WalkParallel`, sending matches across
/// channels to make the process faster. Executes closure `f` on each matching
/// entry
pub(crate) fn reg_ok<F>(pattern: Arc<Regex>, app: &Arc<App>, f: F) -> Result<()>
where
    F: FnMut(&ignore::DirEntry) + Send + Sync + 'static,
{
    let (tx, rx) = channel::unbounded::<ignore::DirEntry>();
    *FNIN.lock().expect("broken lock in closure") = Some(Box::new(f));

    let execution_thread = thread::spawn(move || {
        rx.iter().for_each(|e| {
            if let Some(ref mut handler) = *FNIN.lock().expect("poisoned lock") {
                handler(&e)
            }
        })
        // app.save_registry();
    });

    log::debug!("Using regex with max_depth of: {}", app.max_depth.unwrap());
    log::debug!(
        "Using regex with base_dir of: {}",
        app.base_dir.to_string_lossy().to_string()
    );
    reg_walker(app).unwrap().run(|| {
        let tx = tx.clone();
        let pattern = Arc::clone(&pattern);
        let app = Arc::clone(app);

        Box::new(move |res| {
            //: Result<ignore::DirEntry,ignore::Error>
            let entry = match res {
                Ok(_entry) => _entry,
                Err(err) => {
                    print_err(format!("unable to access entry {}", err));
                    return ignore::WalkState::Continue;
                },
            };

            // Filter out depths that are greater than the configured or default
            // The max depth used when building unwraps to opts.depth and config.depth
            // not the DEFAULT_MAX_DEPTH
            if entry.depth() > app.max_depth.unwrap_or(DEFAULT_MAX_DEPTH) {
                log::trace!("max_depth reached");
                return ignore::WalkState::Continue;
            }

            let entry_path = entry.path();

            // Verify a file name is actually present
            let entry_fname: Cow<OsStr> = match entry_path.file_name() {
                Some(f) => Cow::Borrowed(f),
                _ => unreachable!("Invalid file reached"),
            };

            // Filter out patterns that don't match
            if !pattern.is_match(&osstr_to_bytes(entry_fname.as_ref())) {
                log::trace!("no match, skipping");
                return ignore::WalkState::Continue;
            }

            // Filter out extensions that don't match (if present)
            if let Some(ref ext) = app.extension {
                if let Some(fname) = entry_path.file_name() {
                    if !ext.is_match(&osstr_to_bytes(fname)) {
                        return ignore::WalkState::Continue;
                    }
                } else {
                    return ignore::WalkState::Continue;
                }
            }

            // Using a match statement does not preserve output order for some reason
            let send = tx.send(entry);
            if send.is_err() {
                log::trace!("Sent quit");
                return ignore::WalkState::Quit;
            }

            log::trace!("Sent continue");
            ignore::WalkState::Continue
        })
    });
    drop(tx);
    execution_thread.join().unwrap();
    // app.save_registry();
    Ok(())
}
