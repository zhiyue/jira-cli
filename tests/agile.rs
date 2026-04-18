mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::agile;
use jira_cli::api::paging::PageParams;
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

#[tokio::test]
async fn create_sprint() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/agile/1.0/sprint"))
        .and(wiremock::matchers::body_partial_json(json!({
            "name": "Sprint 42",
            "originBoardId": 5
        })))
        .respond_with(
            ResponseTemplate::new(201)
                .set_body_json(json!({"id":100,"name":"Sprint 42","state":"future"})),
        )
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = agile::create_sprint(&client, 5, "Sprint 42", None, None, None).unwrap();
        assert_eq!(v["id"], 100);
    })
    .await;
}

#[tokio::test]
async fn move_issues_to_sprint_auto_batches_50() {
    let (server, client) = spawn_mock_basic().await;
    for _ in 0..3 {
        Mock::given(method("POST"))
            .and(path("/rest/agile/1.0/sprint/100/issue"))
            .respond_with(ResponseTemplate::new(204))
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
    }
    in_blocking(move || {
        let keys: Vec<String> = (0..120).map(|i| format!("MGX-{}", i + 1)).collect();
        agile::move_issues_to_sprint(&client, 100, &keys).unwrap();
    })
    .await;
}

#[tokio::test]
async fn epic_add_issues() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/agile/1.0/epic/MGX-1/issue"))
        .and(wiremock::matchers::body_partial_json(
            json!({"issues":["MGX-2","MGX-3"]}),
        ))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    in_blocking(move || {
        agile::epic_add_issues(&client, "MGX-1", &["MGX-2".into(), "MGX-3".into()]).unwrap();
    })
    .await;
}

#[tokio::test]
async fn epic_remove_issues_routes_to_none() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/agile/1.0/epic/none/issue"))
        .and(wiremock::matchers::body_partial_json(
            json!({"issues":["MGX-2"]}),
        ))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    in_blocking(move || {
        agile::epic_remove_issues(&client, &["MGX-2".into()]).unwrap();
    })
    .await;
}

#[tokio::test]
async fn backlog_move() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/agile/1.0/backlog/issue"))
        .and(wiremock::matchers::body_partial_json(
            json!({"issues":["MGX-1","MGX-2"]}),
        ))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    in_blocking(move || {
        agile::backlog_move(&client, &["MGX-1".into(), "MGX-2".into()]).unwrap();
    })
    .await;
}

#[tokio::test]
async fn list_boards_paginates_multiple_pages() {
    let (server, client) = spawn_mock_basic().await;

    // Page 1: 3 boards, total=5
    let page1: Vec<_> = (1..=3)
        .map(|i| json!({"id": i, "name": format!("Board {i}"), "type": "scrum"}))
        .collect();
    Mock::given(method("GET"))
        .and(path("/rest/agile/1.0/board"))
        .and(query_param("startAt", "0"))
        .and(query_param("maxResults", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 0, "maxResults": 3, "total": 5, "isLast": false,
            "values": page1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let page2: Vec<_> = (4..=5)
        .map(|i| json!({"id": i, "name": format!("Board {i}"), "type": "scrum"}))
        .collect();
    Mock::given(method("GET"))
        .and(path("/rest/agile/1.0/board"))
        .and(query_param("startAt", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 3, "maxResults": 3, "total": 5, "isLast": true,
            "values": page2
        })))
        .expect(1)
        .mount(&server)
        .await;

    in_blocking(move || {
        let params = PageParams {
            start_at: 0,
            page_size: 3,
            max: None,
        };
        let mut iter = agile::list_boards_paged(&client, None, None, params);
        let mut count = 0;
        for item in iter.by_ref() {
            item.unwrap();
            count += 1;
        }
        assert_eq!(count, 5);
        assert_eq!(iter.total(), Some(5));
    })
    .await;
}
