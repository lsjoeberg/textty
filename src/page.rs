use ansi_term::Colour;
use scraper::{Html, Selector};

pub fn parse(html: &str) -> String {
    let fragment = Html::parse_fragment(html);

    // Select `span` that represent a line of a page. These can be identified
    // by the `class` attribute, and can be different variants, e.g. `line toprow`,
    // `line`, and `line DH`. Use a wildcard to match all line variants.
    let selector = Selector::parse(r#"span[class*="line"]"#).unwrap();

    let mut page = String::new();

    for element in fragment.select(&selector) {
        for c in element.child_elements() {
            // TODO: Parse into custom type; with `parse` and impl `FromStr`?
            let class_str = c.attr("class").unwrap();
            let text = c.text().collect::<String>();

            let text_coloured = match class_str.chars().last().unwrap() {
                'Y' => Colour::Yellow.paint(&text).to_string(),
                'C' => Colour::Cyan.paint(&text).to_string(),
                'B' => Colour::Blue.paint(&text).to_string(),
                'G' => Colour::Green.paint(&text).to_string(),
                _ => text,
            };
            page.push_str(&text_coloured.to_string());
        }
        page.push('\n');
    }

    // FIXME: Hack
    page.pop(); // Remove last '\n'

    page
}
