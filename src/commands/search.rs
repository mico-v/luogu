use crate::cli::SearchArgs;
use crate::net;
use anyhow::Result;
use colored::Colorize;

fn difficulty_label(d: Option<i64>) -> String {
    match d.unwrap_or(-1) {
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

fn difficulty_color(label: &str) -> colored::ColoredString {
    match label {
        s if s.contains("入门") => s.cyan(),
        s if s.contains("普及") => s.green(),
        s if s.contains("提高") => s.yellow(),
        s if s.contains("省选") => s.magenta(),
        s if s.contains("NOI") => s.red(),
        s => s.normal(),
    }
}

pub fn run(args: SearchArgs) -> Result<()> {
    let data = net::fetch_problem_list(
        args.page,
        args.keyword.as_deref(),
        args.difficulty,
        args.ptype.as_deref(),
        args.tag.as_deref(),
        args.order_by.as_deref(),
        args.order.as_deref(),
    )?;

    let problems = &data.result;
    let total = data.count;
    let per_page = data.perPage.unwrap_or(20);

    if problems.is_empty() {
        println!("{} No problems found.", "ℹ".cyan());
        return Ok(());
    }

    let pages = (total as f64 / per_page as f64).ceil() as i64;

    println!();
    println!(
        "{} Found {} problems (page {}/{})",
        "🔍".normal(),
        total.to_string().cyan().bold(),
        args.page.to_string().cyan(),
        pages.to_string().cyan()
    );
    println!(
        "{}",
        format!(
            "{:<10} | {:<30} | {:<15} | {:<10} | {:<10} | {:<6}",
            "PID", "Title", "Difficulty", "Submits", "AC", "Type"
        )
        .cyan()
    );
    println!("{}", "─".repeat(95).cyan());

    for p in problems {
        let label = difficulty_label(p.difficulty);
        let title = if p.title.chars().count() > 27 {
            format!("{}...", p.title.chars().take(24).collect::<String>())
        } else {
            p.title.clone()
        };
        let sub = p.totalSubmit.unwrap_or(0).to_string();
        let ac = p.totalAccepted.unwrap_or(0).to_string();
        let ptype = p.flag.map(|f| match f {
            0 => "P",
            1 => "B",
            2 => "T",
            _ => "?",
        }).unwrap_or("?");

        println!(
            "{:<10} | {:<30} | {:<15} | {:<10} | {:<10} | {:<6}",
            p.pid.cyan().bold(),
            title,
            difficulty_color(&label),
            sub.yellow(),
            ac.green(),
            ptype,
        );
    }

    println!();
    println!(
        "{} Use --page {} to see next page",
        "💡".cyan(),
        args.page + 1
    );

    Ok(())
}