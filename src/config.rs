use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Project-level config file (created on first use if missing).
const CONFIG_FILE: &str = "luogu_config.json";

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    template: TemplateConfig,
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

/// Resolved project configuration.
#[derive(Debug, Clone, Default)]
pub struct Config {
    template: Option<String>,
}

impl Config {
    /// Initial source code to scaffold for new problems, if configured.
    pub fn template(&self) -> Option<&str> {
        self.template.as_deref()
    }
}

fn config_path() -> PathBuf {
    PathBuf::from(CONFIG_FILE)
}

fn default_config_text() -> Result<String> {
    let default = serde_json::json!({
        "template": {
            "code": "",
            "path": ""
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
    Ok(Config { template })
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
}
