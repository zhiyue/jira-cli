mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::link;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn add_link() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issueLink"))
        .and(body_partial_json(json!({
            "type": {"name": "Blocks"},
            "inwardIssue": {"key":"MGX-2"},
            "outwardIssue": {"key":"MGX-1"}
        })))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    in_blocking(move || {
        link::create(&client, "MGX-1", "MGX-2", "Blocks").unwrap();
    })
    .await;
}

#[tokio::test]
async fn delete_link() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("DELETE"))
        .and(path("/rest/api/2/issueLink/10050"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        link::delete(&client, "10050").unwrap();
    })
    .await;
}
