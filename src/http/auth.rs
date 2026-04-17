//! Authentication header injection + response header inspection.

use crate::config::AuthConfig;
use crate::error::{AuthError, Error, Result};
use base64::Engine;
use reqwest::blocking::RequestBuilder;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE};

#[derive(Clone)]
pub struct Authenticator {
    mode: Mode,
}

#[derive(Clone)]
enum Mode {
    Basic(HeaderValue),
    Cookie(HeaderValue),
}

impl Authenticator {
    pub fn new(auth: &AuthConfig) -> Result<Self> {
        let mode = match auth {
            AuthConfig::Basic { user, password } => {
                let token =
                    base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
                let value = HeaderValue::from_str(&format!("Basic {token}"))
                    .map_err(|e| Error::Config(format!("basic auth header: {e}")))?;
                Mode::Basic(value)
            }
            AuthConfig::Cookie { cookie } => {
                let value = HeaderValue::from_str(cookie)
                    .map_err(|e| Error::Config(format!("cookie header: {e}")))?;
                Mode::Cookie(value)
            }
        };
        Ok(Self { mode })
    }

    pub fn inject(&self, req: RequestBuilder) -> RequestBuilder {
        match &self.mode {
            Mode::Basic(v) => req.header(AUTHORIZATION, v.clone()),
            Mode::Cookie(v) => req.header(COOKIE, v.clone()),
        }
    }

    pub fn is_cookie(&self) -> bool {
        matches!(self.mode, Mode::Cookie(_))
    }
}

/// Inspect response headers. Returns `Some(AuthError)` iff the response header
/// `X-Seraph-LoginReason` signals an auth/CAPTCHA failure. Spec §5.2.
pub fn detect_seraph(headers: &HeaderMap) -> Option<AuthError> {
    let raw = headers.get("X-Seraph-LoginReason")?;
    let val = raw.to_str().unwrap_or("").to_ascii_uppercase();
    if val.contains("CAPTCHA") {
        Some(AuthError::CaptchaRequired)
    } else if val.contains("AUTHENTICATION_DENIED") || val.contains("AUTHENTICATED_FAILED") {
        Some(AuthError::Unauthorized)
    } else {
        None
    }
}
