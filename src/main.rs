use textty::{error::Error, page, ttv};

fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    let number: u16 = match args.get(1) {
        Some(s) => {
            if let Ok(n) = s.parse() {
                n
            } else {
                eprintln!("Error: not a number: '{s}'");
                std::process::exit(1);
            }
        }
        None => 100, // home page
    };

    let response = ttv::get_page(number)?;

    // Simple page display.
    if let Some(html) = response.content.first() {
        let page = page::parse(html)?;
        println!("{:-<40}", "");
        println!("{page}");
        println!("{:-<40}", "");
    }

    Ok(())
}
