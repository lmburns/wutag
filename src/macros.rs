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

/// Make a path display in bold letters
#[macro_export]
macro_rules! bold_entry {
    ($entry:ident) => {
        $entry.display().to_string().bold()
    };
}
