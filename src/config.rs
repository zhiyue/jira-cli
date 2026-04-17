//! Configuration resolved from environment variables. Stateless, read once
//! at process startup.

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
}

pub enum AuthConfig {
    Basic { user: String, password: String },
    Cookie { cookie: String },
}

impl JiraConfig {
    pub fn from_env() -> Result<Self> {
        let map: HashMap<String, String> = std::env::vars().collect();
        Self::from_map(&map)
    }

    pub fn from_map(env: &HashMap<String, String>) -> Result<Self> {
        let raw_url = env
            .get("JIRA_URL")
            .ok_or_else(|| Error::Config("JIRA_URL is required".into()))?;
        let base_url = Url::parse(raw_url)
            .map_err(|e| Error::Config(format!("JIRA_URL is not a valid URL: {e}")))?;
        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(Error::Config(
                "JIRA_URL must use http or https scheme".into(),
            ));
        }

        let method = env
            .get("JIRA_AUTH_METHOD")
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "basic".into());
        let auth = match method.as_str() {
            "basic" => {
                let user = env
                    .get("JIRA_USER")
                    .ok_or_else(|| Error::Config("JIRA_USER is required for basic auth".into()))?
                    .clone();
                let password = env
                    .get("JIRA_PASSWORD")
                    .ok_or_else(|| {
                        Error::Config("JIRA_PASSWORD is required for basic auth".into())
                    })?
                    .clone();
                AuthConfig::Basic { user, password }
            }
            "cookie" => {
                let cookie = env
                    .get("JIRA_SESSION_COOKIE")
                    .ok_or_else(|| {
                        Error::Config(
                            "JIRA_SESSION_COOKIE is required for cookie auth (e.g. 'JSESSIONID=abc')"
                                .into(),
                        )
                    })?
                    .clone();
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
            .unwrap_or(30);

        let insecure = parse_bool(env.get("JIRA_INSECURE").map(String::as_str))?;

        let concurrency = env
            .get("JIRA_CONCURRENCY")
            .map(|v| v.parse::<usize>())
            .transpose()
            .map_err(|e| Error::Config(format!("JIRA_CONCURRENCY: {e}")))?
            .unwrap_or(4)
            .clamp(1, 16);

        Ok(Self {
            base_url,
            auth,
            timeout_secs,
            insecure,
            concurrency,
        })
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
        })
    }
}

fn parse_bool(v: Option<&str>) -> Result<bool> {
    match v.map(str::to_lowercase).as_deref() {
        None | Some("") => Ok(false),
        Some("1" | "true" | "yes" | "on") => Ok(true),
        Some("0" | "false" | "no" | "off") => Ok(false),
        Some(other) => Err(Error::Config(format!(
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
}
