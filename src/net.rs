#![allow(dead_code)]

use crate::models::*;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, REFERER};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
const BASE_URL: &str = "https://www.luogu.com.cn";

// ── HTTP client ─────────────────────────────────────────────────

fn build_client() -> Result<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .cookie_store(true)
        .build()
        .context("build http client")
}

fn lentille_headers() -> Result<HeaderMap> {
    let mut h = HeaderMap::new();
    h.insert("x-lentille-request", HeaderValue::from_static("content-only"));
    Ok(h)
}

fn post_headers(csrf: &str) -> Result<HeaderMap> {
    let mut h = HeaderMap::new();
    h.insert(REFERER, HeaderValue::from_static("https://www.luogu.com.cn/"));
    h.insert("x-csrf-token", HeaderValue::from_str(csrf)?);
    h.insert("content-type", HeaderValue::from_static("application/json"));
    Ok(h)
}

fn get_json<T: DeserializeOwned>(client: &Client, url: &str, headers: &HeaderMap) -> Result<T> {
    let resp = client
        .get(url)
        .headers(headers.clone())
        .send()
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("GET {url} status error"))?;
    let text = resp.text().context("read response body")?;
    serde_json::from_str(&text).with_context(|| format!("parse JSON from {url}"))
}

fn get_json_lentille<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    query: &[(&str, String)],
) -> Result<T> {
    let headers = lentille_headers()?;
    let mut req = client.get(url).headers(headers);
    for (k, v) in query {
        req = req.query(&[(*k, v)]);
    }
    let resp = req
        .send()
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("GET {url} status error"))?;
    let text = resp.text().context("read response body")?;

    // Try to parse as JSON directly; if it fails with HTML, try extracting lentille-context
    serde_json::from_str(&text).or_else(|_| {
        extract_lentille_json(&text)
            .and_then(|v| serde_json::from_value(v).map_err(|e| anyhow::anyhow!("{}", e)))
    })
    .with_context(|| format!("parse JSON from {url}"))
}

fn post_json<T: DeserializeOwned, B: Serialize>(
    client: &Client,
    url: &str,
    headers: &HeaderMap,
    body: &B,
) -> Result<T> {
    let resp = client
        .post(url)
        .headers(headers.clone())
        .json(body)
        .send()
        .with_context(|| format!("POST {url}"))?
        .error_for_status()
        .with_context(|| format!("POST {url} status error"))?;
    let text = resp.text().context("read response body")?;
    serde_json::from_str(&text).with_context(|| format!("parse JSON from {url}"))
}

/// Extract JSON data from HTML's lentille-context script tag
fn extract_lentille_json(html: &str) -> Result<serde_json::Value> {
    let re = Regex::new(r#"<script[^>]*id="lentille-context"[^>]*>(?s)(.*?)</script>"#)?;
    let caps = re
        .captures(html)
        .ok_or_else(|| anyhow!("cannot find lentille-context payload"))?;
    let payload = caps
        .get(1)
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow!("empty lentille-context payload"))?;
    let root: serde_json::Value =
        serde_json::from_str(payload).context("parse lentille-context JSON")?;
    // LentilleResponse format: { instance, template, status, locale, data, ... }
    // DataResponse format: { code, currentData, ... }
    // Try to get the data field from either format
    if let Some(data) = root.get("data") {
        Ok(data.clone())
    } else if let Some(cd) = root.get("currentData") {
        Ok(cd.clone())
    } else {
        Ok(root)
    }
}

/// Extract inner data from a LentilleResponse or DataResponse wrapper
fn extract_data<T: DeserializeOwned>(value: serde_json::Value) -> Result<T> {
    // Try LentilleResponse format: { data: T, ... }
    if let Some(data) = value.get("data") {
        return serde_json::from_value(data.clone()).context("parse LentilleResponse data");
    }
    // Try DataResponse format: { currentData: T, ... }
    if let Some(cd) = value.get("currentData") {
        return serde_json::from_value(cd.clone()).context("parse DataResponse currentData");
    }
    // Try direct parsing
    serde_json::from_value(value).context("parse response data")
}

fn get_lentille_data<T: DeserializeOwned>(url: &str, query: &[(&str, String)]) -> Result<T> {
    let client = build_client()?;
    let headers = lentille_headers()?;
    let mut req = client.get(url).headers(headers);
    for (k, v) in query {
        req = req.query(&[(*k, v)]);
    }
    let resp = req
        .send()
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("GET {url} status error"))?;
    let text = resp.text().context("read response body")?;

    // Try direct JSON parse first
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Ok(data) = extract_data::<T>(v) {
            return Ok(data);
        }
    }

    // Fallback: extract from HTML lentille-context
    let value = extract_lentille_json(&text)?;
    extract_data::<T>(value)
}

// ── Warm up session ─────────────────────────────────────────────

pub fn warm_up(client: &Client) -> Result<()> {
    client
        .get(format!("{BASE_URL}/"))
        .send()
        .context("warm up session")?
        .error_for_status()
        .context("warm up failed")?;
    Ok(())
}

// ── Fetch problem ───────────────────────────────────────────────

pub fn fetch_problem(pid: &str) -> Result<ParsedProblem> {
    let client = build_client()?;
    let headers = lentille_headers()?;
    let url = format!("{BASE_URL}/problem/{pid}");

    let result: Result<LentilleResponse<ProblemData>> = get_json(&client, &url, &headers);

    match result {
        Ok(api_resp) => Ok(convert_problem_data(api_resp.data)),
        Err(_) => {
            // Fallback to HTML parsing
            fetch_problem_html(pid)
        }
    }
}

fn convert_problem_data(data: ProblemData) -> ParsedProblem {
    let p = data.problem;
    let content = p.content;
    let mut md = String::new();

    md.push_str(&format!("# {}\n\n", content.name));
    if let Some(ref bg) = content.background {
        if !bg.trim().is_empty() {
            md.push_str("## 题目背景\n\n");
            md.push_str(bg);
            md.push_str("\n\n");
        }
    }
    md.push_str("## 题目描述\n\n");
    md.push_str(content.description.as_deref().unwrap_or(""));
    md.push_str("\n\n## 输入格式\n\n");
    md.push_str(content.formatI.as_deref().unwrap_or(""));
    md.push_str("\n\n## 输出格式\n\n");
    md.push_str(content.formatO.as_deref().unwrap_or(""));

    for (i, (input, output)) in p.samples.iter().enumerate() {
        md.push_str(&format!("\n\n## 样例 #{}\n\n", i + 1));
        md.push_str("### 输入\n\n```text\n");
        md.push_str(input);
        md.push_str("\n```\n\n### 输出\n\n```text\n");
        md.push_str(output);
        md.push_str("\n```\n");
    }

    if let Some(ref hint) = content.hint {
        if !hint.trim().is_empty() {
            md.push_str("\n\n## 提示\n\n");
            md.push_str(hint);
            md.push('\n');
        }
    }

    let time_limit = p.limits.time.iter().max().copied();
    let memory_limit = p.limits.memory.iter().max().copied();
    let provider_name = p
        .provider
        .as_ref()
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    ParsedProblem {
        pid: p.pid,
        title: content.name,
        difficulty: p.difficulty.map(|v| v as i32),
        limits_time_ms: time_limit,
        limits_memory_kb: memory_limit,
        tags: p.tags,
        markdown: md,
        samples: p.samples,
        provider: provider_name,
        total_accepted: p.totalAccepted,
        total_submit: p.totalSubmit,
    }
}

// ── HTML fallback (original scraping logic) ─────────────────────

fn fetch_problem_html(pid: &str) -> Result<ParsedProblem> {
    let client = build_client()?;
    warm_up(&client)?;

    let url = format!("{BASE_URL}/problem/{pid}");
    let html = client
        .get(&url)
        .send()
        .with_context(|| format!("request problem page {url}"))?
        .error_for_status()
        .with_context(|| format!("problem page status not success: {url}"))?
        .text()
        .context("read page text")?;

    let re = Regex::new(r#"<script[^>]*id="lentille-context"[^>]*>(?s)(.*?)</script>"#)?;
    let caps = re
        .captures(&html)
        .ok_or_else(|| anyhow!("cannot find lentille-context payload"))?;
    let payload = caps
        .get(1)
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow!("empty payload"))?;

    let root: serde_json::Value = serde_json::from_str(payload).context("parse payload json")?;
    let problem = root
        .get("data")
        .and_then(|v| v.get("problem"))
        .or_else(|| root.get("currentData").and_then(|v| v.get("problem")))
        .ok_or_else(|| anyhow!("payload missing problem data"))?;

    let pid_out = problem
        .get("pid")
        .and_then(|v| v.as_str())
        .unwrap_or(pid)
        .to_string();
    let title = problem
        .get("content")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Title")
        .to_string();
    let difficulty = problem.get("difficulty").and_then(|v| v.as_i64()).map(|v| v as i32);

    let mut tags = Vec::new();
    if let Some(arr) = problem.get("tags").and_then(|v| v.as_array()) {
        for t in arr {
            if let Some(v) = t.as_i64() {
                tags.push(v);
            }
        }
    }

    let mut time_limit = None;
    let mut memory_limit = None;
    if let Some(limits) = problem.get("limits") {
        if let Some(arr) = limits.get("time").and_then(|v| v.as_array()) {
            time_limit = arr.iter().filter_map(|v| v.as_i64()).max();
        }
        if let Some(arr) = limits.get("memory").and_then(|v| v.as_array()) {
            memory_limit = arr.iter().filter_map(|v| v.as_i64()).max();
        }
    }

    let mut samples = Vec::new();
    for src in [
        problem.get("samples"),
        problem
            .get("content")
            .and_then(|v| v.get("samples")),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(arr) = src.as_array() {
            for pair in arr {
                let input = pair.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
                let output = pair.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                samples.push((input, output));
            }
            break;
        }
    }

    fn pick_text<'a>(content: &'a serde_json::Value, keys: &[&str]) -> &'a str {
        for key in keys {
            if let Some(text) = content.get(*key).and_then(|v| v.as_str()) {
                if !text.trim().is_empty() {
                    return text;
                }
            }
        }
        ""
    }

    let content = problem.get("content").cloned().unwrap_or(serde_json::Value::Null);
    let background = if content.is_null() {
        pick_text(problem, &["background"])
    } else {
        pick_text(&content, &["background"])
    };
    let description = if content.is_null() {
        pick_text(problem, &["description", "statement"])
    } else {
        pick_text(&content, &["description", "statement"])
    };
    let input_format = if content.is_null() {
        pick_text(problem, &["inputFormat", "formatI", "input"])
    } else {
        pick_text(&content, &["inputFormat", "formatI", "input"])
    };
    let output_format = if content.is_null() {
        pick_text(problem, &["outputFormat", "formatO", "output"])
    } else {
        pick_text(&content, &["outputFormat", "formatO", "output"])
    };
    let hint = if content.is_null() {
        pick_text(problem, &["hint"])
    } else {
        pick_text(&content, &["hint"])
    };

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

    for (idx, (input, output)) in samples.iter().enumerate() {
        out.push_str(&format!("\n\n## 样例 #{}\n\n", idx + 1));
        out.push_str("### 输入\n\n```text\n");
        out.push_str(input);
        out.push_str("\n```\n\n### 输出\n\n```text\n");
        out.push_str(output);
        out.push_str("\n```\n");
    }

    if !hint.trim().is_empty() {
        out.push_str("\n\n## 提示\n\n");
        out.push_str(hint);
        out.push('\n');
    }

    Ok(ParsedProblem {
        pid: pid_out,
        title,
        difficulty,
        limits_time_ms: time_limit,
        limits_memory_kb: memory_limit,
        tags,
        markdown: out,
        samples,
        provider: None,
        total_accepted: None,
        total_submit: None,
    })
}

// ── Problem search ──────────────────────────────────────────────

pub fn fetch_problem_list(
    page: i64,
    keyword: Option<&str>,
    difficulty: Option<i64>,
    ptype: Option<&str>,
    tag: Option<&str>,
    order_by: Option<&str>,
    order: Option<&str>,
) -> Result<ProblemListData> {
    let mut params: Vec<(String, String)> = Vec::new();
    params.push(("page".to_string(), page.to_string()));
    if let Some(k) = keyword {
        params.push(("keyword".to_string(), k.to_string()));
    }
    if let Some(d) = difficulty {
        params.push(("difficulty".to_string(), d.to_string()));
    }
    if let Some(t) = ptype {
        params.push(("type".to_string(), t.to_string()));
    }
    if let Some(t) = tag {
        params.push(("tag".to_string(), t.to_string()));
    }
    if let Some(o) = order_by {
        params.push(("orderBy".to_string(), o.to_string()));
    }
    if let Some(o) = order {
        params.push(("order".to_string(), o.to_string()));
    }

    let url = format!("{BASE_URL}/problem/list");
    let query_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let client = build_client()?;
    let headers = lentille_headers()?;
    let resp = client
        .get(&url)
        .headers(headers)
        .query(&query_refs)
        .send()
        .context("request problem list")?
        .error_for_status()
        .context("problem list status error")?;
    let text = resp.text().context("read problem list response")?;

    // Try direct JSON parse first
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(data) = v.get("data") {
            let pl: ProblemListWrapper = serde_json::from_value(data.clone())
                .context("parse problem list data")?;
            return Ok(pl.problems);
        }
    }

    // Fallback: extract from HTML lentille-context
    let value = extract_lentille_json(&text)?;
    let pl: ProblemListWrapper = serde_json::from_value(value)
        .context("parse problem list from HTML")?;
    Ok(pl.problems)
}

#[derive(Deserialize)]
struct ProblemListWrapper {
    problems: ProblemListData,
}

// ── Training API ────────────────────────────────────────────────

pub fn fetch_training(id: i64) -> Result<ProblemSetData> {
    let client = build_client()?;
    let headers = lentille_headers()?;
    let url = format!("{BASE_URL}/training/{id}");
    let resp = client
        .get(&url)
        .headers(headers)
        .send()
        .with_context(|| format!("request training {id}"))?
        .error_for_status()
        .with_context(|| format!("training {id} status error"))?;
    let text = resp.text().context("read training response")?;

    // Try direct JSON parse
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(data) = v.get("data") {
            return serde_json::from_value(data.clone())
                .with_context(|| format!("parse training {id} data"));
        }
    }

    // Fallback: extract from HTML lentille-context
    let value = extract_lentille_json(&text)?;
    serde_json::from_value(value).with_context(|| format!("parse training {id} from HTML"))
}

pub fn fetch_training_list(
    page: i64,
    keyword: Option<&str>,
    ptype: Option<&str>,
) -> Result<ProblemSetListData> {
    let mut params: Vec<(String, String)> = Vec::new();
    params.push(("page".to_string(), page.to_string()));
    if let Some(k) = keyword {
        params.push(("keyword".to_string(), k.to_string()));
    }
    if let Some(t) = ptype {
        params.push(("type".to_string(), t.to_string()));
    }

    let url = format!("{BASE_URL}/training/list");
    let query_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let client = build_client()?;
    let headers = lentille_headers()?;
    let resp = client
        .get(&url)
        .headers(headers)
        .query(&query_refs)
        .send()
        .context("request training list")?
        .error_for_status()
        .context("training list status error")?;
    let text = resp.text().context("read training list response")?;

    // Try direct JSON parse
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(data) = v.get("data") {
            return serde_json::from_value(data.clone())
                .context("parse training list data");
        }
    }

    // Fallback: extract from HTML
    let value = extract_lentille_json(&text)?;
    serde_json::from_value(value).context("parse training list from HTML")
}

// ── Contest API ─────────────────────────────────────────────────

pub fn fetch_contest(id: i64) -> Result<ContestData> {
    let client = build_client()?;
    let headers = lentille_headers()?;
    let url = format!("{BASE_URL}/contest/{id}");
    let resp: LentilleResponse<ContestData> = get_json(&client, &url, &headers)?;
    Ok(resp.data)
}

pub fn fetch_contest_list(
    page: i64,
    name: Option<&str>,
    method: Option<i64>,
    public: Option<i64>,
) -> Result<ContestListData> {
    let mut params: Vec<(String, String)> = Vec::new();
    params.push(("page".to_string(), page.to_string()));
    if let Some(n) = name {
        params.push(("name".to_string(), n.to_string()));
    }
    if let Some(m) = method {
        params.push(("method".to_string(), m.to_string()));
    }
    if let Some(p) = public {
        params.push(("public".to_string(), p.to_string()));
    }

    let url = format!("{BASE_URL}/contest/list");
    let query_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let client = build_client()?;
    let headers = lentille_headers()?;
    let resp = client
        .get(&url)
        .headers(headers)
        .query(&query_refs)
        .send()
        .context("request contest list")?
        .error_for_status()
        .context("contest list status error")?;
    let text = resp.text().context("read contest list response")?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .context("parse contest list JSON")?;
    let data = v.get("data")
        .ok_or_else(|| anyhow!("missing data in contest list response"))?;
    let cd: ContestListData = serde_json::from_value(data.clone())
        .context("parse contest list data")?;
    Ok(cd)
}

// ── User API ────────────────────────────────────────────────────

pub fn fetch_user(uid: i64) -> Result<UserData> {
    let client = build_client()?;
    let headers = lentille_headers()?;
    let url = format!("{BASE_URL}/user/{uid}");
    let resp: LentilleResponse<UserData> = get_json(&client, &url, &headers)?;
    Ok(resp.data)
}

pub fn search_user(keyword: &str) -> Result<UserSearchResult> {
    let client = build_client()?;
    let url = format!("{BASE_URL}/api/user/search");
    let resp = client
        .get(&url)
        .query(&[("keyword", keyword)])
        .send()
        .context("search user")?
        .error_for_status()
        .context("search user status error")?;
    let text = resp.text().context("read search user response")?;
    serde_json::from_str(&text).context("parse search user JSON")
}

// ── Solution API ────────────────────────────────────────────────

pub fn fetch_solutions(pid: &str, page: i64) -> Result<SolutionsData> {
    let client = build_client()?;
    let headers = lentille_headers()?;
    let url = format!("{BASE_URL}/problem/solution/{pid}");
    let resp = client
        .get(&url)
        .headers(headers)
        .query(&[("page", page.to_string())])
        .send()
        .with_context(|| format!("request solutions for {pid}"))?;
    let status = resp.status();
    let text = resp.text().with_context(|| format!("read solutions response for {pid}"))?;

    if !status.is_success() {
        return Err(anyhow!("HTTP {}: {}", status.as_u16(), "solutions require login"));
    }

    let v: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("parse solutions JSON for {pid}"))?;
    let data = v.get("data")
        .ok_or_else(|| anyhow!("missing data in solutions response"))?;
    serde_json::from_value(data.clone())
        .with_context(|| format!("parse solutions data for {pid}"))
}

// ── Record API ──────────────────────────────────────────────────

pub fn fetch_record_list(params: &RecordListParams) -> Result<RecordListData> {
    let mut query_params: Vec<(String, String)> = Vec::new();
    query_params.push(("page".to_string(), params.page.unwrap_or(1).to_string()));
    if let Some(ref p) = params.pid {
        query_params.push(("pid".to_string(), p.clone()));
    }
    if let Some(ref u) = params.user {
        query_params.push(("user".to_string(), u.clone()));
    }
    if let Some(s) = params.status {
        query_params.push(("status".to_string(), s.to_string()));
    }
    if let Some(l) = params.language {
        query_params.push(("language".to_string(), l.to_string()));
    }

    let url = format!("{BASE_URL}/record/list");
    let query_refs: Vec<(&str, &str)> = query_params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let client = build_client()?;
    let headers = lentille_headers()?;
    let resp = client
        .get(&url)
        .headers(headers)
        .query(&query_refs)
        .send()
        .context("request record list")?
        .error_for_status()
        .context("record list status error")?;
    let text = resp.text().context("read record list response")?;

    // Try direct JSON parse
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(data) = v.get("data") {
            // Check if we got webauthn (not logged in) - return empty
            if data.get("webauthn").is_some() {
                return Ok(RecordListData {
                    records: RecordList {
                        result: Vec::new(),
                        count: 0,
                        perPage: None,
                    },
                });
            }
            if let Some(records) = data.get("records") {
                return serde_json::from_value(records.clone())
                    .map(|records| RecordListData { records })
                    .context("parse record list data");
            }
        }
    }

    // Fallback: extract from HTML lentille-context
    let value = extract_lentille_json(&text)?;
    if let Some(records) = value.get("records") {
        let records: RecordList = serde_json::from_value(records.clone())
            .context("parse record list from HTML")?;
        return Ok(RecordListData { records });
    }

    // If we got here, no records found (likely not logged in)
    Ok(RecordListData {
        records: RecordList {
            result: Vec::new(),
            count: 0,
            perPage: None,
        },
    })
}

pub fn fetch_record_detail(id: i64) -> Result<RecordData> {
    let url = format!("{BASE_URL}/record/{id}");
    // Try direct JSON with lentille header
    let client = build_client()?;
    let headers = lentille_headers()?;
    let resp = client.get(&url).headers(headers).send()
        .context("request record detail")?;
    let text = resp.text().context("read record detail response")?;

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(data) = v.get("data") {
            return serde_json::from_value(data.clone())
                .context("parse record detail data");
        }
    }

    // Fallback: extract from HTML
    let value = extract_lentille_json(&text)?;
    serde_json::from_value(value).context("parse record detail from HTML")
}

// ── Tags API ────────────────────────────────────────────────────

pub fn fetch_tags() -> Result<TagsResponse> {
    let client = build_client()?;
    let url = format!("{BASE_URL}/_lfe/tags");
    let resp = client
        .get(&url)
        .send()
        .context("request tags")?
        .error_for_status()
        .context("tags status error")?;
    let text = resp.text().context("read tags response")?;
    serde_json::from_str(&text).context("parse tags JSON")
}

// ── Ranking API ─────────────────────────────────────────────────

pub fn fetch_gu_ranking(page: i64) -> Result<RankingData> {
    let client = build_client()?;
    let headers = lentille_headers()?;
    let url = format!("{BASE_URL}/ranking");
    let resp = client
        .get(&url)
        .headers(headers)
        .query(&[("page", page.to_string())])
        .send()
        .context("request gu ranking")?
        .error_for_status()
        .context("gu ranking status error")?;
    let text = resp.text().context("read gu ranking response")?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .context("parse gu ranking JSON")?;
    let data = v.get("data")
        .ok_or_else(|| anyhow!("missing data in gu ranking"))?;
    serde_json::from_value(data.clone()).context("parse gu ranking data")
}

pub fn fetch_elo_ranking(page: i64) -> Result<EloRankingData> {
    let client = build_client()?;
    let headers = lentille_headers()?;
    let url = format!("{BASE_URL}/ranking/elo");
    let resp = client
        .get(&url)
        .headers(headers)
        .query(&[("page", page.to_string())])
        .send()
        .context("request elo ranking")?
        .error_for_status()
        .context("elo ranking status error")?;
    let text = resp.text().context("read elo ranking response")?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .context("parse elo ranking JSON")?;
    let data = v.get("data")
        .ok_or_else(|| anyhow!("missing data in elo ranking"))?;
    serde_json::from_value(data.clone()).context("parse elo ranking data")
}

// ── Submit API ──────────────────────────────────────────────────

pub fn fetch_csrf_token() -> Result<String> {
    let client = build_client()?;
    let html = client
        .get(format!("{BASE_URL}/problem/P1000"))
        .send()
        .context("fetch page for CSRF token")?
        .error_for_status()
        .context("CSRF page status error")?
        .text()
        .context("read CSRF page")?;

    let re = Regex::new(r#"<meta\s+name="csrf-token"\s+content="([^"]+)"\s*/>"#)?;
    if let Some(caps) = re.captures(&html) {
        return Ok(caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string());
    }

    // Fallback: try to find it in lentille-context
    let re2 =
        Regex::new(r#"<script[^>]*id="lentille-context"[^>]*>(?s)(.*?)</script>"#)?;
    if let Some(caps) = re2.captures(&html) {
        let payload = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let root: serde_json::Value =
            serde_json::from_str(payload).unwrap_or(serde_json::Value::Null);
        if let Some(csrf) = root
            .get("config")
            .and_then(|c| c.get("csrfToken"))
            .and_then(|c| c.as_str())
        {
            return Ok(csrf.to_string());
        }
    }

    Err(anyhow!("cannot find CSRF token"))
}

pub fn submit_code(pid: &str, code: &str, lang: i64, enable_o2: bool, csrf: &str) -> Result<SubmitResponse> {
    let client = build_client()?;
    let headers = post_headers(csrf)?;
    let url = format!("{BASE_URL}/fe/api/problem/submit/{pid}");

    let body = SubmitCodeRequest {
        code: code.to_string(),
        lang: Some(lang),
        enableO2: Some(if enable_o2 { 1 } else { 0 }),
        captcha: None,
    };

    post_json(&client, &url, &headers, &body)
}

// ── Config API ──────────────────────────────────────────────────

pub fn fetch_config() -> Result<ConfigResponse> {
    let client = build_client()?;
    let url = format!("{BASE_URL}/_lfe/config");
    let resp = client
        .get(&url)
        .send()
        .context("request config")?
        .error_for_status()
        .context("config status error")?;
    let text = resp.text().context("read config response")?;
    serde_json::from_str(&text).context("parse config JSON")
}

// ── Helper: deserialize list result ─────────────────────────────

fn deserialize_list_result<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .map(T::deserialize)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::custom),
        serde_json::Value::Object(_) => Ok(Vec::new()),
        _ => Err(Error::custom("expected array or object for list result")),
    }
}