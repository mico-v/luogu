use crate::cli::TrainingArgs;
use crate::models::ProblemRecord;
use crate::{net, storage};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use colored::Colorize;
use std::path::Path;

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

pub fn run(args: TrainingArgs) -> Result<()> {
    if args.list {
        return list_trainings(&args);
    }

    let tid = args.id.ok_or_else(|| anyhow::anyhow!("training ID required. Use --list to list training sets, or provide an ID."))?;
    fetch_training_set(tid, &args.base_dir, args.force)
}

pub fn fetch_training_set(tid: i64, base_dir: &Path, force: bool) -> Result<()> {
    eprintln!("{} Fetching training set {}...", "📚".cyan(), tid.to_string().bold());

    let data = net::fetch_training(tid)?;
    let training = data.training;
    let problems = &training.problems;

    println!();
    println!(
        "{} {} {} ({} problems)",
        "📚".normal(),
        training.name.bold(),
        format!("#{}", tid).cyan(),
        problems.len().to_string().green()
    );
    if !training.description.is_empty() {
        println!("{}", training.description.cyan());
    }
    println!("{}", "─".repeat(60).cyan());

    if problems.is_empty() {
        println!("{} No problems in this training set.", "ℹ".cyan());
        return Ok(());
    }

    let mut map = storage::load_problem_map()?;
    let mut success = 0usize;
    let mut skipped = 0usize;

    for entry in problems {
        let pid = &entry.pid;
        let title = &entry.title;
        let problem_path = storage::problem_dir(base_dir, pid);

        if problem_path.exists() && !force {
            println!("  {} {} {} {}", "⏭".yellow(), pid.cyan(), title, "(skipped, exists)".yellow());
            skipped += 1;
            if !map.contains_key(pid) {
                // Still record metadata if not in map
                let record = ProblemRecord {
                    pid: pid.clone(),
                    title: title.clone(),
                    difficulty: entry.difficulty.map(|v| v as i32),
                    difficulty_label: difficulty_label(entry.difficulty.map(|v| v as i32)),
                    time_limit_ms: None,
                    memory_limit_kb: None,
                    tags: entry.tags.clone(),
                    fetched_at: Utc::now().to_rfc3339(),
                    url: format!("https://www.luogu.com.cn/problem/{pid}"),
                };
                map.insert(pid.clone(), record);
            }
            continue;
        }

        // Fetch and save the problem
        match net::fetch_problem(pid) {
            Ok(problem) => {
                if let Err(e) = save_problem_files(&problem, base_dir, force) {
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
                println!("  {} {} {} {}", "✓".green(), pid.cyan(), title, difficulty_label(problem.difficulty).cyan());
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

fn list_trainings(args: &TrainingArgs) -> Result<()> {
    let data = net::fetch_training_list(
        args.page,
        args.keyword.as_deref(),
        None,
    )?;

    let trainings = &data.trainings.result;
    if trainings.is_empty() {
        println!("{} No training sets found.", "ℹ".cyan());
        return Ok(());
    }

    println!();
    println!("{} Public training sets:", "📚".cyan().bold());
    println!(
        "{}",
        format!("{:<8} | {:<40} | {:<10} | {:<10}", "ID", "Name", "Problems", "Marks").cyan()
    );
    println!("{}", "─".repeat(78).cyan());

    for t in trainings {
        let name = if t.name.len() > 37 {
            format!("{}...", t.name.chars().take(34).collect::<String>())
        } else {
            t.name.clone()
        };
        println!(
            "{:<8} | {:<40} | {:<10} | {:<10}",
            t.id.to_string().cyan().bold(),
            name,
            t.problemCount.to_string().green(),
            t.markCount.to_string().yellow(),
        );
    }

    println!();
    if data.trainings.count > args.page * 20 {
        println!("{} Use --page {} to see more", "💡".cyan(), args.page + 1);
    }

    Ok(())
}

/// Save problem files (called both from fetch::run and training::run)
pub fn save_problem_files(
    problem: &crate::models::ParsedProblem,
    base_dir: &Path,
    force: bool,
) -> Result<()> {
    let problem_path = storage::problem_dir(base_dir, &problem.pid);
    if !problem_path.exists() {
        std::fs::create_dir_all(&problem_path)
            .with_context(|| format!("create {}", problem_path.display()))?;
    }
    std::fs::write(problem_path.join("T.md"), problem.markdown.as_bytes())?;
    let cfg = crate::config::load()?;

    let source_file = cfg.source();
    let source_stem = Path::new(source_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("invalid source filename: {source_file}"))?;
    let source_path = problem_path.join(source_file);
    if !source_path.exists() || force {
        // No template configured means no source file is generated by default.
        if let Some(template) = cfg.template() {
            std::fs::write(&source_path, template)?;
        }
    }

    // Testcases follow the configured layout (default: testcases/main.<n>.in|.ans).
    if !problem.samples.is_empty() {
        let testcases = cfg.testcases();
        let tc_dir = problem_path.join(testcases.directory());
        std::fs::create_dir_all(&tc_dir)
            .with_context(|| format!("create {}", tc_dir.display()))?;
        for (idx, (input, output)) in problem.samples.iter().enumerate() {
            let in_path = testcases.input_path(&tc_dir, source_stem, idx);
            let out_path = testcases.output_path(&tc_dir, source_stem, idx);
            std::fs::write(&in_path, input)?;
            std::fs::write(&out_path, output)?;
        }
    }
    Ok(())
}
