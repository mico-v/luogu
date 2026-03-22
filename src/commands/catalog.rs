use crate::cli::CatalogArgs;
use crate::storage;
use anyhow::Result;

pub fn run(args: CatalogArgs) -> Result<()> {
    if args.history {
        let logs = storage::read_judge_logs(args.limit)?;
        for entry in logs {
            if let Some(ref pid) = args.pid {
                if &entry.pid != pid {
                    continue;
                }
            }
            println!(
                "{} | {} | {} | {}/{}",
                entry.timestamp, entry.pid, entry.status, entry.pass_count, entry.test_count
            );
        }
        return Ok(());
    }

    let map = storage::load_problem_map()?;
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
        println!(
            "{} | {} | {} | time {} | mem {}",
            pid, rec.title, rec.difficulty_label, tl, ml
        );
    }
    Ok(())
}
