//! Macros that are used in more than one file

/// Format errors
#[macro_export]
macro_rules! err {
    ($err:ident, $entry:ident) => {
        err!("", $err, $entry);
    };
    ($prefix:expr, $err:ident, $entry:ident) => {{
        let err = fmt_err($err);
        eprintln!(
            "{}{} - {}",
            $prefix,
            err,
            $entry.path().to_string_lossy().bold()
        );
    }};
}

/// Makeshift ternary 2 == 2 ? "yes" : "no", mainly used for printing
#[macro_export]
macro_rules! ternary {
    ($c:expr, $v:expr, $v1:expr) => {
        if $c {
            $v
        } else {
            $v1
        }
    };
}

/// Detect if the files should be displayed globally
#[macro_export]
macro_rules! global_opts {
    ($local:expr, $global:expr, $app:ident, $garrulous:expr) => {
        if $garrulous {
            ternary!($app.global, println!("{}", $global), println!("{}", $local));
        } else if $app.global {
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
        eprintln!("{}: {}", "[wutag error]".red().bold(), format!($($err)*));
    })
}

/// Expand to a fatal message
#[macro_export]
macro_rules! wutag_fatal {
    ($($err:tt)*) => ({
        eprintln!("{}: {}", "[wutag fatal]".yellow().bold(), format!($($err)*));
        std::process::exit(1);
    })
}

/// Expand to an info message
#[macro_export]
macro_rules! wutag_info {
    ($($err:tt)*) => ({
        eprintln!("{}: {}", "[wutag info]".green().bold(), format!($($err)*));
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

/// A couple characters shorter to write an error message
#[macro_export]
macro_rules! fail {
    ($($arg:tt)*) => ({
        format!("failed to {}", format!($($arg)*))
    })
}

/// Easier error message expansion, since the base part is repeated
#[macro_export]
macro_rules! failure {
    ($action:expr, $kind:expr) => {
        $crate::fail!("{} {}", $action, $kind)
    };
    ($action:expr, $kind:expr, $by:expr) => {
        $crate::fail!("{} {} by {}", $action, $kind, $by)
    };
    ($action:expr, $kind:expr, $by:expr, $val:expr) => {
        $crate::fail!("{} {} by {}: {}", $action, $kind, $by, $val)
    };
}

/// A conversion failure
#[macro_export]
macro_rules! conv_fail {
    ($kind:expr) => {
        $crate::fail!("convert to {}", $kind)
    };
}

/// A retrieve failure
#[macro_export]
macro_rules! retr_fail {
    ($kind:expr) => {
        $crate::failure!("retrieve", $kind)
    };
    ($kind:expr, $by:expr) => {
        $crate::failure!("retrieve", $kind, $by)
    };
    ($kind:expr, $by:expr, $val:expr) => {
        $crate::failure!("retrieve", $kind, $by, $val)
    };
}

/// A query failure
#[macro_export]
macro_rules! query_fail {
    ($kind:expr) => {
        $crate::failure!("query", $kind)
    };
    ($kind:expr, $by:expr) => {
        $crate::failure!("query", $kind, $by)
    };
    ($kind:expr, $by:expr, $val:expr) => {
        $crate::failure!("query", $kind, $by, $val)
    };
}

/// Not a macro, but checks whether the user is qualified for the `file-flags` feature
pub(crate) const fn wants_feature_flags() -> bool {
    cfg!(feature = "file-flags") && cfg!(unix) && !cfg!(macos)
}
