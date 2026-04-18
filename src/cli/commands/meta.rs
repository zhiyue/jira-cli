//! `ping`, `whoami`, `config show`.

use crate::api::meta;
use crate::cli::args::GlobalArgs;
use crate::cli::ConfigCmd;
use crate::config::JiraConfig;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_value, Format};
use std::io::Write;

pub fn ping<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
) -> Result<()> {
    let value = meta::server_info(client)?;
    let fields = g.field_list();
    let renames = cfg.effective_renames(client)?;
    emit_value(
        out,
        value,
        &g.output_options_with_renames(Format::Json, fields.as_deref(), Some(&renames)),
    )
}

pub fn whoami<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
) -> Result<()> {
    let value = meta::myself(client)?;
    let fields = g.field_list();
    let renames = cfg.effective_renames(client)?;
    emit_value(
        out,
        value,
        &g.output_options_with_renames(Format::Json, fields.as_deref(), Some(&renames)),
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
        ConfigCmd::Init(args) => config_init(out, args),
    }
}

pub fn config_init<W: Write>(out: &mut W, args: &crate::cli::ConfigInitArgs) -> Result<()> {
    use crate::error::Error;

    let path = match &args.path {
        Some(p) => p.clone(),
        None => crate::config::ConfigFile::default_path()?,
    };

    if path.exists() && !args.force {
        return Err(Error::Usage(format!(
            "config file already exists at {}; pass --force to overwrite",
            path.display()
        )));
    }

    let method = args
        .auth_method
        .clone()
        .unwrap_or_else(|| "basic".into())
        .to_lowercase();

    // Required fields depending on auth method
    let url = match args.url.clone() {
        Some(u) => u,
        None => prompt("JIRA base URL (e.g. https://jira.example.com): ")?,
    };

    let mut toml_lines: Vec<String> = vec![format!(r#"url = "{}""#, escape_toml(&url))];

    match method.as_str() {
        "basic" => {
            let user = match args.user.clone() {
                Some(u) => u,
                None => prompt("User: ")?,
            };
            let password = match args.password.clone() {
                Some(p) => p,
                None => prompt("Password (will be echoed): ")?,
            };
            toml_lines.push(format!(r#"user = "{}""#, escape_toml(&user)));
            toml_lines.push(format!(r#"password = "{}""#, escape_toml(&password)));
        }
        "cookie" => {
            toml_lines.push(r#"auth_method = "cookie""#.to_string());
            let cookie = match args.session_cookie.clone() {
                Some(c) => c,
                None => prompt("Session cookie (e.g. JSESSIONID=abc...): ")?,
            };
            toml_lines.push(format!(r#"session_cookie = "{}""#, escape_toml(&cookie)));
        }
        other => {
            return Err(Error::Usage(format!(
                "auth-method must be 'basic' or 'cookie', got '{other}'"
            )));
        }
    }

    if args.insecure {
        toml_lines.push("insecure = true".to_string());
    }

    let content = toml_lines.join("\n") + "\n";

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write with mode 0600 on Unix
    write_secure(&path, content.as_bytes())?;

    writeln!(
        out,
        "{}",
        serde_json::json!({
            "ok": true,
            "path": path.display().to_string(),
            "mode": "0600"
        })
    )?;
    Ok(())
}

fn prompt(msg: &str) -> Result<String> {
    use std::io::{BufRead, Write as _};
    eprint!("{msg}");
    std::io::stderr().flush().ok();
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(crate::error::Error::Io)?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

fn escape_toml(s: &str) -> String {
    // Basic TOML string escape for our limited character set — backslash and double-quote
    s.replace('\\', r"\\").replace('"', r#"\""#)
}

#[cfg(unix)]
fn write_secure(path: &std::path::Path, contents: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(contents)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_secure(path: &std::path::Path, contents: &[u8]) -> Result<()> {
    std::fs::write(path, contents)?;
    Ok(())
}

pub fn session_new<W: Write>(out: &mut W, cfg: &JiraConfig) -> Result<()> {
    use crate::api::session;
    let (user, pass) = read_credentials(cfg)?;
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
        field_aliases: Default::default(),
        defaults: Default::default(),
        field_renames: Default::default(),
        jql_aliases: Default::default(),
        default_project: None,
        effective_renames_cache: Default::default(),
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

/// Resolve basic-auth credentials for `session new`.
///
/// Precedence: `JIRA_USER`/`JIRA_PASSWORD` env > already-loaded config > interactive stdin.
/// The loaded `JiraConfig` may itself have been populated from the config file, so if the
/// user has `user` / `password` in `~/.config/jira-cli/config.toml`, that's what we use here.
fn read_credentials(cfg: &JiraConfig) -> Result<(String, String)> {
    use crate::config::AuthConfig;

    let env: std::collections::HashMap<String, String> = std::env::vars().collect();

    let (cfg_user, cfg_pass) = match &cfg.auth {
        AuthConfig::Basic { user, password } => (Some(user.clone()), Some(password.clone())),
        AuthConfig::Cookie { .. } => (None, None),
    };

    let user = match env.get("JIRA_USER").cloned().or(cfg_user) {
        Some(u) if !u.is_empty() => u,
        _ => read_nonempty_line("User: ", "JIRA_USER")?,
    };
    let pass = match env.get("JIRA_PASSWORD").cloned().or(cfg_pass) {
        Some(p) if !p.is_empty() => p,
        _ => read_nonempty_line("Password (will be echoed): ", "JIRA_PASSWORD")?,
    };

    Ok((user, pass))
}

/// Prompt on stderr, read one line from stdin, fail cleanly if empty / EOF.
fn read_nonempty_line(prompt_msg: &str, var_name: &str) -> Result<String> {
    use std::io::{BufRead, Write};
    eprint!("{prompt_msg}");
    std::io::stderr().flush().ok();
    let mut l = String::new();
    let n = std::io::stdin()
        .lock()
        .read_line(&mut l)
        .map_err(crate::error::Error::Io)?;
    let trimmed = l.trim_end_matches(['\r', '\n']).to_string();
    if n == 0 || trimmed.is_empty() {
        return Err(crate::error::Error::Usage(format!(
            "{var_name} not set in env or config, and no input on stdin"
        )));
    }
    Ok(trimmed)
}
