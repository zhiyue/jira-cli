mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::watchers;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn list_watchers() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1/watchers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "watchCount": 1,
            "watchers": [{"name":"alice"}]
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = watchers::list(&client, "MGX-1").unwrap();
        assert_eq!(v["watchCount"], 1);
    })
    .await;
}

#[tokio::test]
async fn add_watcher_uses_quoted_body() {
    use wiremock::matchers::body_string;
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/watchers"))
        .and(body_string("\"bob\""))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        watchers::add(&client, "MGX-1", "bob").unwrap();
    })
    .await;
}

#[tokio::test]
async fn remove_watcher_query_string() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("DELETE"))
        .and(path("/rest/api/2/issue/MGX-1/watchers"))
        .and(query_param("username", "bob"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        watchers::remove(&client, "MGX-1", "bob").unwrap();
    })
    .await;
}
