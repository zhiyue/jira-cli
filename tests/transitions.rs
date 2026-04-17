mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::transitions;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn list_transitions() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1/transitions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "transitions":[
                {"id":"21","name":"In Progress"},
                {"id":"31","name":"Done"}
            ]
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let list = transitions::list(&client, "MGX-1").unwrap();
        assert_eq!(list.transitions.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn transition_by_id_no_fields() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/transitions"))
        .and(body_partial_json(json!({"transition":{"id":"31"}})))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        transitions::execute(&client, "MGX-1", "31", None).unwrap();
    })
    .await;
}

#[tokio::test]
async fn transition_resolves_name() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1/transitions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "transitions":[{"id":"31","name":"Done"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/transitions"))
        .and(body_partial_json(json!({"transition":{"id":"31"}})))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        let id = transitions::resolve_name(&client, "MGX-1", "Done").unwrap();
        transitions::execute(&client, "MGX-1", &id, None).unwrap();
    })
    .await;
}
