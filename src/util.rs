//! Utility functions used throughout this crate

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use clap_complete::{generate, Generator};
use colored::{Color, ColoredString, Colorize};
use crossbeam_channel as channel;
use crossbeam_utils::thread;
use env_logger::fmt::Color as LogColor;
use ignore::{overrides::OverrideBuilder, WalkBuilder};
use lexiclean::Lexiclean;
use log::LevelFilter;
use lscolors::{LsColors, Style};
use once_cell::sync::OnceCell;
use regex::bytes::{Regex, RegexBuilder};
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    fmt::Display,
    fs,
    io::{self, BufRead, BufReader, Cursor, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Once},
    time::SystemTime,
};

use crate::{
    consts::{APP_NAME, DEFAULT_MAX_DEPTH},
    filesystem::{create_temp_ignore, delete_file, osstr_to_bytes, write_temp_ignore},
    registry::types::Tag,
    subcommand::App,
    wutag_error, wutag_fatal, Opts,
};
use wutag_core::tag::Tag as WTag;

/// Run `initialize_logging` one time
static ONCE: Once = Once::new();
/// `Regex` to match uppercase characters
static UPPER_REG: OnceCell<Regex> = OnceCell::new();

/// Initialize logging for this crate
pub(crate) fn initialize_logging(args: &Opts) {
    ONCE.call_once(|| {
        env_logger::Builder::new()
            .format_timestamp(None)
            .format(|buf, record| {
                let mut style = buf.style();
                let level_style = match record.level() {
                    log::Level::Warn => style.set_color(LogColor::Yellow),
                    log::Level::Info => style.set_color(LogColor::Green),
                    log::Level::Debug => style.set_color(LogColor::Magenta),
                    log::Level::Trace => style.set_color(LogColor::Cyan),
                    log::Level::Error => style.set_color(LogColor::Red),
                };

                let mut style = buf.style();
                let target_style = style.set_color(LogColor::Ansi256(14));

                writeln!(
                    buf,
                    " {}: {} {}",
                    level_style.value(record.level()),
                    target_style.value(record.target()),
                    record.args()
                )
            })
            .filter(None, match &args.verbose {
                1 => LevelFilter::Warn,
                2 => LevelFilter::Info,
                3 => LevelFilter::Debug,
                4 => LevelFilter::Trace,
                _ => LevelFilter::Off,
            })
            .init();
    });
}

/// Parse a path for arguments. The path must exist
pub(crate) fn parse_path<P: AsRef<Path>>(path: P) -> Result<(), String> {
    fs::metadata(path)
        .map_err(|_e| "must be a valid path")
        .map(|_| ())
        .map_err(ToString::to_string)
}

/// Format one style of an error message
pub(crate) fn fmt_err<E: Display>(err: E) -> String {
    format!("{} {}", "ERROR:".red().bold(), format!("{}", err).white())
}

/// Format an `OK` message
pub(crate) fn fmt_ok<S: AsRef<str>>(msg: S) -> String {
    format!("{} {}", "OK".green().bold(), msg.as_ref().white())
}

/// Format the colored/non-colored output of a path
pub(crate) fn fmt_path<P: AsRef<Path>>(path: P, base_color: Color, ls_colors: bool) -> String {
    // ls_colors implies forced coloring
    if ls_colors {
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
            path.as_ref().display().to_string().color(base_color).bold()
        )
    }
}

/// Format a local path (i.e., remove path components before files local to
/// directory)
pub(crate) fn fmt_local_path<P: AsRef<Path>>(
    path: P,
    local_path: P,
    base_color: Color,
    ls_colors: bool,
) -> String {
    let mut replaced = local_path.as_ref().display().to_string();
    if !replaced.ends_with('/') {
        replaced.push('/');
    }

    let path = path
        .as_ref()
        .display()
        .to_string()
        .replace(replaced.as_str(), "");

    if ls_colors {
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
        format!("{}", path.color(base_color).bold())
    }
}

/// Format the tag by coloring it the specified color
// XXX: Remove once ready
pub(crate) fn fmt_tag_old(tag: &WTag) -> ColoredString {
    tag.name().color(*tag.color()).bold()
}

/// Format the tag by coloring it the specified color
pub(crate) fn fmt_tag(tag: &Tag) -> ColoredString {
    tag.name().color(tag.color()).bold()
}

/// Return a local path with no color, i.e., one in which /home/user/... is not
/// used and it is relative to the current directory. The searching of the paths
/// does not go above the folder in which this command is read and only searches
/// recursively
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

/// Modify completion output by using [comp_helper](crate::comp_helper)
pub(crate) fn replace(haystack: &mut String, needle: &str, repl: &str) -> Result<()> {
    if let Some(index) = haystack.find(needle) {
        haystack.replace_range(index..index + needle.len(), repl);
        Ok(())
    } else {
        Err(anyhow!(
            "\n====================\nFailed to find text\n====================n{}\n\u{2026}in \
             completion script:\n{}",
            needle,
            haystack
        ))
    }
}

/// Collect the paths that are entered in through `stdin`. This can be achieved
/// by doing something like: `fd <name> -tf | wutag set <tag>`
pub(crate) fn collect_stdin_paths(base: &Path) -> Vec<PathBuf> {
    BufReader::new(io::stdin())
        .lines()
        .map(|p| PathBuf::from(p.expect("failed to get path from `stdin`").as_str()).lexiclean())
        .filter(|path| {
            fs::symlink_metadata(path).is_ok() || fs::symlink_metadata(base.join(path)).is_ok()
        })
        .map(|p| base.join(p))
        .collect::<Vec<_>>()
}

/// Convert a [`SystemTime`](std::time::SystemTime) to
/// [`DateTime`](chrono::DateTime) for displaying purposes
pub(crate) fn systemtime_to_datetime(t: SystemTime) -> String {
    let dt: DateTime<Local> = t.into();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Create a simple yes/no prompt
pub(crate) fn prompt<S: AsRef<str>, P: AsRef<Path>>(prompt: S, path: P) -> bool {
    /// Macro to create a prompt
    macro_rules! prompt {
        ($dis:ident) => {
            $dis!(
                "{}\n\t- {} [{}/{}] ",
                prompt.as_ref(),
                path.as_ref().display().to_string().yellow().bold(),
                "y".green().bold(),
                "N".red().bold()
            )
        };
    }

    let prompt = {
        prompt!(print);

        if io::stdout().flush().is_err() {
            prompt!(println);
        }

        let mut input = String::new();
        let mut stdin = BufReader::new(io::stdin());

        if let Err(e) = stdin.read_line(&mut input) {
            wutag_fatal!("{}", e);
        }

        match input.to_ascii_lowercase().trim() {
            "y" | "ye" | "1" => true,
            "n" | "0" => false,
            s => s.starts_with("yes") || s.starts_with("true"),
        }
    };

    prompt
}

/// Print completions to `stdout` or to a file
pub(crate) fn gen_completions<G: Generator>(
    gen: G,
    app: &mut clap::Command,
    cursor: &mut Cursor<Vec<u8>>,
) {
    generate(gen, app, APP_NAME, cursor);
}

/// Test the output status of a command
#[allow(dead_code)]
pub(crate) fn command_status(cmd: &str, args: &[&str]) -> Result<(i32, String)> {
    use regex::Regex as Regexp;
    let patt = Regexp::new(r"(?i)error").context("failure to create regex")?;

    match Command::new(cmd).args(args).output() {
        Ok(output) =>
            if output.status.success() {
                let s = String::from_utf8(output.stdout)
                    .context("failed to convert command output to UTF-8")?
                    .trim_end()
                    .to_owned();

                if patt.is_match(&s) {
                    return Ok((1_i32, s));
                }

                Ok((output.status.code().unwrap_or(-1_i32), s))
            } else {
                let s = String::from_utf8(output.stderr)
                    .context("failed to convert command output to UTF-8")?
                    .trim_end()
                    .to_owned();
                Ok((output.status.code().unwrap_or(-1_i32), s))
            },
        Err(e) => Err(anyhow!(e.to_string())),
    }
}

/// Build a glob with [`wax`] and return a string to be compiled as a regular
/// expression
pub(crate) fn glob_builder(pattern: &str) -> String {
    wax::Glob::new(pattern)
        .map(|g| g.regex().to_string())
        .expect("failed to build glob")
}

/// Match uppercase characters against Unicode characters as well. WTags can
/// also be any valid Unicode character
pub(crate) fn contains_upperchar(pattern: &str) -> bool {
    let cow_pat: Cow<OsStr> = Cow::Owned(OsString::from(pattern));
    UPPER_REG
        .get_or_init(|| Regex::new(r"[[:upper:]]").expect("failed to build upper Regex"))
        .is_match(&osstr_to_bytes(cow_pat.as_ref()))
}

/// Build a regular expression with [`RegexBuilder`](regex::bytes::RegexBuilder)
pub(crate) fn regex_builder(
    pattern: &str,
    case_insensitive: bool,
    case_sensitive: bool,
) -> regex::bytes::Regex {
    let sensitive = !case_insensitive && (case_sensitive || contains_upperchar(pattern));

    log::debug!(
        "SENSITIVE: {}, INSENSITIVE: {}, SENSITIVE_ACCOUNT: {}",
        case_sensitive,
        case_insensitive,
        sensitive
    );

    RegexBuilder::new(pattern)
        .case_insensitive(!sensitive)
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
pub(crate) fn reg_walker(app: &Arc<App>, follow_links: bool) -> Result<ignore::WalkParallel> {
    let mut override_builder = OverrideBuilder::new(&app.base_dir);
    for excluded in &app.exclude {
        override_builder
            .add(excluded.as_str())
            .map_err(|e| anyhow!("Exclude pattern failed to compile: {}", e))?;
    }

    let overrides = override_builder
        .build()
        .map_err(|e| anyhow!("Exclude pattern mismatch: {}", e))?;

    // if app.ignores.is_some() &&
    // app.ignores.clone().unwrap_or_else(Vec::new).len() > 0 { }

    let mut walker = WalkBuilder::new(&app.base_dir);

    // NOTE: I don't see a difference whether symlinks is true or false
    //       The same result is returned for single files.
    walker
        .threads(num_cpus::get())
        .follow_links(follow_links || app.follow_symlinks)
        .hidden(false)
        .ignore(false)
        .overrides(overrides)
        .git_global(false)
        .git_ignore(false)
        .git_exclude(false)
        .parents(false)
        .max_depth(app.max_depth);

    if let Some(ignore) = &app.ignores {
        let tmp = create_temp_ignore(&move |file: &mut fs::File| write_temp_ignore(ignore, file));
        let res = walker.add_ignore(&tmp);
        scopeguard::defer!(delete_file(tmp));
        match res {
            Some(ignore::Error::Partial(_)) | None => (),
            Some(err) => {
                wutag_error!(
                    "Problem with ignore pattern in wutag.yml: {}",
                    err.to_string()
                );
            },
        }
    }
    Ok(walker.build_parallel())
}

// NOTE: Old method to use for `reg_ok` function
// type FnMutThread = Box<dyn FnMut(&ignore::DirEntry) + Send>;
// static FN: Lazy<Mutex<Option<FnMutThread>>> = Lazy::new(|| Mutex::new(None));
//
// *FN.lock().expect("broken lock in closure") = Some(Box::new(f));
// if let Some(ref mut handler) = *FN.lock().expect("poisoned lock") {
//     handler(&e)
// }

/// Traverses directories using [`WalkParallel`], sending matches across
/// channels to make the process faster. Executes closure `f` on each matching
/// entry
///
/// [`WalkParallel`]: ignore::WalkParallel
pub(crate) fn reg_ok<F, T>(pattern: &Arc<Regex>, app: &Arc<App>, follow_links: bool, mut f: F)
where
    F: FnMut(&ignore::DirEntry) -> Result<T> + Send + Sync,
{
    let walker = reg_walker(app, follow_links).expect("failed to get `reg_walker` result");

    // TODO: Look into order of execution
    // Scope here does not require ownership of all the variables, or the use of a
    // static ref Mutex to execute the closure
    thread::scope(|scope| {
        let (tx, rx) = channel::unbounded::<ignore::DirEntry>();

        scope.spawn(|_| {
            let rx = rx;
            for entry in rx.iter() {
                if let Err(e) = f(&entry) {
                    wutag_error!("{e}");
                }
            }
        });

        scope.spawn(|_| {
            let tx = tx;
            walker.run(|| {
                let tx = tx.clone();
                let pattern = Arc::clone(pattern);
                let app = Arc::clone(app);

                Box::new(move |res| {
                    let entry = match res {
                        Ok(entry_) => entry_,
                        Err(err) => {
                            wutag_error!("unable to access entry {}", err);
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

                    // Filter out non-matching file types
                    if let Some(ref file_types) = app.file_type {
                        if file_types.should_ignore(&entry) {
                            log::debug!("Ignoring: {}", entry_path.display());
                            return ignore::WalkState::Continue;
                        }
                    }

                    // Using a match statement does not preserve output order for some reason
                    if let Err(e) = tx.send(entry) {
                        log::debug!("Sent quit: {:?}", e);
                        return ignore::WalkState::Quit;
                    }

                    log::trace!("Sent continue");
                    ignore::WalkState::Continue
                })
            });
        });
    })
    .expect("failed to spawn thread");

    log::debug!(
        "Using regex with max_depth of: {}",
        app.max_depth.unwrap_or(DEFAULT_MAX_DEPTH)
    );

    log::debug!(
        "Using regex with base_dir of: {}",
        app.base_dir.to_string_lossy().to_string()
    );
}
