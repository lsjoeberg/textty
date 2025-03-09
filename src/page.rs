use ansi_term::Colour;
use scraper::{Html, Selector};
use std::str::FromStr;

use crate::error::Error;
use crate::mosaic;

#[derive(PartialEq, Debug)]
struct PageStyle {
    bg: BgColour,
    fg: FgColour,
    mosaic: bool,
}

impl Default for PageStyle {
    fn default() -> Self {
        Self {
            bg: BgColour::Black,
            fg: FgColour::White,
            mosaic: false,
        }
    }
}

impl FromStr for PageStyle {
    type Err = Error;

    // Parse colour codes from space-separated fields, e.g. 'bgB W bgImg'.
    // There can be 1-3 fields present in the string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split(' ').collect::<Vec<&str>>();
        match parts[..] {
            [s0] => {
                let bg = s0.parse()?;
                Ok(Self {
                    bg,
                    fg: FgColour::default(),
                    mosaic: false,
                })
            }
            [s0, s1] => {
                let bg = s0.parse()?;
                let fg = s1.parse()?;
                Ok(Self {
                    bg,
                    fg,
                    mosaic: false,
                })
            }
            [s0, s1, "bgImg"] => {
                let bg = s0.parse()?;
                let fg = s1.parse()?;
                Ok(Self {
                    bg,
                    fg,
                    mosaic: true,
                })
            }
            _ => Err(Error::ParseHtml(format!("invalid svt colour class: {s}"))),
        }
    }
}

#[derive(PartialEq, Debug)]
enum BgColour {
    Black,
    Blue,
    Red,
    White,
    Yellow,
}

impl Default for BgColour {
    fn default() -> Self {
        Self::Black
    }
}

impl FromStr for BgColour {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bg = match s {
            "bgB" => Self::Blue,
            "bgBl" => Self::Black,
            "bgR" => Self::Red,
            "bgW" => Self::White,
            "bgY" => Self::Yellow,
            _ => return Err(Error::ParseHtml(format!("invalid bg: {s}"))),
        };
        Ok(bg)
    }
}

#[derive(PartialEq, Debug)]
enum FgColour {
    Black,
    Blue,
    Cyan,
    Green,
    Magenta,
    Red,
    White,
    Yellow,
}

impl Default for FgColour {
    fn default() -> Self {
        Self::White
    }
}

impl FromStr for FgColour {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fg = match s {
            "bl" => Self::Black,
            "B" => Self::Blue,
            "C" => Self::Cyan,
            "G" => Self::Green,
            "M" => Self::Magenta,
            "R" => Self::Red,
            "W" | "" => Self::White, // Choice for empty string has no effect since there's no fg char
            "Y" => Self::Yellow,
            _ => return Err(Error::ParseHtml(format!("invalid fg: {s}"))),
        };
        Ok(fg)
    }
}

impl From<PageStyle> for ansi_term::Style {
    fn from(value: PageStyle) -> Self {
        let style = Self::new();
        // Set background
        let style = match value.bg {
            BgColour::Black => style.on(Colour::Black),
            BgColour::Blue => style.on(Colour::Blue),
            BgColour::Red => style.on(Colour::Red),
            BgColour::White => style.on(Colour::White),
            BgColour::Yellow => style.on(Colour::Yellow),
        };
        // Set foreground
        match value.fg {
            FgColour::Black => style.fg(Colour::Black),
            FgColour::Blue => style.fg(Colour::Blue),
            FgColour::Cyan => style.fg(Colour::Cyan),
            FgColour::Green => style.fg(Colour::Green),
            FgColour::Magenta => style.fg(Colour::Purple),
            FgColour::Red => style.fg(Colour::Red),
            FgColour::White => style.fg(Colour::White),
            FgColour::Yellow => style.fg(Colour::Yellow),
        }
    }
}

fn parse_gif_id(attr: &str) -> Option<u64> {
    let start = attr.find(|c: char| c.is_ascii_digit())?;
    let end = attr.find(".gif")?;
    attr[start..end].parse().ok()
}

/// Parse an HTML page from `texttv.nu/api` to a string that can be
/// displayed in a terminal.
///
/// # Errors
///
/// Will return `Err` if `html` cannot be parsed.
pub fn parse(html: &str) -> Result<String, Error> {
    let fragment = Html::parse_fragment(html);

    // Select `span` that represent a line of a page. These can be identified
    // by the `class` attribute, and can be different variants, e.g. `line toprow`,
    // `line`, and `line DH`. Use a wildcard to match all line variants.
    let Ok(selector) = Selector::parse(r#"span[class*="line"]"#) else {
        return Err(Error::ParseHtml("invalid texttv.nu HTML".into()));
    };

    // TODO: Preallocate string with some reasonable size.
    let mut page = String::new();

    for element in fragment.select(&selector) {
        for c in element.child_elements() {
            let Some(class_attr) = c.attr("class") else {
                return Err(Error::ParseHtml("no class string to parse".into()));
            };

            let parsed_style = PageStyle::from_str(class_attr)?;

            // If the HTML style references a GIF image, this means that a teletext mosiac
            // character should be picked to represent the GIF. Each mosiac has multiple
            // representations in the HTML-doc, one for each bg/fg colour combination that
            // exists.
            let text = if parsed_style.mosaic {
                let gif_id = c.attr("style").and_then(parse_gif_id).unwrap_or(0);
                mosaic::from_gif_id(gif_id).to_string()
            } else {
                c.text().collect::<String>()
            };

            let style = ansi_term::Style::from(parsed_style);
            page.push_str(&style.paint(&text).to_string());
        }
        page.push('\n');
    }

    // FIXME: Hack
    page.pop(); // Remove last '\n'

    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase {
        input: &'static str,
        expected: PageStyle,
    }

    #[test]
    fn test_parse_page_style() {
        let test_cases = [
            TestCase {
                input: "bgB",
                expected: PageStyle {
                    bg: BgColour::Blue,
                    fg: FgColour::White,
                    mosaic: false,
                },
            },
            TestCase {
                input: "bgBl W",
                expected: PageStyle {
                    bg: BgColour::Black,
                    fg: FgColour::White,
                    mosaic: false,
                },
            },
            TestCase {
                input: "bgB  bgImg",
                expected: PageStyle {
                    fg: FgColour::White,
                    bg: BgColour::Blue,
                    mosaic: true,
                },
            },
        ];
        for case in test_cases.iter() {
            let result = PageStyle::from_str(case.input).unwrap();
            assert_eq!(result, case.expected);
        }
    }
}
