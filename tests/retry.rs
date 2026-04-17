mod common;

use common::{in_blocking, spawn_mock_basic};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn get_retries_on_429_with_retry_after() {
    let (server, client) = spawn_mock_basic().await;

    // First call: 429 Retry-After: 1
    Mock::given(method("GET"))
        .and(path("/rest/api/2/serverInfo"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Fallback: 200
    Mock::given(method("GET"))
        .and(path("/rest/api/2/serverInfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"version": "8.13.5"})))
        .mount(&server)
        .await;

    in_blocking(move || {
        let v: serde_json::Value = client.get_json("/rest/api/2/serverInfo").unwrap();
        assert_eq!(v["version"], "8.13.5");
    })
    .await;
}

#[tokio::test]
async fn get_retries_on_5xx_then_gives_up() {
    let (server, client) = spawn_mock_basic().await;

    Mock::given(method("GET"))
        .and(path("/rest/api/2/serverInfo"))
        .respond_with(ResponseTemplate::new(503))
        .expect(4) // initial + 3 retries
        .mount(&server)
        .await;

    in_blocking(move || {
        let err = client
            .get_json::<serde_json::Value>("/rest/api/2/serverInfo")
            .unwrap_err();
        assert_eq!(err.exit_code(), 3); // Api error
    })
    .await;
}

#[tokio::test]
async fn post_does_not_retry_on_5xx_by_default() {
    let (server, client) = spawn_mock_basic().await;

    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1) // exactly one call, no retry
        .mount(&server)
        .await;

    in_blocking(move || {
        let body = json!({"fields": {}});
        let err = client
            .post_json::<serde_json::Value, _>("/rest/api/2/issue", &body)
            .unwrap_err();
        assert_eq!(err.exit_code(), 3);
    })
    .await;
}
