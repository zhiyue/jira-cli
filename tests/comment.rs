mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::comment;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn list_comments() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1/comment"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 0, "maxResults": 50, "total": 2,
            "comments": [
                {"id":"10010","body":"first","author":{"name":"alice"}},
                {"id":"10011","body":"second","author":{"name":"bob"}}
            ]
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let page = comment::list(&client, "MGX-1").unwrap();
        assert_eq!(page.comments.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn add_comment() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/comment"))
        .and(body_partial_json(json!({"body": "hello"})))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({"id": "10020","body":"hello"})),
        )
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = comment::add(&client, "MGX-1", "hello").unwrap();
        assert_eq!(v["id"], "10020");
    })
    .await;
}

#[tokio::test]
async fn update_comment() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("PUT"))
        .and(path("/rest/api/2/issue/MGX-1/comment/10020"))
        .and(body_partial_json(json!({"body": "edited"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id":"10020","body":"edited"})),
        )
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = comment::update(&client, "MGX-1", "10020", "edited").unwrap();
        assert_eq!(v["body"], "edited");
    })
    .await;
}

#[tokio::test]
async fn delete_comment() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("DELETE"))
        .and(path("/rest/api/2/issue/MGX-1/comment/10020"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        comment::delete(&client, "MGX-1", "10020").unwrap();
    })
    .await;
}
