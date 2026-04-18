//! CLI end-to-end helpers. Builds the binary via assert_cmd, points it at a
//! wiremock server and sets minimum env vars.

use assert_cmd::Command;

#[allow(dead_code)]
pub fn bin() -> Command {
    Command::cargo_bin("jira-cli").expect("binary built")
}

#[allow(dead_code)]
pub fn bin_with_env(base: &str) -> Command {
    let mut c = bin();
    c.env_clear()
        .env("JIRA_URL", base)
        .env("JIRA_USER", "alice")
        .env("JIRA_PASSWORD", "secret")
        .env("PATH", std::env::var("PATH").unwrap_or_default());
    c
}
