mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::issue;
use jira_cli::error::Error;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn get_happy_path() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "key": "MGX-1",
            "id": "10001",
            "fields": {"summary": "hello", "status": {"name": "Open"}}
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = issue::get(&client, "MGX-1", &issue::GetOpts::default()).unwrap();
        assert_eq!(v["key"], "MGX-1");
    })
    .await;
}

#[tokio::test]
async fn get_with_fields_and_expand_query() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-2"))
        .and(query_param("fields", "summary,status"))
        .and(query_param("expand", "changelog"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"key": "MGX-2"})))
        .mount(&server)
        .await;

    in_blocking(move || {
        let opts = issue::GetOpts {
            fields: vec!["summary".into(), "status".into()],
            expand: vec!["changelog".into()],
        };
        let v = issue::get(&client, "MGX-2", &opts).unwrap();
        assert_eq!(v["key"], "MGX-2");
    })
    .await;
}

#[tokio::test]
async fn get_404_becomes_not_found() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-404"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "errorMessages": ["Issue Does Not Exist"],
            "errors": {}
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let err = issue::get(&client, "MGX-404", &issue::GetOpts::default()).unwrap_err();
        match err {
            Error::NotFound { resource, key } => {
                assert_eq!(resource, "issue");
                assert_eq!(key, "MGX-404");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    })
    .await;
}
