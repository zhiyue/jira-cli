#[path = "cli/mod.rs"]
mod cli_helper;

use assert_cmd::Command;

#[tokio::test]
async fn config_init_non_interactive_writes_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let path = tmp.path().join("config.toml");
    let path_str = path.to_string_lossy().into_owned();

    let path_for_spawn = path_str.clone();
    let out = tokio::task::spawn_blocking(move || {
        Command::cargo_bin("jira-cli")
            .unwrap()
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .args([
                "config",
                "init",
                "--path",
                &path_for_spawn,
                "--url",
                "https://jira.example.com",
                "--user",
                "alice",
                "--password",
                "s3cr3t",
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

    // File exists with expected contents
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains(r#"url = "https://jira.example.com""#));
    assert!(contents.contains(r#"user = "alice""#));
    assert!(contents.contains(r#"password = "s3cr3t""#));

    // Unix-only: file mode is 0600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config should be written with 0600 mode");
    }
}
