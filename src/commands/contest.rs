use crate::cli::ContestArgs;
use crate::commands::training;
use crate::models::ProblemRecord;
use crate::{net, storage};
use anyhow::Result;
use chrono::Utc;
use colored::Colorize;

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

fn method_name(m: i64) -> &'static str {
    match m {
        1 => "OI",
        2 => "ICPC",
        3 => "乐多",
        4 => "IOI",
        _ => "未知",
    }
}

pub fn run(args: ContestArgs) -> Result<()> {
    if args.list {
        return list_contests(&args);
    }

    let cid = args.id.ok_or_else(|| anyhow::anyhow!("contest ID required. Use --list to list contests, or provide an ID."))?;

    eprintln!("{} Fetching contest {}...", "🏁".cyan(), cid.to_string().bold());

    let data = net::fetch_contest(cid)?;
    let contest = data.contest;
    let problems = data.contestProblems.unwrap_or_default();

    println!();
    println!(
        "{} {} {}",
        "🏁".normal(),
        contest.name.bold(),
        format!("#{}", cid).cyan()
    );

    let start = chrono::DateTime::from_timestamp(contest.startTime, 0)
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let end = chrono::DateTime::from_timestamp(contest.endTime, 0)
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!(
        "  {} {} → {}  |  {}  |  {} participants",
        "📅".normal(),
        start.cyan(),
        end.cyan(),
        method_name(contest.method).cyan(),
        contest.totalParticipants.to_string().yellow()
    );

    if let Some(ref desc) = contest.description {
        if !desc.is_empty() {
            println!("  {}", desc.cyan());
        }
    }

    println!("{}", "─".repeat(60).cyan());

    if problems.is_empty() {
        println!("{} No problems in this contest.", "ℹ".cyan());
        return Ok(());
    }

    let base_dir = &args.base_dir;
    let mut map = storage::load_problem_map()?;
    let mut success = 0usize;
    let mut skipped = 0usize;

    for entry in &problems {
        let pid = &entry.problem.pid;
        let title = &entry.problem.title;
        let problem_path = storage::problem_dir(base_dir, pid);

        if problem_path.exists() && !args.force {
            println!("  {} {} {} {}", "⏭".yellow(), pid.cyan(), title, "(skipped)".yellow());
            skipped += 1;
            if !map.contains_key(pid) {
                let record = ProblemRecord {
                    pid: pid.clone(),
                    title: title.clone(),
                    difficulty: entry.problem.difficulty.map(|v| v as i32),
                    difficulty_label: difficulty_label(entry.problem.difficulty.map(|v| v as i32)),
                    time_limit_ms: None,
                    memory_limit_kb: None,
                    tags: vec![],
                    fetched_at: Utc::now().to_rfc3339(),
                    url: format!("https://www.luogu.com.cn/problem/{pid}"),
                };
                map.insert(pid.clone(), record);
            }
            continue;
        }

        match net::fetch_problem(pid) {
            Ok(problem) => {
                if let Err(e) = training::save_problem_files(&problem, base_dir, args.force) {
                    eprintln!("  {} {} {}: {}", "✗".red(), pid.cyan(), title, e);
                    continue;
                }
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
                println!("  {} {} {} {}", "✓".green(), pid.cyan(), title, format!("[{}]", difficulty_label(problem.difficulty)).cyan());
                success += 1;
            }
            Err(e) => {
                eprintln!("  {} {} {}: {}", "✗".red(), pid.cyan(), title, e);
            }
        }
    }

    storage::save_problem_map(&map)?;

    println!();
    println!(
        "{} {} problems downloaded, {} skipped (total {})",
        "📊".normal(),
        success.to_string().green(),
        skipped.to_string().yellow(),
        problems.len().to_string().cyan()
    );

    Ok(())
}

fn list_contests(args: &ContestArgs) -> Result<()> {
    let data = net::fetch_contest_list(
        args.page,
        args.name.as_deref(),
        args.method,
        args.public,
    )?;

    let contests = &data.contests.result;
    if contests.is_empty() {
        println!("{} No contests found.", "ℹ".cyan());
        return Ok(());
    }

    println!();
    println!("{} Contests:", "🏁".cyan().bold());
    println!(
        "{}",
        format!("{:<8} | {:<35} | {:<15} | {:<8} | {:<10}", "ID", "Name", "Time", "Method", "Problems").cyan()
    );
    println!("{}", "─".repeat(88).cyan());

    for c in contests {
        let name = if c.name.len() > 32 {
            format!("{}...", c.name.chars().take(29).collect::<String>())
        } else {
            c.name.clone()
        };
        let start = chrono::DateTime::from_timestamp(c.startTime, 0)
            .map(|t| t.format("%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "?".to_string());
        let end = chrono::DateTime::from_timestamp(c.endTime, 0)
            .map(|t| t.format("%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "?".to_string());

        println!(
            "{:<8} | {:<35} | {:<15} | {:<8} | {:<10}",
            c.id.to_string().cyan().bold(),
            name,
            format!("{} ~ {}", start.cyan(), end.cyan()),
            method_name(c.method),
            c.problemCount.to_string().green(),
        );
    }

    println!();
    if data.contests.count > args.page * 20 {
        println!("{} Use --page {} to see more", "💡".cyan(), args.page + 1);
    }

    Ok(())
}