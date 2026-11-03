use crate::cli::TrainingArgs;
use crate::models::ProblemRecord;
use crate::{config, net, storage};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

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

pub fn run(args: TrainingArgs) -> Result<()> {
    if args.list {
        return list_trainings(&args);
    }

    let cfg = config::load()?;
    let base_dir = args
        .base_dir
        .as_deref()
        .unwrap_or_else(|| cfg.directories().problems());
    storage::migrate_legacy_problem_root(base_dir)?;
    let tid = args.id.ok_or_else(|| {
        anyhow::anyhow!("training ID required. Use --list to list training sets, or provide an ID.")
    })?;
    fetch_training_set(tid, base_dir, &cfg, args.force)
}

pub fn fetch_training_set(
    tid: i64,
    base_dir: &Path,
    cfg: &config::Config,
    force: bool,
) -> Result<()> {
    fetch_training_set_to(tid, base_dir, cfg, force)
}

fn fetch_training_set_to(
    tid: i64,
    base_dir: &Path,
    cfg: &config::Config,
    force: bool,
) -> Result<()> {
    eprintln!(
        "{} Fetching training set {}...",
        "📚".cyan(),
        tid.to_string().bold()
    );

    let data = net::fetch_training(tid)?;
    let training = data.training;
    let problems = &training.problems;
    let training_dir = cfg
        .directories()
        .training()
        .join(cfg.naming().training_name(tid, &training.name)?);

    println!();
    println!(
        "{} {} {} ({} problems)",
        "📚".normal(),
        training.name.bold(),
        format!("#{}", tid).cyan(),
        problems.len().to_string().green()
    );
    if !training.description.is_empty() {
        println!("{}", training.description.cyan());
    }
    println!("{}", "─".repeat(60).cyan());

    if problems.is_empty() {
        println!("{} No problems in this training set.", "ℹ".cyan());
        return Ok(());
    }

    let mut map = storage::load_problem_map()?;
    let mut success = 0usize;
    let mut skipped = 0usize;

    for entry in problems {
        let pid = &entry.pid;
        let title = &entry.title;
        let problem_path = storage::problem_dir(base_dir, pid);

        if problem_path.exists() && !force {
            println!(
                "  {} {} {} {}",
                "⏭".yellow(),
                pid.cyan(),
                title,
                "(skipped, exists)".yellow()
            );
            skipped += 1;
            if !map.contains_key(pid) {
                // Still record metadata if not in map
                let record = ProblemRecord {
                    pid: pid.clone(),
                    title: title.clone(),
                    difficulty: entry.difficulty.map(|v| v as i32),
                    difficulty_label: difficulty_label(entry.difficulty.map(|v| v as i32)),
                    time_limit_ms: None,
                    memory_limit_kb: None,
                    tags: entry.tags.clone(),
                    fetched_at: Utc::now().to_rfc3339(),
                    url: format!("https://www.luogu.com.cn/problem/{pid}"),
                };
                map.insert(pid.clone(), record);
            }
            continue;
        }

        // Fetch and save the problem
        match net::fetch_problem(pid) {
            Ok(problem) => {
                if let Err(e) = save_problem_files(&problem, base_dir, force) {
                    eprintln!("  {} {} {}: {}", "✗".red(), pid.cyan(), title, e);
                    continue;
                }
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
                println!(
                    "  {} {} {} {}",
                    "✓".green(),
                    pid.cyan(),
                    title,
                    difficulty_label(problem.difficulty).cyan()
                );
                success += 1;
            }
            Err(e) => {
                eprintln!("  {} {} {}: {}", "✗".red(), pid.cyan(), title, e);
            }
        }
    }

    let mut prepared = 0usize;
    for entry in problems {
        let problem_path = storage::problem_dir(base_dir, &entry.pid);
        if !problem_path.exists() {
            continue;
        }
        let link_name = cfg.naming().problem_name(&entry.pid, &entry.title)?;
        match prepare_collection_files(&problem_path, &training_dir, &entry.pid, &link_name, force)
        {
            Ok(()) => prepared += 1,
            Err(e) => eprintln!(
                "  {} {} training workspace: {}",
                "✗".red(),
                entry.pid.cyan(),
                e
            ),
        }
    }

    storage::save_problem_map(&map)?;

    println!();
    println!(
        "{} {} problems downloaded, {} skipped, {} training workspaces (total {})",
        "📊".normal(),
        success.to_string().green(),
        skipped.to_string().yellow(),
        prepared.to_string().cyan(),
        problems.len().to_string().cyan()
    );
    println!("{} {}", "🔗".normal(), training_dir.display());

    Ok(())
}

fn list_trainings(args: &TrainingArgs) -> Result<()> {
    let data = net::fetch_training_list(args.page, args.keyword.as_deref(), None)?;

    let trainings = &data.trainings.result;
    if trainings.is_empty() {
        println!("{} No training sets found.", "ℹ".cyan());
        return Ok(());
    }

    println!();
    println!("{} Public training sets:", "📚".cyan().bold());
    println!(
        "{}",
        format!(
            "{:<8} | {:<40} | {:<10} | {:<10}",
            "ID", "Name", "Problems", "Marks"
        )
        .cyan()
    );
    println!("{}", "─".repeat(78).cyan());

    for t in trainings {
        let name = if t.name.len() > 37 {
            format!("{}...", t.name.chars().take(34).collect::<String>())
        } else {
            t.name.clone()
        };
        println!(
            "{:<8} | {:<40} | {:<10} | {:<10}",
            t.id.to_string().cyan().bold(),
            name,
            t.problemCount.to_string().green(),
            t.markCount.to_string().yellow(),
        );
    }

    println!();
    if data.trainings.count > args.page * 20 {
        println!("{} Use --page {} to see more", "💡".cyan(), args.page + 1);
    }

    Ok(())
}

/// Save problem files (called both from fetch::run and training::run)
pub fn save_problem_files(
    problem: &crate::models::ParsedProblem,
    base_dir: &Path,
    force: bool,
) -> Result<()> {
    let problem_path = storage::problem_dir(base_dir, &problem.pid);
    if !problem_path.exists() {
        fs::create_dir_all(&problem_path)
            .with_context(|| format!("create {}", problem_path.display()))?;
    }
    fs::write(problem_path.join("T.md"), problem.markdown.as_bytes())?;
    let cfg = crate::config::load()?;

    let source_file = cfg.source();
    let source_stem = Path::new(source_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("invalid source filename: {source_file}"))?;
    let source_path = problem_path.join(source_file);
    if !source_path.exists() || force {
        // No template configured means no source file is generated by default.
        if let Some(template) = cfg.template() {
            fs::write(&source_path, template)?;
        }
    }

    // Testcases follow the configured layout (default: testcases/main.<n>.in|.ans).
    if !problem.samples.is_empty() {
        let testcases = cfg.testcases();
        let tc_dir = problem_path.join(testcases.directory());
        fs::create_dir_all(&tc_dir).with_context(|| format!("create {}", tc_dir.display()))?;
        for (idx, (input, output)) in problem.samples.iter().enumerate() {
            let in_path = testcases.input_path(&tc_dir, source_stem, idx);
            let out_path = testcases.output_path(&tc_dir, source_stem, idx);
            fs::write(&in_path, input)?;
            fs::write(&out_path, output)?;
        }
    }
    Ok(())
}

pub(crate) fn prepare_collection_files(
    problem_path: &Path,
    collection_dir: &Path,
    pid: &str,
    link_name: &str,
    force: bool,
) -> Result<()> {
    fs::create_dir_all(collection_dir)
        .with_context(|| format!("create {}", collection_dir.display()))?;

    for legacy_name in [
        format!("{pid}.cpp"),
        format!("{pid}.md"),
        format!("{pid}.testcases"),
    ] {
        let legacy_path = collection_dir.join(legacy_name);
        if fs::symlink_metadata(&legacy_path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            fs::remove_file(&legacy_path).with_context(|| {
                format!("remove legacy collection link {}", legacy_path.display())
            })?;
        }
    }

    if link_name != pid {
        let previous_link = collection_dir.join(pid);
        if fs::symlink_metadata(&previous_link)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            fs::remove_file(&previous_link).with_context(|| {
                format!(
                    "remove previous collection link {}",
                    previous_link.display()
                )
            })?;
        }
    }

    create_relative_symlink(problem_path, &collection_dir.join(link_name), true, force)
}

fn create_relative_symlink(target: &Path, link: &Path, is_dir: bool, force: bool) -> Result<()> {
    let target_abs = absolute_link_target(target)?;
    let link_parent = link
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let link_parent_abs = fs::canonicalize(link_parent)
        .with_context(|| format!("resolve {}", link_parent.display()))?;
    let relative_target = relative_path(&link_parent_abs, &target_abs);

    if let Ok(existing) = fs::read_link(link) {
        if existing == relative_target {
            return Ok(());
        }
        if !force {
            return Err(anyhow!(
                "{} already exists and points to {} (use --force to replace it)",
                link.display(),
                existing.display()
            ));
        }
    } else if link.exists() && !force {
        return Err(anyhow!(
            "{} already exists (use --force to replace it)",
            link.display()
        ));
    }

    if force && (link.exists() || fs::symlink_metadata(link).is_ok()) {
        remove_link(link)?;
    }

    create_symlink(&relative_target, link, is_dir)
        .with_context(|| format!("link {} -> {}", link.display(), relative_target.display()))
}

fn absolute_link_target(target: &Path) -> Result<PathBuf> {
    let parent = target
        .parent()
        .ok_or_else(|| anyhow!("target has no parent: {}", target.display()))?;
    let file_name = target
        .file_name()
        .ok_or_else(|| anyhow!("target has no filename: {}", target.display()))?;
    let parent =
        fs::canonicalize(parent).with_context(|| format!("resolve {}", parent.display()))?;
    Ok(parent.join(file_name))
}

fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();
    let common = from_components
        .iter()
        .zip(&to_components)
        .take_while(|(left, right)| left == right)
        .count();

    let mut relative = PathBuf::new();
    for _ in common..from_components.len() {
        relative.push("..");
    }
    for component in &to_components[common..] {
        relative.push(component.as_os_str());
    }
    relative
}

fn remove_link(path: &Path) -> Result<()> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("inspect {}", path.display()))?;
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path).with_context(|| format!("remove {}", path.display()))
    } else {
        fs::remove_file(path).with_context(|| format!("remove {}", path.display()))
    }
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path, _is_dir: bool) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_symlink(target: &Path, link: &Path, is_dir: bool) -> std::io::Result<()> {
    if is_dir {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "luogu_training_{name}_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_problem_directory_link_for_collection() {
        let root = temp_dir("links");
        let problems = root.join("problems");
        let training = root.join("training/100");
        let problem = problems.join("P1000");
        fs::create_dir_all(&problem).unwrap();
        fs::write(problem.join("main.cpp"), "// solution\n").unwrap();
        fs::write(problem.join("T.md"), "# Problem\n").unwrap();
        fs::create_dir_all(problem.join("testcases")).unwrap();
        fs::write(problem.join("testcases/main.0.in"), "1 2\n").unwrap();
        fs::write(problem.join("testcases/main.0.ans"), "3\n").unwrap();

        prepare_collection_files(&problem, &training, "P1000", "P1000-测试题目", false).unwrap();

        let problem_link = training.join("P1000-测试题目");
        assert_eq!(
            fs::read_link(&problem_link).unwrap(),
            PathBuf::from("../../problems/P1000")
        );
        assert!(problem_link.is_dir());
        assert_eq!(
            fs::read_to_string(problem_link.join("main.cpp")).unwrap(),
            "// solution\n"
        );
        assert_eq!(
            fs::read_to_string(problem_link.join("T.md")).unwrap(),
            "# Problem\n"
        );
        assert_eq!(
            fs::read_to_string(problem_link.join("testcases/main.0.in")).unwrap(),
            "1 2\n"
        );
        assert!(!training.join("P1000").exists());
        assert!(!training.join("P1000-测试题目.cpp").exists());
        assert!(!training.join("P1000-测试题目.md").exists());
        assert!(!training.join("P1000-测试题目.testcases").exists());

        // Running the same import again is idempotent without --force.
        prepare_collection_files(&problem, &training, "P1000", "P1000-测试题目", false).unwrap();
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn collection_link_keeps_testcases_in_the_problem_directory() {
        let root = temp_dir("source");
        let problem = root.join("problems/P1000");
        let training = root.join("training/100");
        fs::create_dir_all(&problem).unwrap();
        fs::write(problem.join("T.md"), "# Problem\n").unwrap();
        fs::create_dir_all(problem.join("testcases")).unwrap();
        fs::write(problem.join("testcases/main.0.in"), "1 2\n").unwrap();
        fs::write(problem.join("testcases/main.0.ans"), "3\n").unwrap();

        prepare_collection_files(&problem, &training, "P1000", "P1000-测试题目", false).unwrap();

        assert!(!training.join("testcases").exists());
        assert_eq!(
            fs::read_to_string(training.join("P1000-测试题目/testcases/main.0.ans")).unwrap(),
            "3\n"
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn relative_path_walks_up_to_a_sibling_directory() {
        assert_eq!(
            relative_path(
                Path::new("/workspace/training"),
                Path::new("/workspace/problems/P1000/main.cpp")
            ),
            PathBuf::from("../problems/P1000/main.cpp")
        );
    }
}
