//! Macros that are used in more than one file

/// Format errors
#[macro_export]
macro_rules! err {
    ($err:ident, $entry:ident) => {
        err!("", $err, $entry);
    };
    ($prefix:expr, $err:ident, $entry:ident) => {{
        let err = fmt_err($err);
        eprintln!("{}{} - {}", $prefix, err, $entry.path().to_string_lossy().bold());
    }};
}

/// Detect if the files should be displayed globally
#[macro_export]
macro_rules! global_opts {
    ($local:expr, $global:expr, $is_global:expr, $garrulous:expr) => {
        if $garrulous {
            tern::t!($is_global ? println!("{}", $global) : println!("{}", $local));
        } else if $is_global {
            print!("{}", $global);
        } else {
            print!("{}", $local);
        }
    };
}

/// Create a simple method to a struct that returns the field name. This is to
/// allow access to the field names without direct access for modification. This
/// _always_ returns a reference to the field. There are probably better ways of
/// doing this
#[macro_export]
macro_rules! inner_immute {
    // A placeholder here `$ref` which just implements a non-reference return-type
    ($name:ident, $return:ty, $ref:tt) => {
        pub(crate) const fn $name(&self) -> $return {
            self.$name
        }
    };
    ($name:ident, $return:ty) => {
        pub(crate) const fn $name(&self) -> &$return {
            &self.$name
        }
    };
}

/// Expand to an error message
#[macro_export]
macro_rules! wutag_error {
    ($($err:tt)*) => ({
        use colored::Colorize;
        eprintln!("{}: {}", "[wutag error]".red().bold(), format!($($err)*));
    })
}

/// Expand to a fatal message
#[macro_export]
macro_rules! wutag_fatal {
    ($($err:tt)*) => ({
        use colored::Colorize;
        eprintln!("{}: {}", "[wutag fatal]".magenta().bold(), format!($($err)*));
        std::process::exit(1);
    })
}

/// Expand to an info message
#[macro_export]
macro_rules! wutag_info {
    ($($err:tt)*) => ({
        use colored::Colorize;
        eprintln!("{}: {}", "[wutag info]".green().bold(), format!($($err)*));
    })
}

/// Expand to a warning message
#[macro_export]
macro_rules! wutag_warning {
    ($($err:tt)*) => ({
        use colored::Colorize;
        eprintln!("{}: {}", "[wutag warning]".yellow().bold(), format!($($err)*));
    })
}

/// Use for debugging purposes
#[macro_export]
macro_rules! wutag_debug {
    ($($err:tt)*) => ({
        use colored::Colorize;
        eprintln!("{}: {}", "[wutag debug]".blue().bold(), format!($($err)*));
    })
}

/// Only print messages if the user has not enabled `quiet` mode
#[macro_export]
macro_rules! qprint {
    ($quiet:tt, $($err:tt)*) => ({
        if !$quiet.quiet {
            println!("{}", format!($($err)*));
        }
    })
}

/// Custom `assert` message that allows for a more customized string with
/// variables used in the formatted string. It does require the message to be a
/// string literal like the standard library's `assert`
#[macro_export]
macro_rules! cassert_eq {
    ($left:expr , $right:expr) => ({
        match (&($left), &($right)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    wutag_fatal!("{}: `(left == right)` (left: `{:?}`, right: `{:?}`)",
                        "assertion failed".red().bold(), left_val, right_val)
                }
            }
        }
    });
    ($left:expr , $right:expr, $fmt:expr, $($arg:tt)*) => ({
        match (&($left), &($right)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    wutag_fatal!($fmt, $($arg)*)
                }
            }
        }
    });
}

// Color can be disabled the same way that [`colored`] can

/// *Red literal*
/// Make text red and bold
/// Used to display errors and debugging messages
#[macro_export]
macro_rules! r {
    ($t:tt) => {
        $t.red().bold()
    };
}

/// *Green literal*
/// Make text green and bold. Used more so with ID's
/// Used to display errors and debugging messages
#[macro_export]
macro_rules! g {
    ($t:tt) => {
        $t.green().bold()
    };
}

/// *Green string allocation*
/// Make text green and bold.
/// Same as macro `g`, except this works with string literals
#[macro_export]
macro_rules! gs {
    ($t:tt) => {
        $t.to_string().green().bold()
    };
}

/// Make a path display in bold letters
#[macro_export]
macro_rules! bold_entry {
    ($entry:ident) => {
        $entry.display().to_string().bold()
    };
}

/// Initialize a [`Regex`] once
#[macro_export]
macro_rules! regex {
    ($re:expr $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

/// Convert [`PathBuf`] to [`String`]
#[macro_export]
macro_rules! path_str {
    ($p:expr) => {
        $p.to_string_lossy().to_string()
    };
}

// Probably a much better way to do these macros

/// A couple characters shorter to write an error message ("failed ...ing ...")
#[macro_export]
macro_rules! fail {
    ($($arg:tt)*) => ({
        format!("failed {}", format!($($arg)*))
    })
}

/// A couple characters shorter to write an error message ("failed to ...")
#[macro_export]
macro_rules! failt {
    ($($arg:tt)*) => ({
        format!("failed to {}", format!($($arg)*))
    })
}

/// Not a macro, but checks whether the user is qualified for the `file-flags`
/// feature
pub(crate) const fn wants_feature_flags() -> bool {
    cfg!(feature = "file-flags") && cfg!(unix) && !cfg!(macos)
}
