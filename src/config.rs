//! Configuration resolved from environment variables and optional TOML config file.
//! Precedence: CLI flags (handled in main.rs) > env vars > config file > defaults.

use crate::error::{Error, Result};
use std::collections::HashMap;
use url::Url;

/// Fully-validated config for a single Jira invocation.
pub struct JiraConfig {
    pub base_url: Url,
    pub auth: AuthConfig,
    pub timeout_secs: u64,
    pub insecure: bool,
    pub concurrency: usize,
    /// Display-name → field-id aliases loaded from [field_aliases] in config file.
    pub field_aliases: std::collections::HashMap<String, String>,
}

pub enum AuthConfig {
    Basic { user: String, password: String },
    Cookie { cookie: String },
}

/// TOML config file schema. All fields are optional — missing file is not an error.
#[derive(Debug, Default, serde::Deserialize)]
pub struct ConfigFile {
    pub url: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub auth_method: Option<String>,
    pub session_cookie: Option<String>,
    pub timeout_secs: Option<u64>,
    pub insecure: Option<bool>,
    pub concurrency: Option<usize>,
    /// Display-name → field-id aliases (e.g. "Story Points" = "customfield_10006").
    #[serde(default)]
    pub field_aliases: std::collections::HashMap<String, String>,
}

impl ConfigFile {
    /// Load from default path (~/.config/jira-cli/config.toml) if it exists.
    /// Returns Ok(Default) if the file doesn't exist or the home dir is unavailable.
    pub fn load_default() -> Result<Self> {
        let path = match Self::default_path() {
            Ok(p) => p,
            // If $HOME is not set (e.g. env_clear in tests), treat as no config file.
            Err(_) => return Ok(Self::default()),
        };
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::load_from(&path)
    }

    pub fn load_from(path: &std::path::Path) -> Result<Self> {
        let bytes = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("reading {}: {e}", path.display())))?;
        let cfg: Self = toml::from_str(&bytes)
            .map_err(|e| Error::Config(format!("parsing {}: {e}", path.display())))?;
        warn_insecure_permissions(path, &cfg);
        Ok(cfg)
    }

    pub fn default_path() -> Result<std::path::PathBuf> {
        // XDG: $XDG_CONFIG_HOME/jira-cli/config.toml, default ~/.config/jira-cli/config.toml
        let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg)
        } else {
            let home = std::env::var("HOME")
                .map_err(|_| Error::Config("$HOME is not set; cannot locate config file".into()))?;
            std::path::PathBuf::from(home).join(".config")
        };
        Ok(base.join("jira-cli").join("config.toml"))
    }
}

fn warn_insecure_permissions(path: &std::path::Path, cfg: &ConfigFile) {
    // Only warn if the file contains secrets.
    let has_secret = cfg.password.is_some() || cfg.session_cookie.is_some();
    if !has_secret {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mode = meta.permissions().mode() & 0o777;
            if mode & 0o077 != 0 {
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "warning": format!(
                            "config file {} has mode {:o}; recommend chmod 600 (contains secrets)",
                            path.display(), mode
                        )
                    })
                );
            }
        }
    }
}

impl JiraConfig {
    /// Load config by merging: env vars > config file > defaults.
    /// CLI flag overrides (--timeout, --insecure) are applied in main.rs after loading.
    pub fn load() -> Result<Self> {
        let file = ConfigFile::load_default()?;
        let env: HashMap<String, String> = std::env::vars().collect();
        Self::merge(&env, &file)
    }

    pub fn merge(env: &HashMap<String, String>, file: &ConfigFile) -> Result<Self> {
        let url = env
            .get("JIRA_URL")
            .cloned()
            .or_else(|| file.url.clone())
            .ok_or_else(|| {
                Error::Config(
                    "JIRA_URL is required (set env var or add to ~/.config/jira-cli/config.toml)"
                        .into(),
                )
            })?;
        let base_url = Url::parse(&url)
            .map_err(|e| Error::Config(format!("JIRA_URL is not a valid URL: {e}")))?;
        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(Error::Config(
                "JIRA_URL must use http or https scheme".into(),
            ));
        }

        let method = env
            .get("JIRA_AUTH_METHOD")
            .cloned()
            .or_else(|| file.auth_method.clone())
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "basic".into());

        let auth = match method.as_str() {
            "basic" => {
                let user = env
                    .get("JIRA_USER")
                    .cloned()
                    .or_else(|| file.user.clone())
                    .ok_or_else(|| Error::Config("JIRA_USER is required for basic auth".into()))?;
                let password = env
                    .get("JIRA_PASSWORD")
                    .cloned()
                    .or_else(|| file.password.clone())
                    .ok_or_else(|| {
                        Error::Config("JIRA_PASSWORD is required for basic auth".into())
                    })?;
                AuthConfig::Basic { user, password }
            }
            "cookie" => {
                let cookie = env
                    .get("JIRA_SESSION_COOKIE")
                    .cloned()
                    .or_else(|| file.session_cookie.clone())
                    .ok_or_else(|| {
                        Error::Config(
                            "JIRA_SESSION_COOKIE is required for cookie auth (e.g. 'JSESSIONID=abc')"
                                .into(),
                        )
                    })?;
                AuthConfig::Cookie { cookie }
            }
            other => {
                return Err(Error::Config(format!(
                    "JIRA_AUTH_METHOD must be 'basic' or 'cookie', got '{other}'"
                )));
            }
        };

        let timeout_secs = env
            .get("JIRA_TIMEOUT")
            .map(|v| v.parse::<u64>())
            .transpose()
            .map_err(|e| Error::Config(format!("JIRA_TIMEOUT: {e}")))?
            .or(file.timeout_secs)
            .unwrap_or(30);

        let insecure = match env.get("JIRA_INSECURE").map(String::as_str) {
            Some("") | None => file.insecure.unwrap_or(false),
            Some(v) => parse_bool_str(v)?,
        };

        let concurrency = env
            .get("JIRA_CONCURRENCY")
            .map(|v| v.parse::<usize>())
            .transpose()
            .map_err(|e| Error::Config(format!("JIRA_CONCURRENCY: {e}")))?
            .or(file.concurrency)
            .unwrap_or(4)
            .clamp(1, 16);

        let field_aliases = file.field_aliases.clone();

        Ok(Self {
            base_url,
            auth,
            timeout_secs,
            insecure,
            concurrency,
            field_aliases,
        })
    }

    /// Thin wrapper for backward compat with tests that call from_env.
    pub fn from_env() -> Result<Self> {
        Self::load()
    }

    /// Thin wrapper for backward compat with tests that call from_map.
    pub fn from_map(env: &HashMap<String, String>) -> Result<Self> {
        Self::merge(env, &ConfigFile::default())
    }

    /// Redacted JSON view for `config show`.
    pub fn redacted_json(&self) -> serde_json::Value {
        serde_json::json!({
            "base_url": self.base_url.as_str(),
            "auth_method": match self.auth { AuthConfig::Basic{..} => "basic", AuthConfig::Cookie{..} => "cookie" },
            "user": match &self.auth {
                AuthConfig::Basic { user, .. } => Some(user.as_str()),
                AuthConfig::Cookie { .. } => None,
            },
            "password_set": matches!(self.auth, AuthConfig::Basic{..}),
            "cookie_set": matches!(self.auth, AuthConfig::Cookie{..}),
            "timeout_secs": self.timeout_secs,
            "insecure": self.insecure,
            "concurrency": self.concurrency,
            "field_aliases": &self.field_aliases,
        })
    }
}

fn parse_bool_str(v: &str) -> Result<bool> {
    match v.to_lowercase().as_str() {
        "" => Ok(false),
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(Error::Config(format!(
            "expected boolean value, got '{other}'"
        ))),
    }
}

/// Manual Debug impl that redacts secrets — derive would dump them.
impl std::fmt::Debug for JiraConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JiraConfig")
            .field("base_url", &self.base_url.as_str())
            .field("auth", &self.auth)
            .field("timeout_secs", &self.timeout_secs)
            .field("insecure", &self.insecure)
            .field("concurrency", &self.concurrency)
            .field("field_aliases", &self.field_aliases)
            .finish()
    }
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthConfig::Basic { user, .. } => f
                .debug_struct("Basic")
                .field("user", user)
                .field("password", &"***")
                .finish(),
            AuthConfig::Cookie { .. } => f.debug_struct("Cookie").field("cookie", &"***").finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn basic_auth_minimum() {
        let cfg = JiraConfig::from_map(&env(&[
            ("JIRA_URL", "https://jira.example.com"),
            ("JIRA_USER", "alice"),
            ("JIRA_PASSWORD", "secret"),
        ]))
        .unwrap();
        assert_eq!(cfg.base_url.as_str(), "https://jira.example.com/");
        assert!(matches!(cfg.auth, AuthConfig::Basic { .. }));
        assert_eq!(cfg.timeout_secs, 30);
        assert!(!cfg.insecure);
    }

    #[test]
    fn cookie_auth() {
        let cfg = JiraConfig::from_map(&env(&[
            ("JIRA_URL", "https://jira.example.com"),
            ("JIRA_AUTH_METHOD", "cookie"),
            ("JIRA_SESSION_COOKIE", "JSESSIONID=abc123"),
        ]))
        .unwrap();
        match cfg.auth {
            AuthConfig::Cookie { cookie } => assert_eq!(cookie, "JSESSIONID=abc123"),
            _ => panic!("expected cookie auth"),
        }
    }

    #[test]
    fn missing_url_errors() {
        let e =
            JiraConfig::from_map(&env(&[("JIRA_USER", "a"), ("JIRA_PASSWORD", "b")])).unwrap_err();
        assert!(matches!(e, crate::error::Error::Config(_)));
    }

    #[test]
    fn invalid_url_errors() {
        let e = JiraConfig::from_map(&env(&[("JIRA_URL", "not a url")])).unwrap_err();
        assert!(matches!(e, crate::error::Error::Config(_)));
    }

    #[test]
    fn basic_missing_password_errors() {
        let e = JiraConfig::from_map(&env(&[
            ("JIRA_URL", "https://j.example"),
            ("JIRA_USER", "alice"),
        ]))
        .unwrap_err();
        assert!(matches!(e, crate::error::Error::Config(_)));
    }

    #[test]
    fn cookie_missing_cookie_errors() {
        let e = JiraConfig::from_map(&env(&[
            ("JIRA_URL", "https://j.example"),
            ("JIRA_AUTH_METHOD", "cookie"),
        ]))
        .unwrap_err();
        assert!(matches!(e, crate::error::Error::Config(_)));
    }

    #[test]
    fn redacted_debug() {
        let cfg = JiraConfig::from_map(&env(&[
            ("JIRA_URL", "https://j.example"),
            ("JIRA_USER", "alice"),
            ("JIRA_PASSWORD", "supersecret"),
        ]))
        .unwrap();
        let dbg = format!("{cfg:?}");
        assert!(!dbg.contains("supersecret"));
        assert!(dbg.contains("alice"));
    }

    #[test]
    fn timeout_and_insecure_parsed() {
        let cfg = JiraConfig::from_map(&env(&[
            ("JIRA_URL", "https://j.example"),
            ("JIRA_USER", "alice"),
            ("JIRA_PASSWORD", "p"),
            ("JIRA_TIMEOUT", "60"),
            ("JIRA_INSECURE", "1"),
        ]))
        .unwrap();
        assert_eq!(cfg.timeout_secs, 60);
        assert!(cfg.insecure);
    }

    #[test]
    fn file_plus_env_precedence() {
        let file = ConfigFile {
            url: Some("https://from-file.example".into()),
            user: Some("file-user".into()),
            password: Some("file-pass".into()),
            ..Default::default()
        };
        let env: HashMap<String, String> = [("JIRA_URL".into(), "https://from-env.example".into())]
            .into_iter()
            .collect();
        let cfg = JiraConfig::merge(&env, &file).unwrap();
        // Env wins over file for JIRA_URL:
        assert_eq!(cfg.base_url.as_str(), "https://from-env.example/");
        // File wins for user/password since env doesn't provide them:
        match cfg.auth {
            AuthConfig::Basic { user, password } => {
                assert_eq!(user, "file-user");
                assert_eq!(password, "file-pass");
            }
            _ => panic!("expected basic"),
        }
    }

    #[test]
    fn file_only_still_loads() {
        let file = ConfigFile {
            url: Some("https://j.example".into()),
            user: Some("alice".into()),
            password: Some("secret".into()),
            ..Default::default()
        };
        let cfg = JiraConfig::merge(&HashMap::new(), &file).unwrap();
        assert_eq!(cfg.base_url.as_str(), "https://j.example/");
    }

    #[test]
    fn config_file_parses_full_toml() {
        let raw = r#"
url = "https://j.example"
user = "alice"
password = "p"
timeout_secs = 60
insecure = true
concurrency = 8
"#;
        let cfg: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(cfg.url.as_deref(), Some("https://j.example"));
        assert_eq!(cfg.timeout_secs, Some(60));
        assert_eq!(cfg.insecure, Some(true));
        assert_eq!(cfg.concurrency, Some(8));
    }

    #[test]
    fn file_carries_field_aliases() {
        let raw = r#"
url = "https://j.example"
user = "alice"
password = "p"

[field_aliases]
"Story Points" = "customfield_10006"
"Epic Link" = "customfield_10000"
"#;
        let cfg: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            cfg.field_aliases.get("Story Points"),
            Some(&"customfield_10006".to_string())
        );
        assert_eq!(
            cfg.field_aliases.get("Epic Link"),
            Some(&"customfield_10000".to_string())
        );
    }

    #[test]
    fn jira_config_merges_aliases_from_file() {
        let file = ConfigFile {
            url: Some("https://j.example".into()),
            user: Some("alice".into()),
            password: Some("p".into()),
            field_aliases: [("Story Points".to_string(), "customfield_10006".to_string())]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let cfg = JiraConfig::merge(&HashMap::new(), &file).unwrap();
        assert_eq!(
            cfg.field_aliases.get("Story Points"),
            Some(&"customfield_10006".to_string())
        );
    }
}
