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
        cli::Commands::Serve(args) => commands::serve::run(args),

        cli::Commands::Search(args) => commands::search::run(args),
        cli::Commands::Training(args) => commands::training::run(args),
        cli::Commands::Contest(args) => commands::contest::run(args),
        cli::Commands::User(args) => commands::user::run(args),
        cli::Commands::Solution(args) => commands::solution::run(args),
        cli::Commands::Submit(args) => commands::submit::run(args),
        cli::Commands::Record(args) => commands::record::run(args),
        cli::Commands::Tags => commands::tags::run(),
        cli::Commands::Rank(args) => commands::rank::run(args),
    }
}