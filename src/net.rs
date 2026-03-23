use crate::models::ParsedProblem;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
const HOME_URL: &str = "https://www.luogu.com.cn/";

fn pick_text<'a>(content: &'a Value, keys: &[&str]) -> &'a str {
    for key in keys {
        if let Some(text) = content.get(*key).and_then(Value::as_str) {
            if !text.trim().is_empty() {
                return text;
            }
        }
    }
    ""
}

fn build_markdown(problem: &Value) -> String {
    let content = problem.get("content").cloned().unwrap_or(Value::Null);
    let title = content
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Unknown Title");
    let background = pick_text(&content, &["background"]);
    let description = pick_text(&content, &["description", "statement"]);
    let input_format = pick_text(&content, &["inputFormat", "formatI", "input"]);
    let output_format = pick_text(&content, &["outputFormat", "formatO", "output"]);
    let hint = pick_text(&content, &["hint"]);

    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", title));
    if !background.trim().is_empty() {
        out.push_str("## 题目背景\n\n");
        out.push_str(background);
        out.push_str("\n\n");
    }
    out.push_str("## 题目描述\n\n");
    out.push_str(description);
    out.push_str("\n\n## 输入格式\n\n");
    out.push_str(input_format);
    out.push_str("\n\n## 输出格式\n\n");
    out.push_str(output_format);

    if let Some(samples) = content.get("samples").and_then(Value::as_array) {
        for (idx, sample) in samples.iter().enumerate() {
            let input = sample.get(0).and_then(Value::as_str).unwrap_or("");
            let output = sample.get(1).and_then(Value::as_str).unwrap_or("");
            out.push_str(&format!("\n\n## 样例 #{}\n\n", idx + 1));
            out.push_str("### 输入\n\n```text\n");
            out.push_str(input);
            out.push_str("\n```\n\n### 输出\n\n```text\n");
            out.push_str(output);
            out.push_str("\n```\n");
        }
    }

    if !hint.trim().is_empty() {
        out.push_str("\n\n## 提示\n\n");
        out.push_str(hint);
        out.push('\n');
    }
    out
}

pub fn fetch_problem(pid: &str) -> Result<ParsedProblem> {
    let url = format!("https://www.luogu.com.cn/problem/{pid}");
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .cookie_store(true)
        .build()
        .context("build http client")?;

    client
        .get(HOME_URL)
        .send()
        .context("warm up session with home page")?
        .error_for_status()
        .context("home page status not success")?;

    let html = client
        .get(&url)
        .send()
        .with_context(|| format!("request problem page {url}"))?
        .error_for_status()
        .with_context(|| format!("problem page status not success: {url}"))?
        .text()
        .context("read page text")?;

    let re = Regex::new(r#"<script[^>]*id=\"lentille-context\"[^>]*>(?s)(.*?)</script>"#)?;
    let caps = re
        .captures(&html)
        .ok_or_else(|| anyhow!("cannot find lentille-context payload"))?;
    let payload = caps
        .get(1)
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow!("empty payload"))?;

    let root: Value = serde_json::from_str(payload).context("parse payload json")?;
    let current_data = root
        .get("data")
        .and_then(|v| v.get("problem"))
        .or_else(|| root.get("currentData").and_then(|v| v.get("problem")))
        .ok_or_else(|| anyhow!("payload missing problem data"))?;

    let problem = current_data;
    let pid = problem
        .get("pid")
        .and_then(Value::as_str)
        .unwrap_or(pid)
        .to_string();
    let title = problem
        .get("content")
        .and_then(|v| v.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown Title")
        .to_string();
    let difficulty = problem.get("difficulty").and_then(Value::as_i64).map(|v| v as i32);

    let mut tags = Vec::new();
    if let Some(arr) = problem.get("tags").and_then(Value::as_array) {
        for t in arr {
            if let Some(v) = t.as_i64() {
                tags.push(v);
            }
        }
    }

    let mut time_limit = None;
    let mut memory_limit = None;
    if let Some(limits) = problem.get("limits") {
        if let Some(arr) = limits.get("time").and_then(Value::as_array) {
            time_limit = arr.iter().filter_map(Value::as_i64).max();
        }
        if let Some(arr) = limits.get("memory").and_then(Value::as_array) {
            memory_limit = arr.iter().filter_map(Value::as_i64).max();
        }
    }

    let mut samples = Vec::new();
    if let Some(arr) = problem.get("samples").and_then(Value::as_array) {
        for pair in arr {
            let input = pair.get(0).and_then(Value::as_str).unwrap_or("").to_string();
            let output = pair.get(1).and_then(Value::as_str).unwrap_or("").to_string();
            samples.push((input, output));
        }
    } else if let Some(arr) = problem
        .get("content")
        .and_then(|v| v.get("samples"))
        .and_then(Value::as_array)
    {
        for pair in arr {
            let input = pair.get(0).and_then(Value::as_str).unwrap_or("").to_string();
            let output = pair.get(1).and_then(Value::as_str).unwrap_or("").to_string();
            samples.push((input, output));
        }
    }

    Ok(ParsedProblem {
        pid,
        title,
        difficulty,
        limits_time_ms: time_limit,
        limits_memory_kb: memory_limit,
        tags,
        markdown: build_markdown(problem),
        samples,
    })
}
