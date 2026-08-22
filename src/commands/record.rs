use crate::cli::RecordArgs;
use crate::net;
use crate::models::RecordListParams;
use anyhow::Result;
use colored::Colorize;

fn status_name(status: i64) -> (String, colored::ColoredString) {
    match status {
        0 => ("WA".to_string(), "WA".red()),
        1 => ("TLE".to_string(), "TLE".yellow()),
        2 => ("MLE".to_string(), "MLE".magenta()),
        3 => ("RE".to_string(), "RE".red()),
        4 => ("CE".to_string(), "CE".cyan()),
        6 => ("UKE".to_string(), "UKE".red()),
        7 => ("AC".to_string(), "AC".green()),
        8 => ("noip".to_string(), "noip".normal()),
        11 => ("JGF".to_string(), "JGF".normal()),
        12 => ("FLE".to_string(), "FLE".yellow()),
        _ => (format!("STT({})", status), format!("STT({})", status).normal()),
    }
}

fn language_name(lang: i64) -> &'static str {
    match lang {
        0 => "C",
        1 => "C++11",
        2 => "C++14",
        3 => "C++17",
        4 => "C++20",
        5 => "C++23",
        6 => "Python 2",
        7 => "Python 3",
        8 => "Java",
        9 => "Pascal",
        10 => "C++98",
        11 => "C++11(O2)",
        12 => "C++14(O2) / C++17(O2)",
        13 => "C++20(O2)",
        14 => "C++23(O2)",
        _ => "unknown",
    }
}

pub fn run(args: RecordArgs) -> Result<()> {
    // If an ID is provided, show details
    if let Some(rid) = args.id {
        return show_record_detail(rid, args.source);
    }

    // Otherwise, list records
    let params = RecordListParams {
        page: Some(args.page),
        pid: args.pid.clone(),
        user: args.user.clone(),
        status: args.status,
        language: None,
    };

    let data = net::fetch_record_list(&params)?;
    let records = &data.records.result;
    let total = data.records.count;

    if records.is_empty() {
        println!("{} No records found.", "ℹ".cyan());
        return Ok(());
    }

    println!();
    println!(
        "{} {} records (page {})",
        "📋".normal(),
        total.to_string().cyan().bold(),
        args.page.to_string().cyan()
    );
    println!(
        "{}",
        format!("{:<10} | {:<10} | {:<6} | {:<8} | {:<7} | {:<7} | {:<20} | {:<10}",
            "ID", "PID", "Status", "Score", "Time", "Memory", "Submit Time", "Language"
        ).cyan()
    );
    println!("{}", "─".repeat(95).cyan());

    for r in records {
        let (_status_short, status_colored) = status_name(r.status);
        let time_str = r.time.map(|t| format!("{}ms", t)).unwrap_or_else(|| "?".to_string());
        let mem_str = r.memory.map(|m| format!("{}KB", m)).unwrap_or_else(|| "?".to_string());
        let score_str = r.score.map(|s| format!("{}/100", s)).unwrap_or_else(|| "?".to_string());
        let submit_time = chrono::DateTime::from_timestamp(r.submitTime, 0)
            .map(|t| t.format("%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "?".to_string());

        println!(
            "{:<10} | {:<10} | {:<6} | {:<8} | {:<7} | {:<7} | {:<20} | {:<10}",
            r.id.to_string().cyan(),
            r.problem.pid.cyan().bold(),
            status_colored,
            score_str,
            time_str,
            mem_str,
            submit_time.cyan(),
            language_name(r.language),
        );
    }

    println!();
    if total > args.page * 20 {
        println!("{} Use --page {} to see more", "💡".cyan(), args.page + 1);
    }
    println!("{} Use `luogu record <id>` to view details", "💡".cyan());

    Ok(())
}

fn show_record_detail(rid: i64, show_source: bool) -> Result<()> {
    eprintln!("{} Fetching record {}...", "📋".cyan(), rid.to_string().bold());

    let data = net::fetch_record_detail(rid)?;
    let record = &data.record;

    println!();
    println!(
        "{} Record #{}",
        "📋".normal(),
        rid.to_string().cyan().bold()
    );
    println!("{}", "─".repeat(50).cyan());

    let (_status_short, status_colored) = status_name(record.status);
    println!("  Problem: {} ({})", record.problem.pid.cyan().bold(), record.problem.title);
    println!("  Status:  {}", status_colored);
    println!("  Score:   {}", record.score.map(|s| s.to_string()).unwrap_or_else(|| "?".to_string()).cyan());
    println!("  Time:    {}", record.time.map(|t| format!("{}ms", t)).unwrap_or_else(|| "?".to_string()).cyan());
    println!("  Memory:  {}", record.memory.map(|m| format!("{}KB", m)).unwrap_or_else(|| "?".to_string()).cyan());
    println!("  Language: {}", language_name(record.language));
    println!("  O2:      {}", if record.enableO2 { "enabled".green() } else { "disabled".yellow() });

    let submit_time = chrono::DateTime::from_timestamp(record.submitTime, 0)
        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "?".to_string());
    println!("  Submit:  {}", submit_time.cyan());

    // Compile info
    if let Some(ref detail) = record.detail {
        if let Some(ref compile) = detail.compileResult {
            println!();
            println!("  {} Compilation:", "⚙".cyan().bold());
            if compile.success {
                println!("    Compilation: {}", "OK".green());
            } else {
                println!("    Compilation: {}", "FAILED".red());
                if let Some(ref msg) = compile.message {
                    if !msg.is_empty() {
                        println!("    {}", msg.red());
                    }
                }
            }
        }

        // Judge info
        if let Some(ref judge) = detail.judgeResult {
            println!();
            println!("  {} Judge Result:", "📊".cyan().bold());
            println!("    Score: {}", judge.score.to_string().cyan());
            println!("    Time:  {}ms", judge.time.to_string().cyan());
            println!("    Memory: {}KB", judge.memory.to_string().cyan());
            println!("    Cases: {}", judge.finishedCaseCount.to_string().yellow());
        }
    }

    // Show source code
    if show_source {
        if let Some(ref source) = record.sourceCode {
            println!();
            println!("  {} Source Code:", "📄".cyan().bold());
            println!("{}", "─".repeat(50).cyan());
            for line in source.lines() {
                println!("  {}", line);
            }
            println!("{}", "─".repeat(50).cyan());
        } else {
            println!("  {} Source code not available.", "ℹ".cyan());
        }
    }

    println!();
    println!("  URL: {}", format!("https://www.luogu.com.cn/record/{}", rid).cyan().underline());

    Ok(())
}