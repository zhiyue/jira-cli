//! Blocking HTTP client: reqwest wrapper with auth + standard headers.
//!
//! Error conversion: every response goes through `check_status` which turns
//! non-2xx into typed `Error::{Api, NotFound, Auth}` with a parsed body and
//! request-id header preserved.

pub mod auth;
pub mod retry; // populated in Task 6

use crate::config::JiraConfig;
use crate::error::{ApiErrorBody, AuthError, Error, Result};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use url::Url;

const USER_AGENT_VALUE: &str = concat!("jira-cli/", env!("CARGO_PKG_VERSION"));

#[derive(Clone)]
pub struct HttpClient {
    inner: Client,
    base_url: Url,
    auth: auth::Authenticator,
    retry_writes: bool,
}

impl HttpClient {
    pub fn new(cfg: &JiraConfig) -> Result<Self> {
        let builder = Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .danger_accept_invalid_certs(cfg.insecure)
            .cookie_store(true)
            .user_agent(USER_AGENT_VALUE)
            .gzip(true);
        let inner = builder.build()?;
        Ok(Self {
            inner,
            base_url: cfg.base_url.clone(),
            auth: auth::Authenticator::new(&cfg.auth)?,
            retry_writes: false,
        })
    }

    pub fn with_retry_writes(mut self, yes: bool) -> Self {
        self.retry_writes = yes;
        self
    }

    pub fn retry_writes_enabled(&self) -> bool {
        self.retry_writes
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Build an absolute URL from a path like `/rest/api/2/issue/MGX-1`.
    ///
    /// Ensures the base URL ends with `/` so path joining preserves any
    /// sub-path in `JIRA_URL` (e.g. `https://host/jira`).
    pub fn url(&self, path: &str) -> Result<Url> {
        let base = if self.base_url.path().ends_with('/') {
            self.base_url.clone()
        } else {
            let mut b = self.base_url.clone();
            let new_path = format!("{}/", b.path());
            b.set_path(&new_path);
            b
        };
        base.join(path.trim_start_matches('/'))
            .map_err(|e| Error::Config(format!("invalid URL path {path:?}: {e}")))
    }

    fn prepare(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        let url = self.url(path)?;
        let mut builder = self.inner.request(method, url);
        builder = self.auth.inject(builder);
        builder = builder
            .header(ACCEPT, HeaderValue::from_static("application/json"))
            .header("X-Atlassian-Token", HeaderValue::from_static("no-check"))
            .header(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
        Ok(builder)
    }

    pub fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let req = self.prepare(Method::GET, path)?;
        let resp = retry::send(self, req, true)?;
        decode_json(resp)
    }

    pub fn get_json_query<T, Q>(&self, path: &str, query: &Q) -> Result<T>
    where
        T: DeserializeOwned,
        Q: Serialize,
    {
        let req = self.prepare(Method::GET, path)?.query(query);
        let resp = retry::send(self, req, true)?;
        decode_json(resp)
    }

    pub fn post_json<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let req = self.prepare(Method::POST, path)?.json(body);
        let resp = retry::send(self, req, false)?;
        decode_json(resp)
    }

    pub fn put_json<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let req = self.prepare(Method::PUT, path)?.json(body);
        let resp = retry::send(self, req, false)?;
        decode_json(resp)
    }

    pub fn delete(&self, path: &str) -> Result<()> {
        let req = self.prepare(Method::DELETE, path)?;
        let resp = retry::send(self, req, false)?;
        check_status_no_body(resp)
    }

    /// POST returning an empty body (204/200 with no JSON).
    pub fn post_empty<B: Serialize>(&self, path: &str, body: &B) -> Result<()> {
        let req = self.prepare(Method::POST, path)?.json(body);
        let resp = retry::send(self, req, false)?;
        check_status_no_body(resp)
    }

    pub fn request_builder(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        self.prepare(method, path)
    }

    pub fn send(&self, req: RequestBuilder, is_idempotent: bool) -> Result<Response> {
        retry::send(self, req, is_idempotent)
    }

    pub fn is_cookie_auth(&self) -> bool {
        self.auth.is_cookie()
    }
}

fn decode_json<T: DeserializeOwned>(resp: Response) -> Result<T> {
    let resp = check_status(resp)?;
    let bytes = resp.bytes()?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn check_status_no_body(resp: Response) -> Result<()> {
    check_status(resp).map(|_| ())
}

pub(crate) fn check_status(resp: Response) -> Result<Response> {
    if let Some(err) = auth::detect_seraph(resp.headers()) {
        return Err(Error::Auth(err));
    }
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let request_id = header_string(resp.headers(), "X-AREQUESTID")
        .or_else(|| header_string(resp.headers(), "X-ARequestId"));
    let code = status.as_u16();
    #[allow(clippy::match_same_arms)]
    match status {
        StatusCode::UNAUTHORIZED => Err(Error::Auth(AuthError::Unauthorized)),
        StatusCode::FORBIDDEN => Err(Error::Auth(AuthError::Forbidden)),
        StatusCode::NOT_FOUND => {
            // Callers that want a typed NotFound (e.g. api::issue::get) map the
            // generic Api(404) -> NotFound themselves because they know the
            // resource name.
            let bytes = resp.bytes().unwrap_or_default();
            Err(Error::Api(ApiErrorBody::from_bytes(
                code, request_id, &bytes,
            )))
        }
        _ => {
            let bytes = resp.bytes().unwrap_or_default();
            Err(Error::Api(ApiErrorBody::from_bytes(
                code, request_id, &bytes,
            )))
        }
    }
}

fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

#[cfg(test)]
mod url_tests {
    use super::*;
    use crate::config::{AuthConfig, JiraConfig};

    fn client_with_base(base: &str) -> HttpClient {
        let cfg = JiraConfig {
            base_url: Url::parse(base).unwrap(),
            auth: AuthConfig::Basic {
                user: "u".into(),
                password: "p".into(),
            },
            timeout_secs: 5,
            insecure: false,
            concurrency: 4,
        };
        HttpClient::new(&cfg).unwrap()
    }

    #[test]
    fn url_preserves_subpath_without_trailing_slash() {
        let c = client_with_base("https://host.example/jira");
        let u = c.url("/rest/api/2/serverInfo").unwrap();
        assert_eq!(
            u.as_str(),
            "https://host.example/jira/rest/api/2/serverInfo"
        );
    }

    #[test]
    fn url_with_trailing_slash_is_idempotent() {
        let c = client_with_base("https://host.example/jira/");
        let u = c.url("/rest/api/2/serverInfo").unwrap();
        assert_eq!(
            u.as_str(),
            "https://host.example/jira/rest/api/2/serverInfo"
        );
    }

    #[test]
    fn url_root_base() {
        let c = client_with_base("https://host.example");
        let u = c.url("/rest/api/2/serverInfo").unwrap();
        assert_eq!(u.as_str(), "https://host.example/rest/api/2/serverInfo");
    }

    #[test]
    fn url_handles_path_without_leading_slash() {
        let c = client_with_base("https://host.example/");
        let u = c.url("rest/api/2/serverInfo").unwrap();
        assert_eq!(u.as_str(), "https://host.example/rest/api/2/serverInfo");
    }
}
