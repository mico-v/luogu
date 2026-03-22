mod cli;
mod commands;
mod models;
mod net;
mod storage;

use anyhow::Result;
use clap::Parser;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = cli::Cli::parse();
    match cli.command {
        cli::Commands::Fetch(args) => commands::fetch::run(args),
        cli::Commands::Judge(args) => commands::judge::run(args),
        cli::Commands::Catalog(args) => commands::catalog::run(args),
    }
}
