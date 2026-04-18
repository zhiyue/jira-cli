#[path = "cli/mod.rs"]
mod cli_helper;

#[tokio::test]
async fn project_list_respects_global_fields() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/rest/api/2/project"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"id":"1","key":"MGX","name":"MGX","projectTypeKey":"software"}
            ])),
        )
        .mount(&server)
        .await;

    let uri = server.uri();
    let out = tokio::task::spawn_blocking(move || {
        cli_helper::bin_with_env(&uri)
            .args(["project", "list", "--fields", "key,name"])
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
    // First line is the project, second is summary. Parse first line.
    let first = stdout.lines().next().unwrap();
    let v: serde_json::Value = serde_json::from_str(first).unwrap();
    // Projected fields present
    assert_eq!(v["key"], "MGX");
    assert_eq!(v["name"], "MGX");
    // Non-projected fields absent
    assert!(v.get("id").is_none(), "id should be filtered out, got: {v}");
    assert!(v.get("projectTypeKey").is_none());
}
