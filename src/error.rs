//! Typed error hierarchy + stderr JSON serialization + exit-code mapping.
//!
//! All non-success exits go through `Error::emit_stderr()` to produce a
//! single-line JSON object on stderr, then `process::exit(err.exit_code())`.

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("config: {0}")]
    Config(String),

    #[error("usage: {0}")]
    Usage(String),

    #[error("auth: {0}")]
    Auth(AuthError),

    #[error("{resource} not found: {key}")]
    NotFound { resource: &'static str, key: String },

    #[error("api error (status {})", .0.status)]
    Api(ApiErrorBody),

    #[error("network: {0}")]
    Network(#[from] reqwest::Error),

    #[error("serialization: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("field: {0}")]
    FieldResolve(FieldError),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub enum AuthError {
    Unauthorized,
    Forbidden,
    CaptchaRequired,
    CookieExpired,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Unauthorized => f.write_str("unauthorized"),
            AuthError::Forbidden => f.write_str("forbidden"),
            AuthError::CaptchaRequired => f.write_str("CAPTCHA required"),
            AuthError::CookieExpired => f.write_str("cookie expired"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FieldError {
    Unknown(String),
    Ambiguous {
        name: String,
        candidates: Vec<String>,
    },
}

impl std::fmt::Display for FieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldError::Unknown(name) => write!(f, "unknown field '{name}'"),
            FieldError::Ambiguous { name, candidates } => write!(
                f,
                "field '{name}' is ambiguous (candidates: {})",
                candidates.join(", ")
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorBody {
    pub status: u16,
    #[serde(rename = "errorMessages")]
    pub error_messages: Vec<String>,
    pub errors: BTreeMap<String, String>,
    #[serde(rename = "request_id", skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ApiErrorBody {
    /// Parse a Jira error body. Falls back to a synthetic body containing the
    /// first 200 bytes of the raw text only if JSON parsing fails (Jira can
    /// return HTML for proxy errors, login pages, etc.). If the JSON parses
    /// cleanly, the parsed fields are preserved as-is — including the legitimate
    /// Jira 8.x case of `{"errorMessages":[], "errors":{}}`.
    pub fn from_bytes(status: u16, request_id: Option<String>, bytes: &[u8]) -> Self {
        #[derive(serde::Deserialize)]
        struct Raw {
            #[serde(default, rename = "errorMessages")]
            error_messages: Vec<String>,
            #[serde(default)]
            errors: BTreeMap<String, String>,
        }
        if let Ok(raw) = serde_json::from_slice::<Raw>(bytes) {
            return Self {
                status,
                error_messages: raw.error_messages,
                errors: raw.errors,
                request_id,
            };
        }
        let txt = String::from_utf8_lossy(bytes);
        let excerpt: String = txt.chars().take(200).collect();
        Self {
            status,
            error_messages: vec![excerpt],
            errors: BTreeMap::new(),
            request_id,
        }
    }
}

/// Heuristic: detect the non-JSON fallback shape produced by
/// `ApiErrorBody::from_bytes` — a single error_messages entry that begins with
/// `<` (HTML) or whitespace is almost certainly a raw text excerpt, not a real
/// Jira JSON error body.
fn looks_like_non_json_fallback(body: &ApiErrorBody) -> bool {
    body.errors.is_empty()
        && body.error_messages.len() == 1
        && body.error_messages[0]
            .chars()
            .next()
            .is_some_and(|c| c == '<' || c.is_ascii_whitespace())
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Config(_) | Error::Usage(_) | Error::FieldResolve(_) => 2,
            Error::Api(_) => 3,
            Error::Network(_) => 4,
            Error::Auth(_) => 5,
            Error::NotFound { .. } => 6,
            Error::Serialization(_) | Error::Io(_) => 7,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Error::Config(_) => "config",
            Error::Usage(_) => "usage",
            Error::Auth(_) => "auth",
            Error::NotFound { .. } => "not_found",
            Error::Api(_) => "api_error",
            Error::Network(_) => "network",
            Error::Serialization(_) => "serialization",
            Error::FieldResolve(_) => "field_resolve",
            Error::Io(_) => "io",
        }
    }

    /// Actionable hint for common cases; None when no hint beats a generic one.
    pub fn hint(&self) -> Option<String> {
        match self {
            Error::Auth(AuthError::Unauthorized) => Some(
                "Verify JIRA_USER / JIRA_PASSWORD, or regenerate cookie via `jira-cli session new`"
                    .into(),
            ),
            Error::Auth(AuthError::Forbidden) => Some(
                "The account authenticated successfully but lacks permission for this resource"
                    .into(),
            ),
            Error::Auth(AuthError::CaptchaRequired) => Some(
                "Log into Jira once via browser to clear CAPTCHA, then retry. \
                 Alternatively switch to cookie auth with JIRA_AUTH_METHOD=cookie."
                    .into(),
            ),
            Error::Auth(AuthError::CookieExpired) => {
                Some("Re-run `jira-cli session new` to obtain a fresh JSESSIONID".into())
            }
            Error::NotFound { resource, key } => Some(format!(
                "Verify the {resource} exists (e.g. `jira-cli search \"key = {key}\"`) \
                 or check permissions with `jira-cli whoami`"
            )),
            Error::Config(_) => Some("Ensure JIRA_URL is set and valid".into()),
            Error::Api(body) if looks_like_non_json_fallback(body) => Some(
                "non-JSON response from Jira; verify JIRA_URL points at the API root and auth is valid"
                    .into(),
            ),
            _ => None,
        }
    }

    pub fn to_stderr_json(&self) -> Value {
        let mut error = json!({
            "kind": self.kind(),
            "message": self.to_string(),
        });

        if let Error::Api(body) = self {
            error["status"] = body.status.into();
            error["errorMessages"] = json!(body.error_messages);
            error["errors"] = json!(body.errors);
            if let Some(rid) = &body.request_id {
                error["request_id"] = json!(rid);
            }
        }
        if let Some(hint) = self.hint() {
            error["hint"] = json!(hint);
        }
        json!({ "error": error })
    }

    /// Write the stderr JSON on one line and return the exit code.
    pub fn emit_stderr(&self) -> i32 {
        let v = self.to_stderr_json();
        let line = serde_json::to_string(&v).unwrap_or_else(|_| {
            r#"{"error":{"kind":"io","message":"failed to serialize error"}}"#.to_string()
        });
        eprintln!("{line}");
        self.exit_code()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_table() {
        assert_eq!(Error::Config("x".into()).exit_code(), 2);
        assert_eq!(Error::Usage("bad --set".into()).exit_code(), 2);
        assert_eq!(Error::Auth(AuthError::Unauthorized).exit_code(), 5);
        assert_eq!(Error::Auth(AuthError::CaptchaRequired).exit_code(), 5);
        assert_eq!(
            Error::NotFound {
                resource: "issue",
                key: "MGX-1".into()
            }
            .exit_code(),
            6
        );
        assert_eq!(
            Error::Api(ApiErrorBody {
                status: 400,
                error_messages: vec!["bad".into()],
                errors: Default::default(),
                request_id: None
            })
            .exit_code(),
            3
        );
    }

    #[test]
    fn error_body_from_json_standard() {
        let raw = r#"{"errorMessages":["Issue Does Not Exist"],"errors":{},"status":404}"#;
        let body = ApiErrorBody::from_bytes(404, None, raw.as_bytes());
        assert_eq!(body.error_messages, vec!["Issue Does Not Exist"]);
        assert_eq!(body.status, 404);
    }

    #[test]
    fn error_body_from_html_fallback() {
        let raw = b"<html><body>502 Bad Gateway</body></html>";
        let body = ApiErrorBody::from_bytes(502, Some("req-9".into()), raw);
        assert_eq!(body.status, 502);
        assert_eq!(body.request_id.as_deref(), Some("req-9"));
        assert!(body.error_messages[0].contains("502 Bad Gateway"));
    }

    #[test]
    fn stderr_json_shape_includes_kind_and_hint() {
        let err = Error::NotFound {
            resource: "issue",
            key: "MGX-42".into(),
        };
        let v = err.to_stderr_json();
        assert_eq!(v["error"]["kind"], "not_found");
        assert!(v["error"]["hint"].as_str().is_some());
        assert_eq!(v["error"]["message"], "issue not found: MGX-42");
    }

    #[test]
    fn from_bytes_preserves_empty_fields() {
        // Regression test for the silent-swallow bug.
        let raw = br#"{"errorMessages":[],"errors":{},"status":400}"#;
        let body = ApiErrorBody::from_bytes(400, None, raw);
        assert!(body.error_messages.is_empty());
        assert!(body.errors.is_empty());
        assert_eq!(body.status, 400);
    }

    #[test]
    fn stderr_json_for_api_error() {
        let err = Error::Api(ApiErrorBody {
            status: 409,
            error_messages: vec!["conflict".into()],
            errors: [("field".into(), "bad".into())].into_iter().collect(),
            request_id: Some("rid-1".into()),
        });
        let v = err.to_stderr_json();
        assert_eq!(v["error"]["kind"], "api_error");
        assert_eq!(v["error"]["status"], 409);
        assert_eq!(v["error"]["errorMessages"][0], "conflict");
        assert_eq!(v["error"]["errors"]["field"], "bad");
        assert_eq!(v["error"]["request_id"], "rid-1");
    }

    #[test]
    fn display_for_auth_and_field_errors() {
        let e = Error::Auth(AuthError::CaptchaRequired);
        assert_eq!(e.to_string(), "auth: CAPTCHA required");

        let e = Error::FieldResolve(FieldError::Ambiguous {
            name: "Story Points".into(),
            candidates: vec!["customfield_10020".into(), "customfield_10021".into()],
        });
        assert_eq!(
            e.to_string(),
            "field: field 'Story Points' is ambiguous (candidates: customfield_10020, customfield_10021)"
        );
    }
}
