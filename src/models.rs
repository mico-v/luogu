use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemRecord {
    pub pid: String,
    pub title: String,
    pub difficulty: Option<i32>,
    pub difficulty_label: String,
    pub time_limit_ms: Option<i64>,
    pub memory_limit_kb: Option<i64>,
    pub tags: Vec<i64>,
    pub fetched_at: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeCompileInfo {
    pub success: bool,
    pub elapsed_seconds: f64,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeTestResult {
    pub name: String,
    pub status: String,
    pub time_ms: Option<f64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeLogEntry {
    pub timestamp: String,
    pub pid: String,
    pub status: String,
    pub success: bool,
    pub pass_count: usize,
    pub test_count: usize,
    pub compile: JudgeCompileInfo,
    pub tests: Vec<JudgeTestResult>,
}

#[derive(Debug, Clone)]
pub struct ParsedProblem {
    pub pid: String,
    pub title: String,
    pub difficulty: Option<i32>,
    pub limits_time_ms: Option<i64>,
    pub limits_memory_kb: Option<i64>,
    pub tags: Vec<i64>,
    pub markdown: String,
    pub samples: Vec<(String, String)>,
}
