use clap::Parser;
use textty::cli::Cli;
use textty::tui::App;

fn main() -> color_eyre::Result<()> {
    let args = Cli::parse();
    let terminal = ratatui::init();
    let result = App::new(args.plain).run(terminal);
    ratatui::restore();
    result
}
