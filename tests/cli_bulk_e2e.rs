#[path = "cli/mod.rs"]
mod cli_helper;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn bulk_comment_happy_and_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/comment"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": "1"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-404/comment"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"errorMessages":["x"]})))
        .mount(&server)
        .await;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    {
        use std::io::Write;
        let mut f = std::fs::File::create(tmp.path()).unwrap();
        writeln!(f, r#"{{"key":"MGX-1","body":"ok"}}"#).unwrap();
        writeln!(f, r#"{{"key":"MGX-404","body":"oops"}}"#).unwrap();
    }

    let uri = server.uri();
    let file_path = tmp.path().to_string_lossy().into_owned();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args(["bulk", "comment", "--file", &file_path])
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
    let lines: Vec<serde_json::Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // 2 result lines + 1 summary
    assert_eq!(lines.len(), 3);
    let summary = &lines[2]["summary"];
    assert_eq!(
        summary["ok"].as_u64().unwrap() + summary["failed"].as_u64().unwrap(),
        2
    );
}
