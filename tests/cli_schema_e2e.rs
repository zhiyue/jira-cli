#[path = "cli/mod.rs"]
mod cli_helper;

#[tokio::test]
async fn schema_prints_full_tree() {
    let out = tokio::task::spawn_blocking(|| {
        cli_helper::bin_with_env("http://localhost:1")
            .args(["schema"])
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
    assert!(v["version"].is_string());
    assert!(v["commands"]["ping"].is_object());
    assert!(v["commands"]["issue"]["subcommands"]["get"].is_object());
}

#[tokio::test]
async fn schema_single_subcommand() {
    let out = tokio::task::spawn_blocking(|| {
        cli_helper::bin_with_env("http://localhost:1")
            .args(["schema", "ping"])
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
    // Either an object with about/args/flags, or a compact alias; require `about` key.
    assert!(v["about"].is_string());
}
