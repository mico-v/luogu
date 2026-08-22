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

    // ── New commands ──
    /// Search problems by keyword, difficulty, tag, etc.
    Search(SearchArgs),
    /// Fetch a problem list (training set) and download all its problems.
    Training(TrainingArgs),
    /// Fetch contest info and download all problems.
    Contest(ContestArgs),
    /// Query user information, practice stats, and rating.
    User(UserArgs),
    /// Fetch solutions for a problem.
    Solution(SolutionArgs),
    /// Submit code to Luogu (requires login cookies).
    Submit(SubmitArgs),
    /// View submission records.
    Record(RecordArgs),
    /// List problem tags.
    Tags,
    /// Show ranking (gu rating or elo).
    Rank(RankArgs),
}

// ── Existing command args ───────────────────────────────────────

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

// ── New command args ────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Keyword to search (title).
    #[arg(short, long)]
    pub keyword: Option<String>,
    /// Difficulty filter: 0=none, 1=入门, 2=普及-, 3=普及/提高-, 4=普及+/提高, 5=提高+/省选-, 6=省选/NOI-, 7=NOI/NOI+/CTSC
    #[arg(short, long)]
    pub difficulty: Option<i64>,
    /// Problem type: P (main), B (beginner), T (team), etc.
    #[arg(short, long, default_value = "P")]
    pub ptype: Option<String>,
    /// Tag ID filter (use 'luogu tags' to list).
    #[arg(short, long)]
    pub tag: Option<String>,
    /// Page number.
    #[arg(short, long, default_value_t = 1)]
    pub page: i64,
    /// Order by: pid, title, difficulty, totalSubmit, totalAccepted.
    #[arg(long)]
    pub order_by: Option<String>,
    /// Order: asc or desc.
    #[arg(long)]
    pub order: Option<String>,
    /// Limit results shown per page.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

#[derive(Args, Debug)]
pub struct TrainingArgs {
    /// Training set ID.
    pub id: Option<i64>,
    /// Root folder to store problems.
    #[arg(long, default_value = "problem")]
    pub base_dir: PathBuf,
    /// Overwrite existing problem files.
    #[arg(long)]
    pub force: bool,
    /// List public training sets (with keyword filter).
    #[arg(long)]
    pub list: bool,
    /// Page for listing.
    #[arg(long, default_value_t = 1)]
    pub page: i64,
    /// Search keyword for listing.
    #[arg(long)]
    pub keyword: Option<String>,
}

#[derive(Args, Debug)]
pub struct ContestArgs {
    /// Contest ID.
    pub id: Option<i64>,
    /// Root folder to store problems.
    #[arg(long, default_value = "problem")]
    pub base_dir: PathBuf,
    /// Overwrite existing problem files.
    #[arg(long)]
    pub force: bool,
    /// List contests.
    #[arg(long)]
    pub list: bool,
    /// Page for listing.
    #[arg(long, default_value_t = 1)]
    pub page: i64,
    /// Filter by contest name for listing.
    #[arg(long)]
    pub name: Option<String>,
    /// Filter by contest method: 1=OI, 2=ICPC, 3=Ludo, 4=IOI.
    #[arg(long)]
    pub method: Option<i64>,
    /// Filter by contest visibility.
    #[arg(long)]
    pub public: Option<i64>,
}

#[derive(Args, Debug)]
pub struct UserArgs {
    /// User UID or name.
    pub uid_or_name: Option<String>,
    /// Search for users by keyword.
    #[arg(short, long)]
    pub search: Option<String>,
    /// Show practice details (passed/submitted problems).
    #[arg(short, long)]
    pub practice: bool,
}

#[derive(Args, Debug)]
pub struct SolutionArgs {
    /// Problem ID.
    pub pid: String,
    /// Page number.
    #[arg(short, long, default_value_t = 1)]
    pub page: i64,
    /// Save solutions to problem directory.
    #[arg(short, long)]
    pub save: bool,
    /// Root folder containing problem directories.
    #[arg(long, default_value = "problem")]
    pub base_dir: PathBuf,
    /// Show full solution content.
    #[arg(short, long)]
    pub full: bool,
    /// Output solution index to show (1-based).
    #[arg(short, long)]
    pub index: Option<usize>,
}

#[derive(Args, Debug)]
pub struct SubmitArgs {
    /// Problem ID to submit to.
    pub pid: String,
    /// Source file path (default: problem/PID/main.cpp).
    #[arg(short, long)]
    pub source: Option<String>,
    /// Root folder containing problem directories.
    #[arg(long, default_value = "problem")]
    pub base_dir: PathBuf,
    /// Language ID (default: 12 for C++17 with O2).
    #[arg(short, long, default_value_t = 12)]
    pub lang: i64,
    /// Disable O2 optimization.
    #[arg(long)]
    pub no_o2: bool,
    /// CSRF token (if not provided, will be fetched automatically).
    #[arg(long)]
    pub csrf: Option<String>,
}

#[derive(Args, Debug)]
pub struct RecordArgs {
    /// Record ID to view details.
    pub id: Option<i64>,
    /// Filter by problem ID.
    #[arg(short, long)]
    pub pid: Option<String>,
    /// Filter by user UID.
    #[arg(short, long)]
    pub user: Option<String>,
    /// Filter by status: 0=WA, 1=TLE, 2=MLE, 3=RE, 4=CE, 6=UKE, 7=AC, 8=noip, 11=JGF, 12=FLE
    #[arg(short, long)]
    pub status: Option<i64>,
    /// Page number.
    #[arg(short, long, default_value_t = 1)]
    pub page: i64,
    /// Show source code for a record.
    #[arg(short, long)]
    pub source: bool,
}

#[derive(Args, Debug)]
pub struct RankArgs {
    /// Ranking type: gu (咕值) or elo (等级分).
    #[arg(short, long = "type", default_value = "gu")]
    pub rtype: String,
    /// Page number.
    #[arg(short, long, default_value_t = 1)]
    pub page: i64,
    /// Limit results shown.
    #[arg(long, default_value_t = 30)]
    pub limit: usize,
}