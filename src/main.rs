use anyhow::bail;
use std::env;

mod page;
mod ttv;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let number: u16 = match args.get(1) {
        Some(s) => match s.parse() {
            Ok(n) => n,
            Err(_) => {
                bail!("Error: not a number: '{s}'");
            }
        },
        None => 100, // home page
    };

    let response = ttv::get_page(number)?;

    // Simple page display.
    if let Some(html) = response.content.first() {
        let page = page::parse(html);
        println!("{:-<40}", "");
        println!("{page}");
        println!("{:-<40}", "");
    }

    Ok(())
}
