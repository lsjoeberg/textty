use textty::tui::App;

fn main() -> color_eyre::Result<()> {
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}
