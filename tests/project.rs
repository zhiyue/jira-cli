mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::project;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn list_projects() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/project"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"10000","key":"MGX","name":"MGX"}
        ])))
        .mount(&server)
        .await;

    in_blocking(move || {
        let list = project::list(&client).unwrap();
        assert_eq!(list.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn get_project() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/project/MGX"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"key":"MGX","name":"MGX"})))
        .mount(&server)
        .await;
    in_blocking(move || {
        let v = project::get(&client, "MGX").unwrap();
        assert_eq!(v["key"], "MGX");
    })
    .await;
}

#[tokio::test]
async fn statuses() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/project/MGX/statuses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"name":"Task","statuses":[{"id":"10000","name":"Open"}]}
        ])))
        .mount(&server)
        .await;
    in_blocking(move || {
        let v = project::statuses(&client, "MGX").unwrap();
        assert!(v.as_array().unwrap().len() == 1);
    })
    .await;
}
