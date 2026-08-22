use crate::cli::UserArgs;
use crate::net;
use anyhow::Result;
use colored::Colorize;

fn user_color(s: &str, color: &str) -> colored::ColoredString {
    match color {
        "Gray" => s.white().dimmed(),
        "Blue" => s.blue(),
        "Green" => s.green(),
        "Orange" => s.yellow(),
        "Red" => s.red(),
        "Purple" => s.magenta(),
        "Cheater" => s.red().strikethrough(),
        _ => s.normal(),
    }
}

pub fn run(args: UserArgs) -> Result<()> {
    // Search by keyword
    if let Some(ref keyword) = args.search {
        return search_user(keyword);
    }

    // Look up by UID or name
    let input = match args.uid_or_name {
        Some(ref s) => s.clone(),
        None => {
            println!("{} Please provide a UID or name, or use --search.", "ℹ".cyan());
            return Ok(());
        }
    };

    // Try to parse as UID first
    let uid: i64 = match input.parse() {
        Ok(id) => id,
        Err(_) => {
            // Search by name to find UID
            let result = net::search_user(&input)?;
            let users: Vec<_> = result.users.into_iter().flatten().collect();
            if users.is_empty() {
                println!("{} User not found: {}", "✗".red(), input);
                return Ok(());
            }
            if users.len() == 1 {
                users[0].uid
            } else {
                println!("{} Multiple users found:", "ℹ".cyan());
                for u in &users {
                    let colored = user_color(&u.name, &u.color);
                    println!("  {} {} (UID: {})", "👤".normal(), colored, u.uid.to_string().cyan());
                }
                return Ok(());
            }
        }
    };

    let data = net::fetch_user(uid)?;
    let user = data.user;

    println!();
    println!(
        "{} {} {}",
        "👤".normal(),
        user_color(&user.name, &user.color).bold(),
        format!("#{}", uid).cyan()
    );

    if let Some(ref slogan) = user.slogan {
        if !slogan.is_empty() {
            println!("  {}", slogan.italic().cyan());
        }
    }
    if let Some(ref badge) = user.badge {
        println!("  {}", badge.yellow());
    }

    println!();
    println!("  {}  Level: {}", "🏆".normal(), user.ccfLevel.to_string().cyan());
    println!("  {}  Ranking: {}", "📊".normal(), user.ranking.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()).cyan());
    println!("  {}  Followers: {} | Following: {}", "👥".normal(),
        user.followerCount.to_string().green(),
        user.followingCount.to_string().yellow());
    println!("  {}  Register: {}", "📅".normal(),
        chrono::DateTime::from_timestamp(user.registerTime, 0)
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "?".to_string()).cyan());

    if user.isAdmin {
        println!("  {}  Admin", "🔰".normal());
    }
    if user.isBanned {
        println!("  {}  BANNED", "🚫".red());
    }

    // Practice stats
    println!();
    println!("  {}  Practice:", "📝".cyan().bold());
    println!("      Passed problems: {}", user.passedProblemCount.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string()).green());
    println!("      Submitted problems: {}", user.submittedProblemCount.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string()).yellow());

    // Gu rating
    let gu = &data.gu;
    println!();
    println!("  {}  Gu Rating: {} (time: {})", "⭐".normal(),
        gu.rating.to_string().cyan().bold(),
        chrono::DateTime::from_timestamp(gu.time, 0)
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "?".to_string()).cyan());
    println!(
        "      Rating:{}  Social:{}  Basic:{}  Contest:{}  Practice:{}  Prize:{}",
        format!(" {:>5}", gu.scores.rating).green(),
        format!(" {:>5}", gu.scores.social).yellow(),
        format!(" {:>5}", gu.scores.basic).blue(),
        format!(" {:>5}", gu.scores.contest).magenta(),
        format!(" {:>5}", gu.scores.practice).cyan(),
        format!(" {:>5}", gu.scores.prize).red(),
    );

    // Elo rating
    if let Some(latest_elo) = data.elo.iter().find(|e| e.latest) {
        println!();
        println!("  {}  Elo Rating: {}", "📈".normal(), latest_elo.rating.to_string().cyan().bold());
        if let Some(diff) = latest_elo.prevDiff {
            if diff >= 0 {
                println!("      Change: +{}", diff.to_string().green());
            } else {
                println!("      Change: {}", diff.to_string().red());
            }
        }
        if let Some(uc) = latest_elo.userCount {
            println!("      Users: {}", uc.to_string().yellow());
        }
    }

    if let Some(ref intro) = user.introduction {
        if !intro.is_empty() {
            println!();
            println!("  {}", intro.cyan());
        }
    }

    if args.practice {
        println!();
        println!("  {} Use `luogu record --user {}` to see submission records", "💡".normal(), uid);
    }

    Ok(())
}

fn search_user(keyword: &str) -> Result<()> {
    let result = net::search_user(keyword)?;
    let users: Vec<_> = result.users.into_iter().flatten().collect();

    if users.is_empty() {
        println!("{} No users found matching: {}", "ℹ".cyan(), keyword);
        return Ok(());
    }

    println!();
    println!("{} Users matching '{}':", "🔍".cyan().bold(), keyword.yellow());
    println!("{}", format!("{:<8} | {:<25} | {:<10} | {}", "UID", "Name", "Level", "Badge").cyan());
    println!("{}", "─".repeat(70).cyan());

    for u in &users {
        let badge = u.badge.as_deref().unwrap_or("");
        let colored_name = user_color(&u.name, &u.color);
        println!(
            "{:<8} | {:<25} | {:<10} | {}",
            u.uid.to_string().cyan(),
            colored_name,
            u.ccfLevel.to_string().green(),
            badge,
        );
    }

    Ok(())
}