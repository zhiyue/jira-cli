#[path = "cli/mod.rs"]
mod cli_helper;

use serde_json::json;
use wiremock::matchers::body_partial_json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn issue_get_roundtrip() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "key": "MGX-1",
            "fields": {"summary": "hello"}
        })))
        .mount(&server)
        .await;

    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args(["issue", "get", "MGX-1", "--fields", "key,fields.summary"])
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
    assert_eq!(v["fields"]["summary"], "hello");
}

#[tokio::test]
async fn issue_get_404_exits_6() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-404"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "errorMessages": ["Issue Does Not Exist"]
        })))
        .mount(&server)
        .await;

    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args(["issue", "get", "MGX-404"])
            .output()
            .unwrap()
    })
    .await
    .unwrap();

    assert_eq!(out.status.code(), Some(6));
    let stderr: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("stderr must be JSON");
    assert_eq!(stderr["error"]["kind"], "not_found");
}

#[tokio::test]
async fn search_keys_only_returns_only_keys() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/search"))
        .and(body_partial_json(json!({"startAt": 0})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 0, "maxResults": 100, "total": 2,
            "issues": [
                {"key": "MGX-1", "fields": {"summary": "First", "status": {"name": "Open"}}},
                {"key": "MGX-2", "fields": {"summary": "Second", "status": {"name": "Done"}}}
            ]
        })))
        .mount(&server)
        .await;

    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args(["search", "--keys-only", "project = MGX"])
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
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    // First two lines are issues, last is summary
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    // Only "key" should be present — summary/status projected out
    assert_eq!(first["key"], "MGX-1");
    assert!(
        first.get("fields").is_none(),
        "fields should be absent with --keys-only"
    );
}
