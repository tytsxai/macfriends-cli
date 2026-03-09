use anyhow::Result;
use clap::Parser;
use macfriends::{app, cli::Cli};

fn main() -> Result<()> {
    let cli = Cli::parse();
    app::run(cli)
}
