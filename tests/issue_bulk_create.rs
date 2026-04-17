mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::issue;
use serde_json::json;
use wiremock::matchers::{body_json_schema, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn single_batch_under_50() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/bulk"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "issues": [
                {"id":"10001","key":"MGX-1"},
                {"id":"10002","key":"MGX-2"}
            ],
            "errors": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    in_blocking(move || {
        let input = vec![
            json!({"fields": {"project": {"key":"MGX"}, "summary":"a", "issuetype":{"name":"Task"}}}),
            json!({"fields": {"project": {"key":"MGX"}, "summary":"b", "issuetype":{"name":"Task"}}}),
        ];
        let results = issue::bulk_create(&client, &input).unwrap();
        assert_eq!(results.created.len(), 2);
        assert!(results.errors.is_empty());
    })
    .await;
}

#[tokio::test]
async fn auto_batches_over_50() {
    let (server, client) = spawn_mock_basic().await;
    // Expect 3 calls: 50 + 50 + 20
    for chunk in 0..3u32 {
        let base = chunk * 50;
        let issues: Vec<_> = (0..if chunk == 2 { 20 } else { 50 })
            .map(|i| json!({"id": format!("{}", 10000 + base + i), "key": format!("MGX-{}", base + i + 1)}))
            .collect();
        Mock::given(method("POST"))
            .and(path("/rest/api/2/issue/bulk"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "issues": issues,
                "errors": []
            })))
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
    }

    in_blocking(move || {
        let input: Vec<_> = (0..120)
            .map(|_| json!({"fields": {"project": {"key":"MGX"}, "summary":"x", "issuetype":{"name":"Task"}}}))
            .collect();
        let results = issue::bulk_create(&client, &input).unwrap();
        assert_eq!(results.created.len(), 120);
    })
    .await;

    // drop server triggers expectation verification.
    let _ = body_json_schema::<serde_json::Value>; // silence unused-import warning via ref
}
