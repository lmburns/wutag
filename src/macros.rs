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
