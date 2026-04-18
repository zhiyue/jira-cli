#[path = "cli/mod.rs"]
mod cli_helper;

use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn raw_get_returns_pretty_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/anything"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args(["raw", "GET", "/rest/api/2/anything"])
            .output()
            .unwrap()
    })
    .await
    .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
}

#[tokio::test]
async fn raw_post_sends_body_and_query() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue"))
        .and(query_param("expand", "names"))
        .and(body_partial_json(json!({"fields": {"summary": "x"}})))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"key": "MGX-1"})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args([
                "raw",
                "POST",
                "/rest/api/2/issue",
                "-d",
                r#"{"fields":{"summary":"x"}}"#,
                "--query",
                "expand=names",
            ])
            .output()
            .unwrap()
    })
    .await
    .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["key"], "MGX-1");
}

#[tokio::test]
async fn raw_non_2xx_returns_api_error_exit_3() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/some/path"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_json(json!({"errorMessages": ["oops"], "errors": {}})),
        )
        .mount(&server)
        .await;
    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args(["raw", "GET", "/some/path"])
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(out.status.code(), Some(3));
}
