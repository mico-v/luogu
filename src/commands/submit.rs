use crate::cli::SubmitArgs;
use crate::net;
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::fs;

pub fn run(args: SubmitArgs) -> Result<()> {
    // Determine source file path
    let source = match args.source {
        Some(ref s) => std::path::PathBuf::from(s),
        None => {
            let dir = args.base_dir.join(&args.pid);
            // Try common C++ filenames
            let mut found = dir.join("main.cpp");
            for candidate in &["main.cpp", "main.cc", "main.cxx"] {
                let path = dir.join(candidate);
                if path.exists() {
                    found = path;
                    break;
                }
            }
            found
        }
    };

    if !source.exists() {
        return Err(anyhow!("source file not found: {}", source.display()));
    }

    let code = fs::read_to_string(&source)
        .with_context(|| format!("read {}", source.display()))?;

    if code.trim().is_empty() {
        return Err(anyhow!("source file is empty: {}", source.display()));
    }

    println!("{} Submitting {} to {} ...", "🚀".cyan(), source.display().to_string().cyan(), args.pid.cyan().bold());
    println!("  Language ID: {} (C++ with O2)", args.lang);
    println!("  O2: {}", if args.no_o2 { "disabled" } else { "enabled" }.yellow());

    // Get CSRF token
    let csrf = match args.csrf {
        Some(ref token) => token.clone(),
        None => {
            eprintln!("  {} Fetching CSRF token...", "🔑".cyan());
            net::fetch_csrf_token()?
        }
    };

    if csrf.is_empty() {
        return Err(anyhow!("failed to get CSRF token. You may need to login first."));
    }

    // Submit
    let result = net::submit_code(&args.pid, &code, args.lang, !args.no_o2, &csrf)?;

    println!();
    println!("{} Submitted successfully!", "✓".green());
    println!("  Record ID: {}", result.rid.to_string().cyan().bold());
    println!("  URL: {}", format!("https://www.luogu.com.cn/record/{}", result.rid).cyan().underline());
    println!();
    println!("{} Use `luogu record {}` to check the result", "💡".normal(), result.rid);

    Ok(())
}