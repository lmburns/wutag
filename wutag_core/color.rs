//! Utility functions used through this crate and by the main executable
use crate::{Error, Result};
use colored::Color;
use tui::style as tui;

// TODO: Add underline and inverse options

/// Parses a [Color](colored::Color) from a foreground color string
pub fn color_from_fg_str(s: &str) -> Option<Color> {
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

/// Parses a [Color](tui::style::Color) from a foreground color string
pub fn color_tui_from_fg_str(s: &str) -> Option<tui::Color> {
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

const fn hex_chars_to_u8(ch: (u8, u8)) -> u8 {
    let mut result = 0;
    result |= hex_val(ch.0);
    result <<= 4;
    result |= hex_val(ch.1);
    result
}

fn parse_hex(color: &str) -> Option<(u8, u8, u8)> {
    let mut bytes = color.as_bytes().chunks(2);

    Some((
        bytes.next().map(|arr| hex_chars_to_u8((arr[0], arr[1])))?,
        bytes.next().map(|arr| hex_chars_to_u8((arr[0], arr[1])))?,
        bytes.next().map(|arr| hex_chars_to_u8((arr[0], arr[1])))?,
    ))
}

/// Parses a [Color](colored::Color) from a String. If the provided string
/// starts with `0x` or `#` or without any prefix the color will be treated as
/// hex color notation so any colors like `0x1f1f1f` or `#ABBA12` or `121212`
/// are valid.
pub fn parse_color<S: AsRef<str>>(color: S) -> Result<Color> {
    let color = color.as_ref();
    macro_rules! if_6 {
        ($c:ident) => {
            if $c.len() == 6 {
                Some($c)
            } else {
                None
            }
        };
    }

    let result = if let Some(c) = color.strip_prefix("0x") {
        if_6!(c)
    } else if let Some(c) = color.strip_prefix('#') {
        if_6!(c)
    } else {
        if_6!(color)
    };

    if let Some(color) = result {
        // hex
        if let Some((r, g, b)) = parse_hex(color) {
            return Ok(Color::TrueColor { r, g, b });
        }
    }
    Err(Error::InvalidColor(color.to_string()))
}

/// Parses a [Color](cli_table::Color) from a String. If the provided string
/// starts with `0x` or `#` or without any prefix the color will be treated as
/// hex color notation so any colors like `0x1f1f1f` or `#ABBA12` or `121212`
/// are valid.
pub fn parse_color_cli_table<S: AsRef<str>>(color: S) -> Result<cli_table::Color> {
    let color = color.as_ref();
    macro_rules! if_6 {
        ($c:ident) => {
            if $c.len() == 6 {
                Some($c)
            } else {
                None
            }
        };
    }

    let result = if let Some(c) = color.strip_prefix("0x") {
        if_6!(c)
    } else if let Some(c) = color.strip_prefix('#') {
        if_6!(c)
    } else {
        if_6!(color)
    };

    if let Some(color) = result {
        // hex
        if let Some((r, g, b)) = parse_hex(color) {
            return Ok(cli_table::Color::Rgb(r, g, b));
        }
    }
    Err(Error::InvalidColor(color.to_string()))
}

/// Parses a [Color](tui::styles::Color) from a String. If the provided string
/// starts with `0x` or `#` or without any prefix the color will be treated as
/// hex color notation so any colors like `0x1f1f1f` or `#ABBA12` or `121212`
/// are valid.
pub fn parse_color_tui<S: AsRef<str>>(color: S) -> Result<tui::Color> {
    let color = color.as_ref();
    macro_rules! if_6 {
        ($c:ident) => {
            if $c.len() == 6 {
                Some($c)
            } else {
                None
            }
        };
    }

    let result = if let Some(c) = color.strip_prefix("0x") {
        if_6!(c)
    } else if let Some(c) = color.strip_prefix('#') {
        if_6!(c)
    } else {
        if_6!(color)
    };

    if let Some(color) = result {
        // hex
        if let Some((r, g, b)) = parse_hex(color) {
            return Ok(tui::Color::Rgb(r, g, b));
        }
    }
    Err(Error::InvalidColor(color.to_string()))
}

/// Wrapper for tui widget colors
#[derive(Clone, Copy, Debug)]
pub struct TuiColor {
    /// Inner tui widget color type
    inner: tui::Color,
}

impl TuiColor {
    /// Returns the underlying [`TuiColor`] type
    ///
    /// [`Color`](tui::style::Color)
    pub fn get(self) -> tui::Color {
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

impl Default for TuiColor {
    fn default() -> Self {
        Self {
            inner: tui::Color::Gray,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_color;
    use colored::Color::*;
    #[test]
    fn parses_colors() {
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
    }
    #[test]
    fn errors_on_invalid_colors() {
        assert!(parse_color("0ff00").is_err());
        assert!(parse_color("0x12345").is_err());
        assert!(parse_color("#53241").is_err());
        assert!(parse_color("1234567").is_err());
        assert!(parse_color("#1234567").is_err());
        assert!(parse_color("0x1234567").is_err());
    }
}
