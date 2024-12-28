use std::env;

mod ttv;

fn main() {
    let args: Vec<String> = env::args().collect();
    let number: u16 = match args.get(1) {
        Some(s) => match s.parse() {
            Ok(n) => n,
            Err(_) => {
                eprintln!("Error: not a number: '{s}'");
                return;
            }
        },
        None => 100, // home page
    };

    match ttv::get_page(number) {
        Ok(page) => println!("{page}"),
        Err(err) => eprintln!("Error: {err}"),
    }
}
