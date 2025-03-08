use ansi_term::Colour;
use scraper::{Html, Selector};
use std::str::FromStr;

#[derive(PartialEq, Debug)]
struct PageStyle {
    bg: BgColour,
    fg: FgColour,
}

impl Default for PageStyle {
    fn default() -> Self {
        Self {
            bg: BgColour::Black,
            fg: FgColour::White,
        }
    }
}

impl FromStr for PageStyle {
    type Err = String;

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
                })
            }
            [s0, s1] => {
                let bg = s0.parse()?;
                let fg = s1.parse()?;
                Ok(Self { bg, fg })
            }
            // TODO: Converting referenced GIF from 'bgImg' to ASCII is not yet supported.
            [_, _, _] => Ok(Self::default()),
            _ => Err(format!("invalid svt colour class: {s}")),
        }
    }
}

#[derive(PartialEq, Debug)]
enum BgColour {
    Black,
    Blue,
    White,
    Yellow,
}

impl Default for BgColour {
    fn default() -> Self {
        Self::Black
    }
}

impl FromStr for BgColour {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bg = match s {
            "bgB" => Self::Blue,
            "bgBl" => Self::Black,
            "bgW" => Self::White,
            "bgY" => Self::Yellow,
            _ => return Err(format!("invalid bg: {s}")),
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
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fg = match s {
            "bl" => Self::Black,
            "B" => Self::Blue,
            "C" => Self::Cyan,
            "G" => Self::Green,
            "R" => Self::Red,
            "W" => Self::White,
            "Y" => Self::Yellow,
            _ => return Err(format!("invalid fg: {s}")),
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
            BgColour::White => style.on(Colour::White),
            BgColour::Yellow => style.on(Colour::Yellow),
        };
        // Set foreground
        match value.fg {
            FgColour::Black => style.fg(Colour::Black),
            FgColour::Blue => style.fg(Colour::Blue),
            FgColour::Cyan => style.fg(Colour::Cyan),
            FgColour::Green => style.fg(Colour::Green),
            FgColour::Red => style.fg(Colour::Red),
            FgColour::White => style.fg(Colour::White),
            FgColour::Yellow => style.fg(Colour::Yellow),
        }
    }
}

pub fn parse(html: &str) -> String {
    let fragment = Html::parse_fragment(html);

    // Select `span` that represent a line of a page. These can be identified
    // by the `class` attribute, and can be different variants, e.g. `line toprow`,
    // `line`, and `line DH`. Use a wildcard to match all line variants.
    let selector = Selector::parse(r#"span[class*="line"]"#).unwrap();

    // TODO: Preallocate string with some reasonable size.
    let mut page = String::new();

    for element in fragment.select(&selector) {
        for c in element.child_elements() {
            // TODO: Parse into custom type; with `parse` and impl `FromStr`?
            let class_str = c.attr("class").unwrap();
            let text = c.text().collect::<String>();

            let svt_color = PageStyle::from_str(class_str).unwrap();
            let style = ansi_term::Style::from(svt_color);
            page.push_str(&style.paint(&text).to_string());
        }
        page.push('\n');
    }

    // FIXME: Hack
    page.pop(); // Remove last '\n'

    page
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
                },
            },
            TestCase {
                input: "bgBl W",
                expected: PageStyle {
                    bg: BgColour::Black,
                    fg: FgColour::White,
                },
            },
            TestCase {
                input: "bgB W bgImg",
                expected: PageStyle::default(),
            },
        ];
        for case in test_cases.iter() {
            let result = PageStyle::from_str(case.input).unwrap();
            assert_eq!(result, case.expected);
        }
    }
}
