mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::search;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn paginates_through_three_pages() {
    let (server, client) = spawn_mock_basic().await;

    for page in 0..3u32 {
        let issues: Vec<_> = (0..if page < 2 { 100 } else { 50 })
            .map(|i| json!({"key": format!("MGX-{}", page * 100 + i + 1)}))
            .collect();

        Mock::given(method("POST"))
            .and(path("/rest/api/2/search"))
            .and(body_partial_json(json!({"startAt": page * 100})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "startAt": page * 100,
                "maxResults": 100,
                "total": 250,
                "issues": issues
            })))
            .expect(1)
            .mount(&server)
            .await;
    }

    in_blocking(move || {
        let iter = search::iter(
            &client,
            search::SearchParams {
                jql: "project = MGX".into(),
                fields: vec![],
                expand: vec![],
                max: None,
                page_size: 100,
            },
        );
        let mut count = 0;
        for item in iter {
            item.unwrap();
            count += 1;
        }
        assert_eq!(count, 250);
    })
    .await;
}

#[tokio::test]
async fn honours_max_cap() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 0,
            "maxResults": 100,
            "total": 500,
            "issues": (0..100).map(|i| json!({"key": format!("MGX-{}", i)})).collect::<Vec<_>>()
        })))
        .expect(1) // stops after 50 issues, single page is enough
        .mount(&server)
        .await;

    in_blocking(move || {
        let iter = search::iter(
            &client,
            search::SearchParams {
                jql: "project = MGX".into(),
                fields: vec![],
                expand: vec![],
                max: Some(50),
                page_size: 100,
            },
        );
        let mut count = 0;
        for _ in iter {
            count += 1;
        }
        assert_eq!(count, 50);
    })
    .await;
}
