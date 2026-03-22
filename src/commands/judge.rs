use crate::cli::JudgeArgs;
use crate::models::{JudgeCompileInfo, JudgeLogEntry, JudgeTestResult};
use crate::storage;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;
use wait_timeout::ChildExt;

fn normalize_text(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}

fn compile(problem_dir: &std::path::Path, source: &str, cpp_std: &str, cflags: &[String]) -> Result<JudgeCompileInfo> {
    let src_path = problem_dir.join(source);
    if !src_path.exists() {
        return Err(anyhow!("source file not found: {}", src_path.display()));
    }

    let bin_path = problem_dir.join(".luogu_solution");
    let start = Instant::now();
    let mut cmd = Command::new("g++");
    cmd.arg(&src_path)
        .arg(format!("-std={cpp_std}"))
        .arg("-O2")
        .arg("-pipe")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-DLOCAL=1")
        .arg("-o")
        .arg(&bin_path);
    for flag in cflags {
        cmd.arg(flag);
    }
    let out = cmd.output().context("run g++")?;
    let elapsed = start.elapsed().as_secs_f64();

    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    Ok(JudgeCompileInfo {
        success: out.status.success(),
        elapsed_seconds: elapsed,
        stderr,
    })
}

fn collect_samples(problem_dir: &std::path::Path) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut v = Vec::new();
    if let Ok(entries) = fs::read_dir(problem_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("in") {
                v.push((p.clone(), p.with_extension("out")));
            }
        }
    }
    v.sort();
    v
}

pub fn run(args: JudgeArgs) -> Result<()> {
    let pid = args.pid.trim().to_uppercase();
    let problem_dir = storage::problem_dir(&args.base_dir, &pid);
    if !problem_dir.exists() {
        return Err(anyhow!("problem dir not found: {}", problem_dir.display()));
    }

    println!("Judging {} @ {}", pid, problem_dir.display());

    let compile_info = compile(&problem_dir, &args.source, &args.std, &args.cflags)?;
    if !compile_info.success {
        println!("Compile failed in {:.2}s", compile_info.elapsed_seconds);
        if !compile_info.stderr.trim().is_empty() {
            println!("{}", compile_info.stderr);
        }
        let entry = JudgeLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            pid,
            status: "COMPILE_ERROR".to_string(),
            success: false,
            pass_count: 0,
            test_count: 0,
            compile: compile_info,
            tests: vec![],
        };
        storage::append_judge_log(&entry)?;
        std::process::exit(1);
    }

    let bin_path = problem_dir.join(".luogu_solution");
    let samples = collect_samples(&problem_dir);
    if samples.is_empty() {
        return Err(anyhow!("no sample *.in files in {}", problem_dir.display()));
    }

    let mut tests = Vec::new();
    let mut pass_count = 0usize;

    for (input, output) in samples {
        let name = input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("sample")
            .to_string();

        if !output.exists() {
            tests.push(JudgeTestResult {
                name,
                status: "NO_EXPECTED".to_string(),
                time_ms: None,
                message: format!("missing {}", output.display()),
            });
            continue;
        }

        let input_text = fs::read_to_string(&input).unwrap_or_default();
        let expected = fs::read_to_string(&output).unwrap_or_default();

        let start = Instant::now();
        let mut child = Command::new(&bin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("spawn {}", bin_path.display()))?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(input_text.as_bytes())?;
        }

        let timeout = std::time::Duration::from_secs_f64(args.timeout.unwrap_or(3.0));
        let status = child.wait_timeout(timeout)?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        let test = if status.is_none() {
            let _ = child.kill();
            let _ = child.wait();
            JudgeTestResult {
                name,
                status: "TLE".to_string(),
                time_ms: Some(elapsed_ms),
                message: format!("timeout after {:.2}s", timeout.as_secs_f64()),
            }
        } else {
            let out = child.wait_with_output()?;
            let actual = String::from_utf8_lossy(&out.stdout).to_string();
            if !out.status.success() {
                JudgeTestResult {
                    name,
                    status: "RE".to_string(),
                    time_ms: Some(elapsed_ms),
                    message: format!("exit code {:?}", out.status.code()),
                }
            } else if normalize_text(&actual) == normalize_text(&expected) {
                pass_count += 1;
                JudgeTestResult {
                    name,
                    status: "AC".to_string(),
                    time_ms: Some(elapsed_ms),
                    message: "accepted".to_string(),
                }
            } else {
                JudgeTestResult {
                    name,
                    status: "WA".to_string(),
                    time_ms: Some(elapsed_ms),
                    message: "wrong answer".to_string(),
                }
            }
        };
        println!(
            "[{}] {} | {:.2} ms | {}",
            test.name,
            test.status,
            test.time_ms.unwrap_or(0.0),
            test.message
        );
        tests.push(test);
    }

    let test_count = tests.len();
    let all_pass = pass_count == test_count;
    let status = if all_pass { "AC" } else { "FAILED" }.to_string();

    let entry = JudgeLogEntry {
        timestamp: Utc::now().to_rfc3339(),
        pid: pid.clone(),
        status: status.clone(),
        success: all_pass,
        pass_count,
        test_count,
        compile: compile_info,
        tests,
    };
    storage::append_judge_log(&entry)?;

    println!("Summary: {} ({}/{})", status, pass_count, test_count);

    if all_pass {
        Ok(())
    } else {
        std::process::exit(2);
    }
}
