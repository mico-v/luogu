use crate::cli::CatalogArgs;
use crate::storage;
use anyhow::Result;
use colored::Colorize;

pub fn run(args: CatalogArgs) -> Result<()> {
    if args.history {
        let logs = storage::read_judge_logs(args.limit)?;
        if logs.is_empty() {
            println!("{} No judge history yet", "ℹ".cyan());
            return Ok(());
        }
        
        println!();
        println!("{}", "━━━━━━━━━━━ Judge History ━━━━━━━━━━━".cyan().bold());
        for entry in logs {
            if let Some(ref pid) = args.pid {
                if &entry.pid != pid {
                    continue;
                }
            }
            let status_colored = match entry.status.as_str() {
                "AC" => entry.status.green(),
                "FAILED" => entry.status.red(),
                "COMPILE_ERROR" => entry.status.red(),
                _ => entry.status.yellow(),
            };
            
            let time_str = entry.timestamp.split('T').next().unwrap_or(&entry.timestamp);
            println!("{} | {} | {} | {}/{} {}",
                time_str.cyan(),
                entry.pid.bold(),
                status_colored,
                entry.pass_count.to_string().green(),
                entry.test_count,
                if entry.success { "✓".green() } else { "✗".red() }
            );
        }
        return Ok(());
    }

    let map = storage::load_problem_map()?;
    if map.is_empty() {
        println!("{} No problems fetched yet. Try: luogu fetch <pid>", "ℹ".cyan());
        return Ok(());
    }

    println!();
    println!("{}", "━━━━━━━━ Downloaded Problems ━━━━━━━━".cyan().bold());
    println!("{}", format!("{:<10} | {:<30} | {:<15} | {:<10} | {:<10}", "PID", "Title", "Difficulty", "Time", "Memory").cyan());
    println!("{}", "─".repeat(85).cyan());
    
    for (pid, rec) in map {
        if let Some(ref filter_pid) = args.pid {
            if &pid != filter_pid {
                continue;
            }
        }
        let tl = rec
            .time_limit_ms
            .map(|v| format!("{:.2}s", v as f64 / 1000.0))
            .unwrap_or_else(|| "n/a".to_string());
        let ml = rec
            .memory_limit_kb
            .map(|v| format!("{:.2}MB", v as f64 / 1024.0))
            .unwrap_or_else(|| "n/a".to_string());
        
        let title = if rec.title.len() > 28 {
            format!("{}...", &rec.title[..25])
        } else {
            rec.title.clone()
        };
        
        let difficulty_color = match rec.difficulty_label.as_str() {
            s if s.contains("入门") => s.cyan(),
            s if s.contains("普及") => s.green(),
            s if s.contains("提高") => s.yellow(),
            s if s.contains("省选") => s.magenta(),
            s if s.contains("NOI") => s.red(),
            s => s.normal(),
        };
        
        println!("{:<10} | {:<30} | {:<15} | {:<10} | {:<10}",
            pid.cyan(),
            title,
            difficulty_color,
            tl.green(),
            ml.green()
        );
    }
    println!();
    Ok(())
}
