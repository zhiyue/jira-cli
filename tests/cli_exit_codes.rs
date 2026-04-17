#[path = "cli/mod.rs"]
mod cli_helper;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn parse_err(stderr: &[u8]) -> serde_json::Value {
    serde_json::from_slice(stderr).expect("stderr must be valid JSON")
}

#[tokio::test]
async fn exit_0_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/serverInfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"version":"8.13.5"})))
        .mount(&server)
        .await;
    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri).arg("ping").output().unwrap()
    })
    .await
    .unwrap();
    assert_eq!(out.status.code(), Some(0));
}

#[tokio::test]
async fn exit_2_usage_missing_env() {
    let out = tokio::task::spawn_blocking(|| {
        let mut c = assert_cmd::Command::cargo_bin("jira-cli").unwrap();
        c.env_clear().arg("ping").output().unwrap()
    })
    .await
    .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(parse_err(&out.stderr)["error"]["kind"], "config");
}

#[tokio::test]
async fn exit_3_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "errorMessages": ["bad input"],
            "errors": {"summary":"required"}
        })))
        .mount(&server)
        .await;
    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args(["issue", "create", "-p", "MGX", "-t", "Task", "-s", "x"])
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(parse_err(&out.stderr)["error"]["kind"], "api_error");
}

#[tokio::test]
async fn exit_4_network_unreachable() {
    let out = tokio::task::spawn_blocking(|| {
        cli_helper::bin_with_env("http://127.0.0.1:1")
            .arg("ping")
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert_eq!(parse_err(&out.stderr)["error"]["kind"], "network");
}

#[tokio::test]
async fn exit_5_auth_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/myself"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .arg("whoami")
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(out.status.code(), Some(5));
    assert_eq!(parse_err(&out.stderr)["error"]["kind"], "auth");
}

#[tokio::test]
async fn exit_6_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-404"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"errorMessages":["no"]})))
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
    assert_eq!(parse_err(&out.stderr)["error"]["kind"], "not_found");
}

#[tokio::test]
async fn exit_7_io_error_on_missing_bulk_file() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args(["bulk", "comment", "--file", "/nonexistent/path"])
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(out.status.code(), Some(7));
    assert_eq!(parse_err(&out.stderr)["error"]["kind"], "io");
}
