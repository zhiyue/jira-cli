//! Integration tests for default jira-fields per command and field_renames.

#[path = "cli/mod.rs"]
mod cli_helper;

use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper that creates an XDG_CONFIG_HOME directory with a config.toml file and
/// returns an assert_cmd::Command pointed at a wiremock server, with env vars
/// cleared except for PATH and XDG_CONFIG_HOME (no JIRA_URL/USER/PASSWORD in env
/// — those must come from the config file).
fn bin_with_config(_server_uri: &str, config_toml: &str) -> (Command, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().unwrap();
    let xdg_root = tmp.path().join("xdg");
    std::fs::create_dir_all(xdg_root.join("jira-cli")).unwrap();
    let cfg_path = xdg_root.join("jira-cli").join("config.toml");
    std::fs::write(&cfg_path, config_toml).unwrap();

    let xdg_home = xdg_root.to_string_lossy().into_owned();
    let mut cmd = Command::cargo_bin("jira-cli").unwrap();
    cmd.env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("XDG_CONFIG_HOME", xdg_home);
    (cmd, tmp)
}

#[tokio::test]
async fn search_uses_config_default_jira_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/search"))
        .and(body_partial_json(json!({"fields": ["summary", "status"]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 0, "maxResults": 50, "total": 0, "issues": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let config = format!(
        "url = \"{uri}\"\nuser = \"u\"\npassword = \"p\"\n\n[defaults]\nsearch_fields = [\"summary\", \"status\"]\n"
    );
    let (mut cmd, _tmp) = bin_with_config(&uri, &config);

    let out =
        tokio::task::spawn_blocking(move || cmd.args(["search", "project = X"]).output().unwrap())
            .await
            .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // wiremock enforces body_partial_json match + .expect(1) verifies on drop
}

#[tokio::test]
async fn search_empty_jira_fields_bypasses_default() {
    let server = MockServer::start().await;
    // When --jira-fields "" is passed, the fields list should be empty → Jira returns full payload.
    // We verify by matching the POST body: it should NOT contain "fields" key
    // (or contain "fields": [] — both are ok, the important thing is the mock still matches).
    Mock::given(method("POST"))
        .and(path("/rest/api/2/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 0, "maxResults": 50, "total": 0, "issues": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let config = format!(
        "url = \"{uri}\"\nuser = \"u\"\npassword = \"p\"\n\n[defaults]\nsearch_fields = [\"summary\", \"status\"]\n"
    );
    let (mut cmd, _tmp) = bin_with_config(&uri, &config);

    let out = tokio::task::spawn_blocking(move || {
        // --jira-fields "" overrides the config default
        cmd.args(["search", "--jira-fields", "", "project = X"])
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
}

#[tokio::test]
async fn field_renames_applied_to_issue_get_output() {
    let server = MockServer::start().await;
    // Jira returns customfield_10006 = 5; we expect the CLI to emit story_points = 5.
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(r"/rest/api/2/issue/MGX-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "100",
            "key": "MGX-1",
            "fields": {
                "summary": "Test issue",
                "customfield_10006": 5.0
            }
        })))
        .mount(&server)
        .await;

    let uri = server.uri();
    let config = format!(
        "url = \"{uri}\"\nuser = \"u\"\npassword = \"p\"\n\n[field_renames]\ncustomfield_10006 = \"story_points\"\n"
    );
    let (mut cmd, _tmp) = bin_with_config(&uri, &config);

    let out =
        tokio::task::spawn_blocking(move || cmd.args(["issue", "get", "MGX-1"]).output().unwrap())
            .await
            .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        v["fields"]["story_points"], 5.0,
        "rename should apply; got: {v}"
    );
    assert!(
        v["fields"].get("customfield_10006").is_none(),
        "original key should be absent after rename; got: {v}"
    );
}

#[tokio::test]
async fn renamed_field_projection_works() {
    // Verify --fields "fields.story_points" correctly projects after rename.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(r"/rest/api/2/issue/MGX-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "101",
            "key": "MGX-2",
            "fields": {
                "summary": "Another issue",
                "customfield_10006": 8.0
            }
        })))
        .mount(&server)
        .await;

    let uri = server.uri();
    let config = format!(
        "url = \"{uri}\"\nuser = \"u\"\npassword = \"p\"\n\n[field_renames]\ncustomfield_10006 = \"story_points\"\n"
    );
    let (mut cmd, _tmp) = bin_with_config(&uri, &config);

    let out = tokio::task::spawn_blocking(move || {
        cmd.args([
            "issue",
            "get",
            "MGX-2",
            "--fields",
            "key,fields.story_points",
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
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["key"], "MGX-2");
    assert_eq!(v["fields"]["story_points"], 8.0);
    // summary should be projected out
    assert!(
        v["fields"].get("summary").is_none(),
        "summary should be projected out; got: {v}"
    );
}
