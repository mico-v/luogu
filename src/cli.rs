use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "luogu", version, about = "Luogu local practice CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Fetch problem and scaffold workspace files.
    Fetch(FetchArgs),
    /// Compile and judge local solution with samples.
    Judge(JudgeArgs),
    /// Show problem/history summary in terminal.
    Catalog(CatalogArgs),
    /// Start local web server for problem catalog/history.
    Serve(ServeArgs),
}

#[derive(Args, Debug)]
pub struct FetchArgs {
    /// Problem ID, such as P1000.
    pub pid: String,
    /// Root folder to store problems.
    #[arg(long, default_value = "problem")]
    pub base_dir: PathBuf,
    /// Overwrite existing problem directory files.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct JudgeArgs {
    /// Problem ID to judge.
    pub pid: String,
    /// Root folder containing problem directories.
    #[arg(long, default_value = "problem")]
    pub base_dir: PathBuf,
    /// C++ source filename in problem directory (default: main.cpp/main.cc/main.cxx).
    #[arg(long)]
    pub source: Option<String>,
    /// Timeout in seconds for each sample.
    #[arg(long, default_value_t = 3.0)]
    pub timeout: f64,
    /// Extra compile flags.
    #[arg(long, num_args = 0.., value_delimiter = ' ')]
    pub cflags: Vec<String>,
}

#[derive(Args, Debug)]
pub struct CatalogArgs {
    /// Optional pid filter.
    #[arg(long)]
    pub pid: Option<String>,
    /// Show judge history lines instead of problem list.
    #[arg(long)]
    pub history: bool,
    /// Maximum history entries.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

#[derive(Args, Debug)]
pub struct ServeArgs {
    /// HTTP bind host.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// HTTP bind port.
    #[arg(long, default_value_t = 8787)]
    pub port: u16,
    /// Max history records returned by API.
    #[arg(long, default_value_t = 200)]
    pub history_limit: usize,
}
