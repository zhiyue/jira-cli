//! `ping`, `whoami`, `config show`.

use crate::api::meta;
use crate::cli::args::GlobalArgs;
use crate::cli::ConfigCmd;
use crate::config::JiraConfig;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_value, Format};
use std::io::Write;

pub fn ping<W: Write>(out: &mut W, client: &HttpClient, g: &GlobalArgs) -> Result<()> {
    let value = meta::server_info(client)?;
    let fields = g.field_list();
    emit_value(
        out,
        value,
        &g.output_options(Format::Json, fields.as_deref()),
    )
}

pub fn whoami<W: Write>(out: &mut W, client: &HttpClient, g: &GlobalArgs) -> Result<()> {
    let value = meta::myself(client)?;
    let fields = g.field_list();
    emit_value(
        out,
        value,
        &g.output_options(Format::Json, fields.as_deref()),
    )
}

pub fn config_show<W: Write>(out: &mut W, cfg: &JiraConfig, g: &GlobalArgs) -> Result<()> {
    let fields = g.field_list();
    emit_value(
        out,
        cfg.redacted_json(),
        &g.output_options(Format::Json, fields.as_deref()),
    )
}

pub fn config(
    out: &mut impl Write,
    cfg: &JiraConfig,
    g: &GlobalArgs,
    cmd: &ConfigCmd,
) -> Result<()> {
    match cmd {
        ConfigCmd::Show => config_show(out, cfg, g),
    }
}

pub fn session_new<W: Write>(out: &mut W, cfg: &JiraConfig) -> Result<()> {
    use crate::api::session;
    let (user, pass) = read_credentials()?;
    // Build a temporary client with basic auth to call the session endpoint
    // (cookie auth can't bootstrap itself).
    let tmp_cfg = JiraConfig {
        base_url: cfg.base_url.clone(),
        auth: crate::config::AuthConfig::Basic {
            user: user.clone(),
            password: pass.clone(),
        },
        timeout_secs: cfg.timeout_secs,
        insecure: cfg.insecure,
        concurrency: cfg.concurrency,
    };
    let client = crate::http::HttpClient::new(&tmp_cfg)?;
    let info = session::new(&client, &user, &pass)?;
    writeln!(
        out,
        "{}",
        serde_json::json!({
            "ok": true,
            "cookie": info.cookie_header(),
            "name": info.name,
            "value": info.value
        })
    )?;
    Ok(())
}

fn read_credentials() -> Result<(String, String)> {
    use std::io::BufRead;
    let env: std::collections::HashMap<String, String> = std::env::vars().collect();
    let user = env
        .get("JIRA_USER")
        .cloned()
        .or_else(|| {
            let mut l = String::new();
            std::io::stdin().lock().read_line(&mut l).ok()?;
            Some(l.trim_end().to_string())
        })
        .ok_or_else(|| crate::error::Error::Usage("JIRA_USER not set and no stdin".into()))?;
    let pass = env
        .get("JIRA_PASSWORD")
        .cloned()
        .or_else(|| {
            let mut l = String::new();
            std::io::stdin().lock().read_line(&mut l).ok()?;
            Some(l.trim_end().to_string())
        })
        .ok_or_else(|| crate::error::Error::Usage("JIRA_PASSWORD not set and no stdin".into()))?;
    Ok((user, pass))
}
