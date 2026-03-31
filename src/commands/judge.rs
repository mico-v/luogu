use crate::cli::JudgeArgs;
use crate::models::{JudgeCompileInfo, JudgeLogEntry, JudgeTestResult};
use crate::storage;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use colored::Colorize;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;
use wait_timeout::ChildExt;

fn normalize_text(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}

fn truncate_line(s: &str, limit: usize) -> String {
    if s.chars().count() <= limit {
        return s.to_string();
    }
    let mut out = String::new();
    for (idx, ch) in s.chars().enumerate() {
        if idx >= limit {
            break;
        }
        out.push(ch);
    }
    out.push_str("...");
    out
}

fn build_diff_message(expected: &str, actual: &str) -> String {
    let exp_lines: Vec<&str> = expected.lines().collect();
    let act_lines: Vec<&str> = actual.lines().collect();
    let max_len = exp_lines.len().max(act_lines.len());

    let mut diff_lines = Vec::new();
    diff_lines.push("--- expected".to_string());
    diff_lines.push("+++ actual".to_string());

    for i in 0..max_len {
        let e = exp_lines.get(i).copied().unwrap_or("");
        let a = act_lines.get(i).copied().unwrap_or("");
        if e == a {
            continue;
        }
        if !e.is_empty() {
            diff_lines.push(format!("-{:>4} | {}", i + 1, truncate_line(e, 200)));
        } else {
            diff_lines.push(format!("-{:>4} | <EOF>", i + 1));
        }
        if !a.is_empty() {
            diff_lines.push(format!("+{:>4} | {}", i + 1, truncate_line(a, 200)));
        } else {
            diff_lines.push(format!("+{:>4} | <EOF>", i + 1));
        }
    }

    if diff_lines.len() > 2 {
        return diff_lines.join("\n");
    }

    // Fallback for whitespace/token differences.
    format!(
        "token mismatch\nexpected(tokens): {:?}\nactual(tokens):   {:?}",
        normalize_text(expected),
        normalize_text(actual)
    )
}

fn parse_opt_level(raw: &str) -> Result<&'static str> {
    let normalized = raw.trim().to_ascii_uppercase();
    match normalized.as_str() {
        "NONE" | "O0" => Ok("O0"),
        "O1" => Ok("O1"),
        "O2" => Ok("O2"),
        "O3" => Ok("O3"),
        "OS" => Ok("Os"),
        _ => Err(anyhow!("unsupported --opt value: {} (supported: none/O1/O2/O3/Os)", raw)),
    }
}

#[derive(Debug)]
struct CompileConfig {
    language: String,
    source: String,
    cpp_std: String,
    optimization: String,
}

fn detect_language_and_source(problem_dir: &Path, language_override: Option<&str>, source_override: Option<&str>) -> Result<CompileConfig> {
    // If source is explicitly specified, determine language from extension
    if let Some(src) = source_override {
        let source_path = problem_dir.join(src);
        if !source_path.exists() {
            return Err(anyhow!("source file not found: {}", source_path.display()));
        }
        let lang = if src.ends_with(".cpp") || src.ends_with(".cc") || src.ends_with(".cxx") {
            "cpp"
        } else if src.ends_with(".py") {
            "python"
        } else {
            return Err(anyhow!("unsupported file extension: {}", src));
        };
        return Ok(CompileConfig {
            language: lang.to_string(),
            source: src.to_string(),
            cpp_std: "c++17".to_string(),
            optimization: "O2".to_string(),
        });
    }

    // If language is explicitly specified, find appropriate source file
    if let Some(lang) = language_override {
        let (candidates, expected_lang) = if lang.starts_with("c++") || lang == "cpp" {
            (vec!["main.cpp", "main.cc", "main.cxx"], "cpp")
        } else if lang == "python" || lang == "py" {
            (vec!["main.py"], "python")
        } else {
            return Err(anyhow!("unsupported language: {}", lang));
        };

        for candidate in candidates {
            if problem_dir.join(candidate).exists() {
                return Ok(CompileConfig {
                    language: expected_lang.to_string(),
                    source: candidate.to_string(),
                    cpp_std: "c++17".to_string(),
                    optimization: "O2".to_string(),
                });
            }
        }
        return Err(anyhow!("no source file found for language: {}", lang));
    }

    // Auto-detect: try cpp first, then python
    if problem_dir.join("main.cpp").exists() {
        return Ok(CompileConfig {
            language: "cpp".to_string(),
            source: "main.cpp".to_string(),
            cpp_std: "c++17".to_string(),
            optimization: "O2".to_string(),
        });
    }
    if problem_dir.join("main.py").exists() {
        return Ok(CompileConfig {
            language: "python".to_string(),
            source: "main.py".to_string(),
            cpp_std: String::new(),
            optimization: String::new(),
        });
    }

    Err(anyhow!("no source file found (main.cpp or main.py)"))
}

fn compile_cpp(
    problem_dir: &Path,
    source: &str,
    cpp_std: &str,
    optimization: &str,
    cflags: &[String],
) -> Result<JudgeCompileInfo> {
    let src_path = problem_dir.join(source);
    let bin_path = problem_dir.join(".luogu_solution");
    let start = Instant::now();

    let opt_flag = parse_opt_level(optimization)?;

    let mut cmd = Command::new("g++");
    cmd.arg(&src_path)
        .arg(format!("-std={}", cpp_std))
        .arg(format!("-{}", opt_flag))
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

fn compile_python(problem_dir: &Path, source: &str) -> Result<JudgeCompileInfo> {
    let src_path = problem_dir.join(source);
    let start = Instant::now();

    // Just check if Python file is valid syntax
    let out = Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&src_path)
        .output()
        .context("run python syntax check")?;

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

    // Detect language and source file
    let mut config = detect_language_and_source(&problem_dir, args.language.as_deref(), args.source.as_deref())?;

    // Override C++ standard if specified
    if let Some(std) = args.std {
        config.cpp_std = std;
    }

    // Override optimization level if specified
    if let Some(opt) = args.opt {
        config.optimization = opt;
    }

    // Compile
    let compile_info = if config.language == "cpp" {
        let opt_flag = parse_opt_level(&config.optimization)?;
        eprintln!("{} Compiling C++ with {} -{} ...", "⚙".cyan(), config.cpp_std, opt_flag);
        compile_cpp(&problem_dir, &config.source, &config.cpp_std, &config.optimization, &args.cflags)?
    } else {
        eprintln!("{} Checking Python syntax ...", "⚙".cyan());
        compile_python(&problem_dir, &config.source)?
    };

    if !compile_info.success {
        println!("{} {} - Compilation failed in {:.2}s", "✗".red(), pid, compile_info.elapsed_seconds);
        if !compile_info.stderr.trim().is_empty() {
            println!("\n{}", compile_info.stderr);
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
    eprintln!("{} Compiled in {:.2}s", "✓".green(), compile_info.elapsed_seconds);

    let samples = collect_samples(&problem_dir);
    if samples.is_empty() {
        return Err(anyhow!("no sample *.in files in {}", problem_dir.display()));
    }

    eprintln!("{} Running {} samples ...", "▶".cyan(), samples.len());

    let mut tests = Vec::new();
    let mut pass_count = 0usize;

    // Determine binary path based on language
    let bin_path = if config.language == "cpp" {
        problem_dir.join(".luogu_solution")
    } else {
        problem_dir.join(&config.source)
    };

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

        let mut cmd = if config.language == "cpp" {
            Command::new(&bin_path)
        } else {
            let mut cmd = Command::new("python3");
            cmd.arg(&bin_path);
            cmd
        };

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("spawn {}", if config.language == "cpp" { bin_path.display().to_string() } else { "python3".to_string() }))?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(input_text.as_bytes())?;
        }

        let timeout = std::time::Duration::from_secs_f64(args.timeout);
        let status = child.wait_timeout(timeout)?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        let test = if status.is_none() {
            let _ = child.kill();
            let _ = child.wait();
            JudgeTestResult {
                name,
                status: "TLE".to_string(),
                time_ms: Some(elapsed_ms),
                message: format!("timeout after {:.2}s", args.timeout),
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
                let diff_msg = build_diff_message(&expected, &actual);
                JudgeTestResult {
                    name,
                    status: "WA".to_string(),
                    time_ms: Some(elapsed_ms),
                    message: format!("wrong answer\n{}", diff_msg),
                }
            }
        };
        if test.status != "AC" {
            println!("[{}] {}", test.name, test.status);
            println!("{}", test.message);
            println!();
        }
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

    println!();
    if all_pass {
        println!("{} {} - {}/{} {} - All tests passed!", "✓".green(), pid, pass_count, test_count, "AC".green());
    } else {
        println!("{} {} - {}/{} {} - Some tests failed", "✗".red(), pid, pass_count, test_count, "FAILED".red());
    }

    if all_pass {
        Ok(())
    } else {
        std::process::exit(2);
    }
}
