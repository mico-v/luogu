use crate::models::{JudgeLogEntry, ProblemRecord};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const APP_DIR: &str = ".luogu";
const PROBLEMS_FILE: &str = "problems.json";
const LOG_FILE: &str = "judge_log.jsonl";

fn app_dir() -> PathBuf {
    PathBuf::from(APP_DIR)
}

fn ensure_app_dir() -> Result<PathBuf> {
    let dir = app_dir();
    fs::create_dir_all(&dir).with_context(|| format!("create app dir {}", dir.display()))?;
    Ok(dir)
}

pub fn load_problem_map() -> Result<BTreeMap<String, ProblemRecord>> {
    let path = app_dir().join(PROBLEMS_FILE);
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
    let path = app_dir().join(LOG_FILE);
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
