use crate::net;
use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    eprintln!("{} Fetching tags...", "🏷".cyan());

    let data = net::fetch_tags()?;
    let tags = &data.tags;
    let types = &data.types;

    if tags.is_empty() {
        println!("{} No tags found.", "ℹ".cyan());
        return Ok(());
    }

    // Group tags by type
    println!();
    println!("{} Tags (version {})", "🏷".cyan().bold(), data.version.to_string().cyan());
    println!("{}", "─".repeat(60).cyan());

    for ttype in types {
        let type_tags: Vec<_> = tags
            .iter()
            .filter(|t| t.ptype == ttype.id)
            .collect();

        if type_tags.is_empty() {
            continue;
        }

        println!();
        println!("  {} {} (type {})", "▸".cyan(), ttype.name.bold(), ttype.id.to_string().cyan());

        for tag in type_tags {
            let parent_info = if let Some(parent) = tag.parent {
                // Find parent name
                let parent_name = tags.iter()
                    .find(|t| t.id == parent)
                    .map(|t| t.name.as_str())
                    .unwrap_or("?");
                format!(" (parent: {})", parent_name.cyan())
            } else {
                String::new()
            };
            println!("    {:>5}  {}{}", tag.id.to_string().cyan(), tag.name, parent_info);
        }
    }

    println!();
    println!(
        "{} Use `luogu search --tag <id>` to filter by tag",
        "💡".cyan()
    );

    Ok(())
}