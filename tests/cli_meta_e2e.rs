#[path = "cli/mod.rs"]
mod cli_helper;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn ping_prints_server_info() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/serverInfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"version":"8.13.5"})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let output = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .arg("ping")
            .output()
            .expect("spawn")
    })
    .await
    .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["version"], "8.13.5");
}
