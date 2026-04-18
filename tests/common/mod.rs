//! Common test helpers.
//!
//! reqwest::blocking::Client creates its own tokio runtime internally; that
//! runtime panics if dropped inside an existing tokio context.  We therefore
//! build the HttpClient in a true OS thread (via std::thread::spawn) so that
//! reqwest's internal runtime never sees the outer test runtime.

use jira_cli::config::{AuthConfig, JiraConfig};
use jira_cli::http::HttpClient;
use url::Url;
use wiremock::MockServer;

#[allow(dead_code)] // used in auth_basic.rs; appears unused in other test binaries
pub async fn spawn_mock_basic() -> (MockServer, HttpClient) {
    let server = MockServer::start().await;
    let uri = server.uri();
    let client = std::thread::spawn(move || {
        let cfg = JiraConfig {
            base_url: Url::parse(&uri).unwrap(),
            auth: AuthConfig::Basic {
                user: "alice".into(),
                password: "secret".into(),
            },
            timeout_secs: 5,
            insecure: false,
            concurrency: 4,
            field_aliases: Default::default(),
            defaults: Default::default(),
            field_renames: Default::default(),
            jql_aliases: Default::default(),
            default_project: None,
            effective_renames_cache: Default::default(),
        };
        HttpClient::new(&cfg).expect("build HttpClient")
    })
    .join()
    .expect("thread panicked");
    (server, client)
}

#[allow(dead_code)] // used in auth_cookie.rs; appears unused in other test binaries
pub async fn spawn_mock_cookie() -> (MockServer, HttpClient) {
    let server = MockServer::start().await;
    let uri = server.uri();
    let client = std::thread::spawn(move || {
        let cfg = JiraConfig {
            base_url: Url::parse(&uri).unwrap(),
            auth: AuthConfig::Cookie {
                cookie: "JSESSIONID=abc".into(),
            },
            timeout_secs: 5,
            insecure: false,
            concurrency: 4,
            field_aliases: Default::default(),
            defaults: Default::default(),
            field_renames: Default::default(),
            jql_aliases: Default::default(),
            default_project: None,
            effective_renames_cache: Default::default(),
        };
        HttpClient::new(&cfg).expect("build HttpClient")
    })
    .join()
    .expect("thread panicked");
    (server, client)
}

/// Run a closure containing blocking reqwest calls from within a tokio test.
///
/// Safe here because the HttpClient (and its internal reqwest runtime) was
/// already constructed on a plain OS thread by `spawn_mock_*`; by the time
/// this runs, reqwest's send path is just channel-sending into that existing
/// runtime and does not call `block_on` against the outer tokio context.
pub async fn in_blocking<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f).await.expect("blocking task")
}
