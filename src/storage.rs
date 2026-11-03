use crate::config;
use crate::models::{JudgeLogEntry, ProblemRecord};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const PROBLEMS_FILE: &str = "problems.json";
const LOG_FILE: &str = "judge_log.jsonl";
const LEGACY_PROBLEMS_DIR: &str = "problem";
const DEFAULT_PROBLEMS_DIR: &str = "problems";

fn app_dir() -> Result<PathBuf> {
    Ok(config::load()?.directories().state().to_path_buf())
}

fn ensure_app_dir() -> Result<PathBuf> {
    let dir = app_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("create app dir {}", dir.display()))?;
    Ok(dir)
}

pub fn load_problem_map() -> Result<BTreeMap<String, ProblemRecord>> {
    let path = app_dir()?.join(PROBLEMS_FILE);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let map: BTreeMap<String, ProblemRecord> =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(map)
}

pub fn save_problem_map(map: &BTreeMap<String, ProblemRecord>) -> Result<()> {
    let dir = ensure_app_dir()?;
    let path = dir.join(PROBLEMS_FILE);
    let text = serde_json::to_string_pretty(map)?;
    fs::write(&path, format!("{text}\n")).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn append_judge_log(entry: &JudgeLogEntry) -> Result<()> {
    let dir = ensure_app_dir()?;
    let path = dir.join(LOG_FILE);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(entry)?)?;
    Ok(())
}

pub fn read_judge_logs(limit: usize) -> Result<Vec<JudgeLogEntry>> {
    let path = app_dir()?.join(LOG_FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = OpenOptions::new().read(true).open(&path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let entry: JudgeLogEntry = match serde_json::from_value(value) {
            Ok(v) => v,
            Err(_) => continue,
        };
        entries.push(entry);
    }
    entries.reverse();
    if entries.len() > limit {
        entries.truncate(limit);
    }
    Ok(entries)
}

pub fn problem_dir(base_dir: &Path, pid: &str) -> PathBuf {
    base_dir.join(pid)
}

/// Move the old singular problem directory to the current default when safe.
///
/// When both directories exist we leave them untouched because one may contain
/// a solution that the other does not.
pub fn migrate_legacy_problem_root(base_dir: &Path) -> Result<()> {
    if base_dir != Path::new(DEFAULT_PROBLEMS_DIR) {
        return Ok(());
    }

    let legacy_dir = Path::new(LEGACY_PROBLEMS_DIR);
    migrate_problem_root(legacy_dir, base_dir)
}

fn migrate_problem_root(legacy_dir: &Path, current_dir: &Path) -> Result<()> {
    if !legacy_dir.exists() {
        return Ok(());
    }

    if current_dir.exists() {
        eprintln!(
            "warning: both {} and {} exist; merge the legacy directory manually to avoid duplicate workspaces",
            legacy_dir.display(),
            current_dir.display()
        );
        return Ok(());
    }

    fs::rename(legacy_dir, current_dir).with_context(|| {
        format!(
            "migrate {} to {}",
            legacy_dir.display(),
            current_dir.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "luogu_storage_{name}_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn migrates_legacy_problem_root_when_current_is_absent() {
        let root = temp_dir("migrate");
        let legacy = root.join("problem");
        let current = root.join("problems");
        fs::create_dir_all(legacy.join("P1000")).unwrap();
        fs::write(legacy.join("P1000/main.cpp"), "// solution\n").unwrap();

        migrate_problem_root(&legacy, &current).unwrap();

        assert!(!legacy.exists());
        assert_eq!(
            fs::read_to_string(current.join("P1000/main.cpp")).unwrap(),
            "// solution\n"
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn leaves_both_problem_roots_untouched_when_both_exist() {
        let root = temp_dir("both");
        let legacy = root.join("problem");
        let current = root.join("problems");
        fs::create_dir_all(legacy.join("P1000")).unwrap();
        fs::create_dir_all(current.join("P1000")).unwrap();
        fs::write(legacy.join("P1000/main.cpp"), "legacy\n").unwrap();
        fs::write(current.join("P1000/main.cpp"), "current\n").unwrap();

        migrate_problem_root(&legacy, &current).unwrap();

        assert_eq!(
            fs::read_to_string(legacy.join("P1000/main.cpp")).unwrap(),
            "legacy\n"
        );
        assert_eq!(
            fs::read_to_string(current.join("P1000/main.cpp")).unwrap(),
            "current\n"
        );
        fs::remove_dir_all(root).ok();
    }
}
