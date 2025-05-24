use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Display pages as plain text.
    #[arg(short, long)]
    pub plain: bool,
}
