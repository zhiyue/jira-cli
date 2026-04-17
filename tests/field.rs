mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::field;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn list_fields() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/field"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"summary","name":"Summary","custom":false,"clauseNames":["summary"]},
            {"id":"customfield_10020","name":"Story Points","custom":true,"clauseNames":["cf[10020]"]},
            {"id":"customfield_10021","name":"Story Points","custom":true,"clauseNames":["cf[10021]"]}
        ])))
        .mount(&server)
        .await;

    in_blocking(move || {
        let list = field::list(&client).unwrap();
        assert_eq!(list.len(), 3);
        assert!(list.iter().any(|f| f.id == "summary"));
    })
    .await;
}
