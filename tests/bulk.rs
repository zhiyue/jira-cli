mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::{comment, transitions};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn api_building_blocks_exist() {
    // Placeholder: the bulk commands only compose existing api functions via
    // the CLI layer + std::thread::scope. The behavioural test lives in the
    // CLI E2E suite (tests/cli_bulk_e2e.rs).
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/comment"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"1"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/transitions"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        comment::add(&client, "MGX-1", "x").unwrap();
        transitions::execute(&client, "MGX-1", "31", None).unwrap();
    })
    .await;
}
