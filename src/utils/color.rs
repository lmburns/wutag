//! Utility functions used through this crate and by the main executable
use crate::xattr::{Error, Result};
use std::str::FromStr;

use colored::Color;
use tui::style as tui;

/// Parses a [`Color`] from a foreground color string
pub(crate) fn color_from_fg_str(s: &str) -> Option<Color> {
    match s {
        "30" => Some(Color::Black),
        "31" => Some(Color::Red),
        "32" => Some(Color::Green),
        "33" => Some(Color::Yellow),
        "34" => Some(Color::Blue),
        "35" => Some(Color::Magenta),
        "36" => Some(Color::Cyan),
        "37" => Some(Color::White),
        "90" => Some(Color::BrightBlack),
        "91" => Some(Color::BrightRed),
        "92" => Some(Color::BrightGreen),
        "93" => Some(Color::BrightYellow),
        "94" => Some(Color::BrightBlue),
        "95" => Some(Color::BrightMagenta),
        "96" => Some(Color::BrightCyan),
        "97" => Some(Color::BrightWhite),
        color =>
            if color.starts_with("38;2;") {
                let mut it = s.split(';');
                it.next()?;
                it.next()?;
                Some(Color::TrueColor {
                    r: it.next()?.parse().ok()?,
                    g: it.next()?.parse().ok()?,
                    b: it.next()?.parse().ok()?,
                })
            } else {
                None
            },
    }
}

/// Parses a [`Color`](tui::Color) from a foreground color string
#[cfg(feature = "ui")]
pub(crate) fn color_tui_from_fg_str(s: &str) -> Option<tui::Color> {
    match s {
        "30" => Some(tui::Color::Black),
        "31" => Some(tui::Color::Red),
        "32" => Some(tui::Color::Green),
        "33" => Some(tui::Color::Yellow),
        "34" => Some(tui::Color::Blue),
        "35" => Some(tui::Color::Magenta),
        "36" => Some(tui::Color::Cyan),
        "37" => Some(tui::Color::White),
        "90" => Some(tui::Color::DarkGray),
        "91" => Some(tui::Color::LightRed),
        "92" => Some(tui::Color::LightGreen),
        "93" => Some(tui::Color::LightYellow),
        "94" => Some(tui::Color::LightBlue),
        "95" => Some(tui::Color::LightMagenta),
        "96" => Some(tui::Color::LightCyan),
        "97" => None,
        color =>
            if color.starts_with("38;2;") {
                let mut it = s.split(';');
                it.next()?;
                it.next()?;
                Some(tui::Color::Rgb(
                    it.next()?.parse().ok()?,
                    it.next()?.parse().ok()?,
                    it.next()?.parse().ok()?,
                ))
            } else {
                None
            },
    }
}

const fn hex_val(ch: u8) -> u8 {
    match ch {
        b'0'..=b'9' => ch - 48,
        b'A'..=b'F' => ch - 55,
        b'a'..=b'f' => ch - 87,
        _ => 0,
    }
}

/// Convert 2 digits of a 6 digit hex-color into one number
///
/// Either R, G, or B
const fn hex_chars_to_u8(ch: (u8, u8)) -> u8 {
    let mut result = 0;
    result |= hex_val(ch.0);
    result <<= 4_i32;
    result |= hex_val(ch.1);
    result
}

/// Parse a string into an `RGB` representation of a color
#[allow(clippy::indexing_slicing)]
fn parse_hex(color: &str) -> Option<(u8, u8, u8)> {
    let mut bytes = color.as_bytes().chunks(2);

    Some((
        bytes.next().map(|arr| hex_chars_to_u8((arr[0], arr[1])))?,
        bytes.next().map(|arr| hex_chars_to_u8((arr[0], arr[1])))?,
        bytes.next().map(|arr| hex_chars_to_u8((arr[0], arr[1])))?,
    ))
}

/// Parses a [`Color`] from a `String`. If the provided string
/// starts with `0x` or `#` or without any prefix the color will be treated as
/// hex color notation so any colors like `0x1f1f1f` or `#ABBA12` or `121212`
/// are valid.
///
/// # Errors
/// If the color is unable to be parsed into a [`Color`], an `Error` is raised
///
/// [`Color`]: colored::Color
pub(crate) fn parse_color<S: AsRef<str>>(color: S) -> Result<Color> {
    let color = color.as_ref();

    /// If the given item is 6 in length, wrap it in `Some`, else `None`
    // if $c.len() == 6 { Some($c) } else { None }
    macro_rules! if_6 {
        ($c:ident) => {
            ($c.len() == 6).then(|| $c)
        };
    }

    // Needs to be checked first. Some colors may pass hex parse
    if let Ok(c) = Color::from_str(color) {
        return Ok(c);
    }

    let result = color.strip_prefix("0x").map_or_else(
        || {
            color
                .strip_prefix('#')
                .map_or_else(|| if_6!(color), |c| if_6!(c))
        },
        |c| if_6!(c),
    );

    if let Some(color) = result {
        // hex
        if let Some((r, g, b)) = parse_hex(color) {
            return Ok(Color::TrueColor { r, g, b });
        }
    }
    Err(Error::InvalidColor(color.to_owned()))
}

/// Parses a [`Color`](cli_table::Color) from a String. If the provided string
/// starts with `0x` or `#` or without any prefix the color will be treated as
/// hex color notation so any colors like `0x1f1f1f` or `#ABBA12` or `121212`
/// are valid.
///
/// # Errors
/// If the color is unable to be parsed into a [`Color`], an `Error` is raised
///
/// [`Color`]: cli_table::Color
pub(crate) fn parse_color_cli_table<S: AsRef<str>>(color: S) -> Result<cli_table::Color> {
    let color = color.as_ref();

    /// If the given item is 6 in length, wrap it in `Some`, else `None`
    macro_rules! if_6 {
        ($c:ident) => {
            ($c.len() == 6).then(|| $c)
        };
    }

    // Needs to be checked first. Some colors may pass hex parse
    if let Ok(c) = cli_table::Color::from_str(color) {
        return Ok(c);
    }

    let result = color.strip_prefix("0x").map_or_else(
        || {
            color
                .strip_prefix('#')
                .map_or_else(|| if_6!(color), |c| if_6!(c))
        },
        |c| if_6!(c),
    );

    if let Some(color) = result {
        // hex
        if let Some((r, g, b)) = parse_hex(color) {
            return Ok(cli_table::Color::Rgb(r, g, b));
        }
    }
    Err(Error::InvalidColor(color.to_owned()))
}

/// Parses a [`Color`](tui::Color) from a String. If the provided string
/// starts with `0x` or `#` or without any prefix the color will be treated as
/// hex color notation so any colors like `0x1f1f1f` or `#ABBA12` or `121212`
/// are valid.
///
/// # Errors
/// If the color is unable to be parsed into a [`Color`], an `Error` is raised
///
/// [`Color`]: tui::style::Color
#[cfg(feature = "ui")]
pub(crate) fn parse_color_tui<S: AsRef<str>>(color: S) -> Result<tui::Color> {
    let color = color.as_ref();

    /// If the given item is 6 in length, wrap it in `Some`, else `None`
    macro_rules! if_6 {
        ($c:ident) => {
            ($c.len() == 6).then(|| $c)
        };
    }

    let result = color.strip_prefix("0x").map_or_else(
        || {
            color
                .strip_prefix('#')
                .map_or_else(|| if_6!(color), |c| if_6!(c))
        },
        |c| if_6!(c),
    );

    if let Some(color) = result {
        // hex
        if let Some((r, g, b)) = parse_hex(color) {
            return Ok(tui::Color::Rgb(r, g, b));
        }
    }
    Err(Error::InvalidColor(color.to_owned()))
}

/// Wrapper for [`tui`] widget colors
#[cfg(feature = "ui")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct TuiColor {
    /// Inner [`tui`] widget color type
    inner: tui::Color,
}

impl TuiColor {
    /// Returns the underlying [`Color`] type
    ///
    /// [`Color`](tui::Color)
    pub(crate) const fn get(self) -> tui::Color {
        self.inner
    }
}

impl<'a> From<&'a str> for TuiColor {
    fn from(s: &'a str) -> Self {
        Self {
            inner: match s.to_ascii_lowercase().trim() {
                "black" => tui::Color::Black,
                "red" => tui::Color::Red,
                "green" => tui::Color::Green,
                "yellow" => tui::Color::Yellow,
                "blue" => tui::Color::Blue,
                "magenta" => tui::Color::Magenta,
                "cyan" => tui::Color::Cyan,
                "gray" => tui::Color::Gray,
                "darkgray" => tui::Color::DarkGray,
                "lightred" => tui::Color::LightRed,
                "lightgreen" => tui::Color::LightGreen,
                "lightyellow" => tui::Color::LightYellow,
                "lightblue" => tui::Color::LightBlue,
                "lightmagenta" => tui::Color::LightMagenta,
                "lightcyan" => tui::Color::LightCyan,
                "white" => tui::Color::White,
                _ => match parse_color_tui(s) {
                    Ok(rgb) => rgb,
                    Err(_) => Self::default().get(),
                },
            },
        }
    }
}

impl From<tui::Color> for TuiColor {
    fn from(t: tui::Color) -> Self {
        Self { inner: t }
    }
}

impl Default for TuiColor {
    #[inline]
    fn default() -> Self {
        Self {
            inner: tui::Color::Gray,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_color, parse_color_cli_table, parse_color_tui, tui};
    use colored::Color::{self, TrueColor};

    #[test]
    fn parse_colored_colors() {
        assert_eq!(parse_color("0xffffff").unwrap(), TrueColor {
            r: 255,
            g: 255,
            b: 255,
        });
        assert_eq!(parse_color("#ffffff").unwrap(), TrueColor {
            r: 255,
            g: 255,
            b: 255,
        });
        assert_eq!(parse_color("0ff00f").unwrap(), TrueColor {
            r: 15,
            g: 240,
            b: 15,
        });

        assert_eq!(parse_color("black").unwrap(), Color::Black);
        assert_eq!(parse_color("red").unwrap(), Color::Red);
        assert_eq!(parse_color("green").unwrap(), Color::Green);
        assert_eq!(parse_color("yellow").unwrap(), Color::Yellow);
        assert_eq!(parse_color("bLue").unwrap(), Color::Blue);
        assert_eq!(parse_color("magenTA").unwrap(), Color::Magenta);
        assert_eq!(parse_color("purple").unwrap(), Color::Magenta);
        assert_eq!(parse_color("cyan").unwrap(), Color::Cyan);
        assert_eq!(parse_color("white").unwrap(), Color::White);

        assert_eq!(parse_color("bright black").unwrap(), Color::BrightBlack);
        assert_eq!(parse_color("bright red").unwrap(), Color::BrightRed);
        assert_eq!(parse_color("Bright green").unwrap(), Color::BrightGreen);
        assert_eq!(parse_color("BrighT YELLOW").unwrap(), Color::BrightYellow);
        assert_eq!(parse_color("bright bLue").unwrap(), Color::BrightBlue);
        assert_eq!(parse_color("bright magenTA").unwrap(), Color::BrightMagenta);
        assert_eq!(parse_color("bright cyan").unwrap(), Color::BrightCyan);
        assert_eq!(parse_color("BRIght white").unwrap(), Color::BrightWhite);
    }

    #[test]
    fn parse_clitable_colors() {
        assert_eq!(
            parse_color_cli_table("0xffffff").unwrap(),
            cli_table::Color::Rgb(255, 255, 255)
        );
        assert_eq!(
            parse_color_cli_table("#ffffff").unwrap(),
            cli_table::Color::Rgb(255, 255, 255)
        );
        assert_eq!(
            parse_color_cli_table("0ff00f").unwrap(),
            cli_table::Color::Rgb(15, 240, 15)
        );

        assert_eq!(
            parse_color_cli_table("black").unwrap(),
            cli_table::Color::Black
        );
        assert_eq!(
            parse_color_cli_table("bLue").unwrap(),
            cli_table::Color::Blue
        );
        assert_eq!(
            parse_color_cli_table("green").unwrap(),
            cli_table::Color::Green
        );
        assert_eq!(parse_color_cli_table("red").unwrap(), cli_table::Color::Red);
        assert_eq!(
            parse_color_cli_table("cyan").unwrap(),
            cli_table::Color::Cyan
        );
        assert_eq!(
            parse_color_cli_table("magenTA").unwrap(),
            cli_table::Color::Magenta
        );
        assert_eq!(
            parse_color_cli_table("yellow").unwrap(),
            cli_table::Color::Yellow
        );
        assert_eq!(
            parse_color_cli_table("white").unwrap(),
            cli_table::Color::White
        );
    }

    #[cfg(feature = "ui")]
    #[test]
    fn parse_tui_colors() {
        assert_eq!(
            parse_color_tui("0xffffff").unwrap(),
            tui::Color::Rgb(255, 255, 255)
        );
        assert_eq!(
            parse_color_tui("#ffffff").unwrap(),
            tui::Color::Rgb(255, 255, 255)
        );
        assert_eq!(
            parse_color_tui("0ff00f").unwrap(),
            tui::Color::Rgb(15, 240, 15)
        );
    }

    #[test]
    fn invalid_colored_colors() {
        assert!(parse_color("0ff00").is_err());
        assert!(parse_color("0x12345").is_err());
        assert!(parse_color("#53241").is_err());
        assert!(parse_color("1234567").is_err());
        assert!(parse_color("#1234567").is_err());
        assert!(parse_color("0x1234567").is_err());
    }

    #[test]
    fn invalid_clitable_colors() {
        assert!(parse_color_cli_table("0ff00").is_err());
        assert!(parse_color_cli_table("0x12345").is_err());
        assert!(parse_color_cli_table("#53241").is_err());
        assert!(parse_color_cli_table("1234567").is_err());
        assert!(parse_color_cli_table("#1234567").is_err());
        assert!(parse_color_cli_table("0x1234567").is_err());
    }

    #[cfg(feature = "ui")]
    #[test]
    fn invalid_tui_colors() {
        assert!(parse_color_tui("0ff00").is_err());
        assert!(parse_color_tui("0x12345").is_err());
        assert!(parse_color_tui("#53241").is_err());
        assert!(parse_color_tui("1234567").is_err());
        assert!(parse_color_tui("#1234567").is_err());
        assert!(parse_color_tui("0x1234567").is_err());
    }
}
