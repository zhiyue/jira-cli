mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::agile;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn list_boards_filters_type() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/agile/1.0/board"))
        .and(query_param("type", "scrum"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "values":[{"id":5,"name":"Team Scrum","type":"scrum"}],
            "total":1,
            "isLast": true
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let list = agile::list_boards(&client, Some("scrum"), None).unwrap();
        assert_eq!(list.values.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn board_backlog_streams() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/agile/1.0/board/5/backlog"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt":0,"maxResults":50,"total":1,
            "issues":[{"key":"MGX-10"}]
        })))
        .mount(&server)
        .await;
    in_blocking(move || {
        let v = agile::board_backlog(&client, 5).unwrap();
        assert_eq!(v["issues"][0]["key"], "MGX-10");
    })
    .await;
}
