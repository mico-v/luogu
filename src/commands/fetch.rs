use crate::cli::FetchArgs;
use crate::models::ProblemRecord;
use crate::{net, storage};
use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use std::collections::BTreeMap;
use std::fs;

fn difficulty_label(level: Option<i32>) -> String {
    match level.unwrap_or(0) {
        0 => "暂无评定",
        1 => "入门",
        2 => "普及-",
        3 => "普及/提高-",
        4 => "普及+/提高",
        5 => "提高+/省选-",
        6 => "省选/NOI-",
        7 => "NOI/NOI+/CTSC",
        _ => "未知",
    }
    .to_string()
}

pub fn run(args: FetchArgs) -> Result<()> {
    let pid = args.pid.trim().to_uppercase();
    eprintln!("{}", "Fetching problem...".cyan());
    let problem = net::fetch_problem(&pid)?;
    let problem_path = storage::problem_dir(&args.base_dir, &problem.pid);

    if problem_path.exists() && !args.force {
        println!("{} Problem directory exists: {}", "⚠".yellow(), problem_path.display());
        println!("{} Use --force to overwrite generated files.", "💡".cyan());
        return Ok(());
    }

    fs::create_dir_all(&problem_path).with_context(|| format!("create {}", problem_path.display()))?;
    fs::write(problem_path.join("T.md"), problem.markdown.as_bytes())?;
    if !problem_path.join("main.cpp").exists() || args.force {
        fs::write(
            problem_path.join("main.cpp"),
            b"#include <iostream>\nusing namespace std;\n\nint main() {\n    ios::sync_with_stdio(false);\n    cin.tie(nullptr);\n\n    return 0;\n}\n",
        )?;
    }
    for (idx, (input, output)) in problem.samples.iter().enumerate() {
        let i = idx + 1;
        fs::write(problem_path.join(format!("sample{i}.in")), input)?;
        fs::write(problem_path.join(format!("sample{i}.out")), output)?;
    }

    let mut map: BTreeMap<String, ProblemRecord> = storage::load_problem_map()?;
    let record = ProblemRecord {
        pid: problem.pid.clone(),
        title: problem.title.clone(),
        difficulty: problem.difficulty,
        difficulty_label: difficulty_label(problem.difficulty),
        time_limit_ms: problem.limits_time_ms,
        memory_limit_kb: problem.limits_memory_kb,
        tags: problem.tags,
        fetched_at: Utc::now().to_rfc3339(),
        url: format!("https://www.luogu.com.cn/problem/{}", problem.pid),
    };
    map.insert(problem.pid.clone(), record);
    storage::save_problem_map(&map)?;

    println!();
    println!("{} {} {} {}", "✓".green(), problem.pid, problem.title, format!("[{}]", difficulty_label(problem.difficulty)).cyan());
    println!("{} {}", "📁".normal(), problem_path.display());
    println!("{} {} samples | {} time | {} memory", "📊".normal(), problem.samples.len(), 
        format!("{}ms", problem.limits_time_ms.unwrap_or(0)).green(), 
        format!("{}KB", problem.limits_memory_kb.unwrap_or(0)).green());
    Ok(())
}
