mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::meta;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn server_info_returns_version() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/serverInfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "version": "8.13.5",
            "buildNumber": 813005,
            "baseUrl": "https://jira.example.com"
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let info = meta::server_info(&client).unwrap();
        assert_eq!(info["version"], "8.13.5");
    })
    .await;
}

#[tokio::test]
async fn myself_returns_user() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/myself"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "alice",
            "emailAddress": "alice@example.com",
            "displayName": "Alice Chen"
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let me = meta::myself(&client).unwrap();
        assert_eq!(me["name"], "alice");
    })
    .await;
}
