#[path = "cli/mod.rs"]
mod cli_helper;

use serde_json::json;
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
