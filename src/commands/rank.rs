use crate::cli::RankArgs;
use crate::net;
use anyhow::Result;
use colored::Colorize;

pub fn run(args: RankArgs) -> Result<()> {
    match args.rtype.to_lowercase().as_str() {
        "gu" | "咕值" => gu_ranking(args.page, args.limit),
        "elo" | "等级分" => elo_ranking(args.page, args.limit),
        _ => {
            println!("{} Unknown ranking type: {}. Use 'gu' or 'elo'.", "✗".red(), args.rtype);
            Ok(())
        }
    }
}

fn gu_ranking(page: i64, limit: usize) -> Result<()> {
    eprintln!("{} Fetching gu ranking...", "🏆".cyan());

    let data = net::fetch_gu_ranking(page)?;
    let entries = &data.ranking.result;

    if entries.is_empty() {
        println!("{} No ranking data found.", "ℹ".cyan());
        return Ok(());
    }

    println!();
    println!("{} Gu Rating Ranking (page {})", "🏆".cyan().bold(), page.to_string().cyan());
    println!(
        "{}",
        format!("{:<6} | {:<20} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8}",
            "Rank", "User", "Total", "Rating", "Social", "Basic", "Contest", "Practice"
        ).cyan()
    );
    println!("{}", "─".repeat(95).cyan());

    let base = (page - 1) * 20;
    for (i, entry) in entries.iter().enumerate() {
        let rank = base + i as i64 + 1;
        let name = if entry.user.name.len() > 18 {
            format!("{}...", entry.user.name.chars().take(15).collect::<String>())
        } else {
            entry.user.name.clone()
        };
        let colored_name = match entry.user.color.as_str() {
            "Gray" => name.white().dimmed(),
            "Blue" => name.blue(),
            "Green" => name.green(),
            "Orange" => name.yellow(),
            "Red" => name.red(),
            "Purple" => name.magenta(),
            _ => name.normal(),
        };

        println!(
            "{:<6} | {:<20} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8}",
            rank.to_string().cyan(),
            colored_name,
            entry.rating.to_string().cyan().bold(),
            entry.scores.rating.to_string().green(),
            entry.scores.social.to_string().yellow(),
            entry.scores.basic.to_string().blue(),
            entry.scores.contest.to_string().magenta(),
            entry.scores.practice.to_string().cyan(),
        );
    }

    if limit >= 20 {
        println!();
        println!("{} Use --page {} to see more", "💡".cyan(), page + 1);
    }

    Ok(())
}

fn elo_ranking(page: i64, limit: usize) -> Result<()> {
    eprintln!("{} Fetching elo ranking...", "📈".cyan());

    let data = net::fetch_elo_ranking(page)?;
    let entries = &data.ranking.result;

    if entries.is_empty() {
        println!("{} No ranking data found.", "ℹ".cyan());
        return Ok(());
    }

    println!();
    println!("{} Elo Rating Ranking (page {})", "📈".cyan().bold(), page.to_string().cyan());
    println!(
        "{}",
        format!("{:<6} | {:<20} | {:<8} | {:<8}",
            "Rank", "User", "Rating", "Change"
        ).cyan()
    );
    println!("{}", "─".repeat(55).cyan());

    let base = (page - 1) * 20;
    for (i, entry) in entries.iter().enumerate() {
        let rank = base + i as i64 + 1;
        let name = if entry.user.name.len() > 18 {
            format!("{}...", entry.user.name.chars().take(15).collect::<String>())
        } else {
            entry.user.name.clone()
        };
        let colored_name = match entry.user.color.as_str() {
            "Gray" => name.white().dimmed(),
            "Blue" => name.blue(),
            "Green" => name.green(),
            "Orange" => name.yellow(),
            "Red" => name.red(),
            "Purple" => name.magenta(),
            _ => name.normal(),
        };

        let diff = entry.prevDiff.map(|d| {
            if d >= 0 {
                format!("+{}", d).green().to_string()
            } else {
                d.to_string().red().to_string()
            }
        }).unwrap_or_else(|| "N/A".to_string());

        println!(
            "{:<6} | {:<20} | {:<8} | {:<8}",
            rank.to_string().cyan(),
            colored_name,
            entry.rating.to_string().cyan().bold(),
            diff,
        );
    }

    if limit >= 20 {
        println!();
        println!("{} Use --page {} to see more", "💡".cyan(), page + 1);
    }

    Ok(())
}