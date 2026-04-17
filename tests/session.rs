mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::session;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn new_session_returns_cookie_value() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/auth/1/session"))
        .and(body_partial_json(
            json!({"username":"alice","password":"secret"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "session": {"name":"JSESSIONID","value":"ABC123"},
            "loginInfo": {}
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let info = session::new(&client, "alice", "secret").unwrap();
        assert_eq!(info.name, "JSESSIONID");
        assert_eq!(info.value, "ABC123");
        assert_eq!(info.cookie_header(), "JSESSIONID=ABC123");
    })
    .await;
}
