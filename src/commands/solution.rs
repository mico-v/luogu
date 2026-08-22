use crate::cli::SolutionArgs;
use crate::net;
use anyhow::Result;
use colored::Colorize;
use std::fs;

pub fn run(args: SolutionArgs) -> Result<()> {
    eprintln!("{} Fetching solutions for {}...", "📝".cyan(), args.pid.cyan().bold());

    let data = match net::fetch_solutions(&args.pid, args.page) {
        Ok(d) => d,
        Err(e) => {
            let msg = format!("{:#}", e);
            if msg.contains("401") || msg.contains("login") || msg.contains("Unauthorized") {
                println!("{} Solutions for {} require login. Try using a browser to view solutions.", "🔒".normal(), args.pid.cyan());
            } else {
                println!("{} Failed to fetch solutions: {}", "✗".red(), e);
            }
            return Ok(());
        }
    };
    let solutions = &data.solutions.result;
    let total = data.solutions.count;

    if solutions.is_empty() {
        println!("{} No solutions found for {}.", "ℹ".cyan(), args.pid);
        return Ok(());
    }

    println!();
    println!(
        "{} {} solutions for {} (page {})",
        "📝".normal(),
        total.to_string().cyan().bold(),
        args.pid.cyan().bold(),
        args.page.to_string().cyan()
    );
    println!("{}", "─".repeat(70).cyan());

    for (i, sol) in solutions.iter().enumerate() {
        let idx = (args.page - 1) * 20 + i as i64 + 1;
        println!();
        println!(
            "  {}. {} [{}]",
            idx.to_string().cyan().bold(),
            sol.title.bold(),
            format!("by {}", sol.author.name).yellow()
        );
        let time = chrono::DateTime::from_timestamp(sol.time, 0)
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "?".to_string());
        println!(
            "     {}  |  👍 {}  |  💬 {}  |  {}",
            time.cyan(),
            sol.upvote.to_string().green(),
            sol.replyCount.to_string().yellow(),
            if sol.status == 1 { "public".green() } else { "hidden".red() }
        );

        // Show content if requested
        if let Some(ref content) = sol.content {
            if args.full || args.index == Some(idx as usize) {
                println!();
                println!("{}", "─".repeat(50).cyan());
                // Strip HTML tags for terminal display
                let text = strip_html(content);
                for line in text.lines() {
                    println!("  {}", line);
                }
                println!("{}", "─".repeat(50).cyan());
            }

            // Save if requested
            if args.save {
                let problem_path = args.base_dir.join(&args.pid);
                fs::create_dir_all(&problem_path)?;
                let file_path = problem_path.join(format!("solution_{}.md", idx));
                fs::write(&file_path, content)?;
                println!("     💾 Saved to {}", file_path.display().to_string().cyan());
            }
        }
    }

    println!();
    if total > (args.page * 20) as i64 {
        println!("{} Use --page {} to see more", "💡".cyan(), args.page + 1);
    }
    println!("{} Use --index <n> to view full solution content", "💡".cyan());
    println!("{} Use --save to save solutions to problem directory", "💡".cyan());

    Ok(())
}

fn strip_html(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    let without_tags = re.replace_all(html, "");
    // Decode common HTML entities
    let without_entities = without_tags
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    without_entities.to_string()
}