mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::worklog;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn add_worklog() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/worklog"))
        .and(body_partial_json(
            json!({"timeSpent":"1h 30m","comment":"debug"}),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"30010"})))
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = worklog::add(&client, "MGX-1", "1h 30m", None, Some("debug")).unwrap();
        assert_eq!(v["id"], "30010");
    })
    .await;
}

#[tokio::test]
async fn delete_worklog() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("DELETE"))
        .and(path("/rest/api/2/issue/MGX-1/worklog/30010"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        worklog::delete(&client, "MGX-1", "30010").unwrap();
    })
    .await;
}
