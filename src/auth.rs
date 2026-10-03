use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const APP_DIR: &str = ".luogu";
const SESSION_FILE: &str = "session.json";
const CAPTCHA_FILE: &str = "login-captcha.jpg";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub cookie: String,
    pub username: Option<String>,
    pub uid: Option<i64>,
    pub saved_at: String,
}

fn app_dir() -> PathBuf {
    PathBuf::from(APP_DIR)
}

fn ensure_app_dir() -> Result<PathBuf> {
    let dir = app_dir();
    fs::create_dir_all(&dir).with_context(|| format!("create app dir {}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
            .with_context(|| format!("set permissions on {}", dir.display()))?;
    }
    Ok(dir)
}

pub fn session_path() -> PathBuf {
    app_dir().join(SESSION_FILE)
}

pub fn captcha_path() -> Result<PathBuf> {
    Ok(ensure_app_dir()?.join(CAPTCHA_FILE))
}

pub fn normalize_cookie_header(raw: &str) -> Result<String> {
    let mut text = raw.trim();
    if text
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("cookie:"))
    {
        text = text.get(7..).unwrap_or_default().trim();
    }

    let joined = text.replace(['\r', '\n'], " ");
    let mut pairs = Vec::new();
    for part in joined.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if !part.contains('=') {
            return Err(anyhow!("invalid cookie pair: {part}"));
        }
        pairs.push(part);
    }

    let has_uid = pairs.iter().any(|part| part.starts_with("_uid="));
    let has_client_id = pairs.iter().any(|part| part.starts_with("__client_id="));
    if !has_uid || !has_client_id {
        return Err(anyhow!(
            "cookie must contain both _uid and __client_id session cookies"
        ));
    }

    Ok(pairs.join("; "))
}

pub fn uid_from_cookie_header(cookie: &str) -> Option<i64> {
    cookie.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        if name == "_uid" {
            value.parse().ok()
        } else {
            None
        }
    })
}

pub fn save_session(cookie: &str, username: Option<&str>, uid: Option<i64>) -> Result<()> {
    let dir = ensure_app_dir()?;
    let path = dir.join(SESSION_FILE);
    let session = Session {
        cookie: normalize_cookie_header(cookie)?,
        username: username.map(str::to_string),
        uid,
        saved_at: chrono::Utc::now().to_rfc3339(),
    };
    let text = format!("{}\n", serde_json::to_string_pretty(&session)?);

    let tmp_path = path.with_extension("json.tmp");
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&tmp_path)
        .with_context(|| format!("open {}", tmp_path.display()))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("write {}", tmp_path.display()))?;
    file.sync_all()
        .with_context(|| format!("sync {}", tmp_path.display()))?;
    drop(file);
    fs::rename(&tmp_path, &path).with_context(|| format!("replace {}", path.display()))?;

    set_private_file_permissions(&path)?;
    Ok(())
}

pub fn load_session() -> Result<Option<Session>> {
    let path = session_path();
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let session: Session =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(session))
}

pub fn clear_session() -> Result<bool> {
    let path = session_path();
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    Ok(true)
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("set permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_cookie_header() {
        let cookie =
            normalize_cookie_header("Cookie: __client_id=client-token;\n _uid=12345; theme=light")
                .unwrap();
        assert_eq!(cookie, "__client_id=client-token; _uid=12345; theme=light");
        assert_eq!(uid_from_cookie_header(&cookie), Some(12345));
    }

    #[test]
    fn rejects_incomplete_session_cookie() {
        assert!(normalize_cookie_header("_uid=12345").is_err());
        assert!(normalize_cookie_header("__client_id=token").is_err());
    }
}
