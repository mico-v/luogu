use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Project-level config file (created on first use if missing).
const CONFIG_FILE: &str = "luogu_config.json";

// Defaults follow the competitest.nvim convention:
//   <problem>/testcases/<source-stem>.<n>.in / .ans   (n starts at 0)
fn default_testcases_dir() -> String {
    "testcases".to_string()
}

fn default_input_format() -> String {
    "$(FNOEXT).$(TCNUM).in".to_string()
}

fn default_output_format() -> String {
    "$(FNOEXT).$(TCNUM).ans".to_string()
}

fn default_source_file() -> String {
    "main.cpp".to_string()
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    template: TemplateConfig,
    #[serde(default)]
    testcases: TestcasesConfig,
    /// Source filename scaffolded for new problems (its stem seeds `$(FNOEXT)`).
    #[serde(default = "default_source_file")]
    source: String,
}

#[derive(Debug, Deserialize, Default)]
struct TemplateConfig {
    /// Inline template source code. Takes priority over `path` when non-empty.
    #[serde(default)]
    code: Option<String>,
    /// Path to a template file (absolute, or relative to the config file).
    #[serde(default)]
    path: Option<String>,
}

/// Testcase storage settings, aligned with competitest.nvim conventions.
#[derive(Debug, Clone, Deserialize)]
pub struct TestcasesConfig {
    /// Subdirectory (relative to the problem dir) holding testcases.
    #[serde(default = "default_testcases_dir")]
    pub directory: String,
    /// Input filename template. Supports `$(FNOEXT)` and `$(TCNUM)`.
    #[serde(default = "default_input_format")]
    pub input_format: String,
    /// Expected-output filename template. Supports `$(FNOEXT)` and `$(TCNUM)`.
    #[serde(default = "default_output_format")]
    pub output_format: String,
}

impl Default for TestcasesConfig {
    fn default() -> Self {
        Self {
            directory: default_testcases_dir(),
            input_format: default_input_format(),
            output_format: default_output_format(),
        }
    }
}

impl TestcasesConfig {
    pub fn directory(&self) -> &str {
        &self.directory
    }

    pub fn input_name(&self, fnoext: &str, tcnum: usize) -> String {
        expand_format(&self.input_format, fnoext, tcnum)
    }

    pub fn output_name(&self, fnoext: &str, tcnum: usize) -> String {
        expand_format(&self.output_format, fnoext, tcnum)
    }

    pub fn input_path(&self, dir: &Path, fnoext: &str, tcnum: usize) -> PathBuf {
        dir.join(self.input_name(fnoext, tcnum))
    }

    pub fn output_path(&self, dir: &Path, fnoext: &str, tcnum: usize) -> PathBuf {
        dir.join(self.output_name(fnoext, tcnum))
    }

    /// Regex matching input filenames, capturing `fnoext` and `tcnum`.
    pub fn input_regex(&self) -> Result<Regex> {
        let pattern = format_to_regex(&self.input_format);
        Regex::new(&pattern)
            .map_err(|e| anyhow!("invalid testcases input_format {:?}: {e}", self.input_format))
    }
}

/// Resolved project configuration.
#[derive(Debug, Clone)]
pub struct Config {
    template: Option<String>,
    testcases: TestcasesConfig,
    source: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            template: None,
            testcases: TestcasesConfig::default(),
            source: default_source_file(),
        }
    }
}

impl Config {
    /// Initial source code to scaffold for new problems, if configured.
    pub fn template(&self) -> Option<&str> {
        self.template.as_deref()
    }

    /// Testcase storage settings.
    pub fn testcases(&self) -> &TestcasesConfig {
        &self.testcases
    }

    /// Source filename used for new problems (default `main.cpp`).
    pub fn source(&self) -> &str {
        &self.source
    }
}

fn config_path() -> PathBuf {
    PathBuf::from(CONFIG_FILE)
}

fn default_config_text() -> Result<String> {
    let default = serde_json::json!({
        "source": default_source_file(),
        "template": {
            "code": "",
            "path": ""
        },
        "testcases": {
            "directory": default_testcases_dir(),
            "input_format": default_input_format(),
            "output_format": default_output_format()
        }
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&default)?))
}

/// Load the project config, creating a default file when missing.
pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        fs::write(&path, default_config_text()?)
            .with_context(|| format!("write default {}", path.display()))?;
    }

    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file_cfg: ConfigFile =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;

    let template = resolve_template(&file_cfg.template, &path)?;
    Ok(Config {
        template,
        testcases: file_cfg.testcases,
        source: file_cfg.source,
    })
}

fn resolve_template(template: &TemplateConfig, config_path: &Path) -> Result<Option<String>> {
    // Inline code has priority when set.
    if let Some(code) = template.code.as_deref() {
        if !code.is_empty() {
            return Ok(Some(code.to_string()));
        }
    }

    // Otherwise fall back to an external template file.
    if let Some(raw_path) = template.path.as_deref() {
        let raw_path = raw_path.trim();
        if !raw_path.is_empty() {
            let tpl_path = Path::new(raw_path);
            let tpl_path = if tpl_path.is_absolute() {
                tpl_path.to_path_buf()
            } else {
                config_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(tpl_path)
            };
            let content = fs::read_to_string(&tpl_path)
                .with_context(|| format!("read template {}", tpl_path.display()))?;
            return Ok(Some(content));
        }
    }

    Ok(None)
}

/// Expand `$(FNOEXT)` / `$(TCNUM)` placeholders in a filename template.
fn expand_format(fmt: &str, fnoext: &str, tcnum: usize) -> String {
    let mut out = String::with_capacity(fmt.len() + fnoext.len());
    let mut rest = fmt;
    while let Some(pos) = rest.find("$(") {
        out.push_str(&rest[..pos]);
        let after = &rest[pos..];
        match after.find(')') {
            Some(end) => {
                match &after[2..end] {
                    "FNOEXT" => out.push_str(fnoext),
                    "TCNUM" => out.push_str(&tcnum.to_string()),
                    _ => out.push_str(&after[..=end]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(after);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Turn a filename template into an anchored regex with `fnoext` / `tcnum` groups.
fn format_to_regex(fmt: &str) -> String {
    let mut pat = String::from("^");
    let mut rest = fmt;
    while let Some(pos) = rest.find("$(") {
        pat.push_str(&regex::escape(&rest[..pos]));
        let after = &rest[pos..];
        match after.find(')') {
            Some(end) => {
                match &after[2..end] {
                    "FNOEXT" => pat.push_str("(?P<fnoext>.+?)"),
                    "TCNUM" => pat.push_str("(?P<tcnum>[0-9]+)"),
                    _ => pat.push_str(&regex::escape(&after[..=end])),
                }
                rest = &after[end + 1..];
            }
            None => {
                pat.push_str(&regex::escape(after));
                rest = "";
            }
        }
    }
    pat.push_str(&regex::escape(rest));
    pat.push('$');
    pat
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tpl(code: Option<&str>, path: Option<&str>) -> TemplateConfig {
        TemplateConfig {
            code: code.map(str::to_string),
            path: path.map(str::to_string),
        }
    }

    #[test]
    fn none_when_empty() {
        let cfg = resolve_template(&tpl(None, None), Path::new("luogu_config.json")).unwrap();
        assert_eq!(cfg, None);
        let cfg = resolve_template(&tpl(Some(""), Some("")), Path::new("luogu_config.json"))
            .unwrap();
        assert_eq!(cfg, None);
    }

    #[test]
    fn inline_code_wins() {
        let cfg = resolve_template(
            &tpl(Some("int main(){}"), Some("missing.cpp")),
            Path::new("luogu_config.json"),
        )
        .unwrap();
        assert_eq!(cfg.as_deref(), Some("int main(){}"));
    }

    #[test]
    fn reads_external_path() {
        let dir = std::env::temp_dir().join(format!("luogu_cfg_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("luogu_config.json");
        fs::write(dir.join("main.cpp"), "// template\n").unwrap();
        let cfg = resolve_template(&tpl(None, Some("main.cpp")), &config_path).unwrap();
        assert_eq!(cfg.as_deref(), Some("// template\n"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_testcase_names() {
        let tc = TestcasesConfig::default();
        assert_eq!(tc.directory(), "testcases");
        assert_eq!(tc.input_name("main", 0), "main.0.in");
        assert_eq!(tc.output_name("main", 0), "main.0.ans");
        assert_eq!(tc.input_name("main", 12), "main.12.in");
    }

    #[test]
    fn parses_input_names() {
        let tc = TestcasesConfig::default();
        let re = tc.input_regex().unwrap();

        let caps = re.captures("main.7.in").unwrap();
        assert_eq!(caps.name("fnoext").unwrap().as_str(), "main");
        assert_eq!(caps.name("tcnum").unwrap().as_str(), "7");

        // Output files and the legacy layout must not match.
        assert!(re.captures("main.7.ans").is_none());
        assert!(re.captures("sample1.in").is_none());
    }

    #[test]
    fn default_config_text_has_testcases() {
        let text = default_config_text().unwrap();
        let cfg: ConfigFile = serde_json::from_str(&text).unwrap();
        assert_eq!(cfg.source, "main.cpp");
        assert_eq!(cfg.testcases.directory, "testcases");
        assert_eq!(cfg.testcases.input_format, "$(FNOEXT).$(TCNUM).in");
        assert_eq!(cfg.testcases.output_format, "$(FNOEXT).$(TCNUM).ans");
    }

    #[test]
    fn testcases_section_is_optional() {
        let cfg: ConfigFile =
            serde_json::from_str(r#"{"template":{"code":"","path":""}}"#).unwrap();
        assert_eq!(cfg.source, "main.cpp");
        assert_eq!(cfg.testcases.directory, "testcases");
        assert_eq!(cfg.testcases.input_format, "$(FNOEXT).$(TCNUM).in");

        let cfg: ConfigFile =
            serde_json::from_str(r#"{"testcases":{"directory":"tc"}}"#).unwrap();
        assert_eq!(cfg.source, "main.cpp");
        assert_eq!(cfg.testcases.directory, "tc");
        assert_eq!(cfg.testcases.input_format, "$(FNOEXT).$(TCNUM).in");
        assert_eq!(cfg.testcases.output_format, "$(FNOEXT).$(TCNUM).ans");
    }
}
