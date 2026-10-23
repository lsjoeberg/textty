use clap::Parser;
use textty::cli::Cli;
use textty::tui::App;

fn main() -> color_eyre::Result<()> {
    // Install before `ratatui::init`, which wraps the current panic hook in one
    // that restores the terminal before the panic report is printed.
    color_eyre::install()?;

    let args = Cli::parse();
    let terminal = ratatui::init();
    let result = App::new(args.plain).run(terminal);
    ratatui::restore();
    result
}
