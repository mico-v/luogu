use crate::cli::FetchArgs;
use crate::commands::training;
use crate::models::ProblemRecord;
use crate::{net, storage};
use anyhow::{bail, Context, Result};
use chrono::Utc;
use colored::Colorize;
use reqwest::Url;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
enum FetchTarget {
    Problem(String),
    Training(i64),
}

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

pub fn run(args: FetchArgs) -> Result<()> {
    match parse_fetch_target(&args.pid)? {
        FetchTarget::Problem(pid) => fetch_problem(&pid, &args.base_dir, args.force),
        FetchTarget::Training(id) => training::fetch_training_set(id, &args.base_dir, args.force),
    }
}

fn fetch_problem(pid: &str, base_dir: &Path, force: bool) -> Result<()> {
    eprintln!("{}", "Fetching problem...".cyan());
    let problem = net::fetch_problem(pid)?;
    let problem_path = storage::problem_dir(base_dir, &problem.pid);

    if problem_path.exists() && !force {
        println!("{} Problem directory exists: {}", "⚠".yellow(), problem_path.display());
        println!("{} Use --force to overwrite generated files.", "💡".cyan());
        return Ok(());
    }

    fs::create_dir_all(&problem_path).with_context(|| format!("create {}", problem_path.display()))?;
    training::save_problem_files(&problem, base_dir, force)?;

    let mut map: BTreeMap<String, ProblemRecord> = storage::load_problem_map()?;
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
    storage::save_problem_map(&map)?;

    println!();
    println!("{} {} {} {}", "✓".green(), problem.pid, problem.title, format!("[{}]", difficulty_label(problem.difficulty)).cyan());
    println!("{} {}", "📁".normal(), problem_path.display());
    print!("{} {} samples | {} time | {} memory", "📊".normal(), problem.samples.len(),
        format!("{}ms", problem.limits_time_ms.unwrap_or(0)).green(),
        format!("{}KB", problem.limits_memory_kb.unwrap_or(0)).green());

    if let Some(ref provider) = problem.provider {
        print!(" | {}", format!("by {provider}").yellow());
    }
    if let (Some(ac), Some(sub)) = (problem.total_accepted, problem.total_submit) {
        print!(" | AC {}/{}", ac.to_string().green(), sub.to_string().yellow());
    }
    println!();
    Ok(())
}

fn parse_training_id(raw: &str) -> Result<i64> {
    let id = raw
        .trim()
        .parse::<i64>()
        .with_context(|| format!("invalid training ID: {raw}"))?;
    if id <= 0 {
        bail!("training ID must be a positive integer");
    }
    Ok(id)
}

fn parse_luogu_url(raw: &str) -> Result<FetchTarget> {
    let url = if raw.starts_with("http://") || raw.starts_with("https://") {
        Url::parse(raw)
    } else {
        Url::parse(&format!("https://{raw}"))
    }
    .with_context(|| format!("invalid Luogu URL: {raw}"))?;

    if !matches!(url.host_str(), Some("www.luogu.com.cn" | "luogu.com.cn")) {
        bail!("unsupported URL host: {}", url.host_str().unwrap_or("unknown"));
    }

    let segments: Vec<&str> = url
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|segment| !segment.is_empty())
        .collect();
    match segments.as_slice() {
        ["problem", pid] => Ok(FetchTarget::Problem(pid.to_ascii_uppercase())),
        ["training", id] => parse_training_id(id).map(FetchTarget::Training),
        _ => bail!(
            "unsupported Luogu URL path: {} (expected /problem/<pid> or /training/<id>)",
            url.path()
        ),
    }
}

fn parse_fetch_target(raw: &str) -> Result<FetchTarget> {
    let raw = raw.trim();
    if raw.is_empty() {
        bail!("fetch target cannot be empty");
    }
    if raw.starts_with("http://")
        || raw.starts_with("https://")
        || raw.contains("luogu.com.cn")
    {
        return parse_luogu_url(raw);
    }

    let path = raw.trim_matches('/');
    if let Some(id) = path.strip_prefix("training/") {
        return parse_training_id(id).map(FetchTarget::Training);
    }
    if let Some(pid) = path.strip_prefix("problem/") {
        return Ok(FetchTarget::Problem(pid.to_ascii_uppercase()));
    }
    if path.chars().all(|ch| ch.is_ascii_digit()) {
        return parse_training_id(path).map(FetchTarget::Training);
    }

    Ok(FetchTarget::Problem(path.to_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_problem_targets() {
        assert_eq!(
            parse_fetch_target("P1000").unwrap(),
            FetchTarget::Problem("P1000".to_string())
        );
        assert_eq!(
            parse_fetch_target("https://www.luogu.com.cn/problem/P1000?foo=bar").unwrap(),
            FetchTarget::Problem("P1000".to_string())
        );
    }

    #[test]
    fn parses_training_targets() {
        for target in [
            "100",
            "training/100",
            "https://www.luogu.com.cn/training/100",
            "https://www.luogu.com.cn/training/100/",
        ] {
            assert_eq!(
                parse_fetch_target(target).unwrap(),
                FetchTarget::Training(100),
                "target: {target}"
            );
        }
    }

    #[test]
    fn rejects_unsupported_urls() {
        assert!(parse_fetch_target("https://www.luogu.com.cn/contest/100").is_err());
        assert!(parse_fetch_target("https://example.com/training/100").is_err());
    }
}
