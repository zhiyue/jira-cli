mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::issue;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn create_issue_happy() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue"))
        .and(body_partial_json(json!({
            "fields": {
                "project": {"key": "MGX"},
                "summary": "do the thing",
                "issuetype": {"name": "Task"}
            }
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": "10001",
            "key": "MGX-1",
            "self": "http://host/rest/api/2/issue/10001"
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let body = json!({
            "fields": {
                "project": {"key": "MGX"},
                "summary": "do the thing",
                "issuetype": {"name": "Task"}
            }
        });
        let v = issue::create(&client, &body).unwrap();
        assert_eq!(v["key"], "MGX-1");
    })
    .await;
}

#[tokio::test]
async fn update_issue_204() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("PUT"))
        .and(path("/rest/api/2/issue/MGX-1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        let body = json!({"fields": {"summary": "updated"}});
        issue::update(&client, "MGX-1", &body).unwrap();
    })
    .await;
}

#[tokio::test]
async fn delete_issue_204() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("DELETE"))
        .and(path("/rest/api/2/issue/MGX-1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        issue::delete(&client, "MGX-1").unwrap();
    })
    .await;
}

#[tokio::test]
async fn assign_to_user() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("PUT"))
        .and(path("/rest/api/2/issue/MGX-1/assignee"))
        .and(body_partial_json(json!({"name": "bob"})))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        issue::assign(&client, "MGX-1", Some("bob")).unwrap();
    })
    .await;
}

#[tokio::test]
async fn unassign() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("PUT"))
        .and(path("/rest/api/2/issue/MGX-1/assignee"))
        .and(body_partial_json(json!({"name": null})))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        issue::assign(&client, "MGX-1", None).unwrap();
    })
    .await;
}
