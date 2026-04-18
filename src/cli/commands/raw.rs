//! `raw <METHOD> <PATH>` — escape hatch that hits arbitrary Jira endpoints
//! using the established auth / retry / error handling.

use crate::cli::args::GlobalArgs;
use crate::cli::RawArgs;
use crate::config::JiraConfig;
use crate::error::{Error, Result};
use crate::http::{check_status, HttpClient};
use reqwest::Method;
use std::io::Write;

pub fn run<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &RawArgs,
) -> Result<()> {
    let method_str = args.method.to_uppercase();
    let method = Method::from_bytes(method_str.as_bytes())
        .map_err(|e| Error::Usage(format!("invalid HTTP method '{}': {e}", args.method)))?;

    let mut req = client.request_builder(method.clone(), &args.path)?;

    // Query params
    if !args.query.is_empty() {
        let pairs: Result<Vec<(String, String)>> = args
            .query
            .iter()
            .map(|kv| {
                let (k, v) = kv
                    .split_once('=')
                    .ok_or_else(|| Error::Usage(format!("--query expects KEY=VALUE, got: {kv}")))?;
                Ok((k.trim().to_string(), v.to_string()))
            })
            .collect();
        req = req.query(&pairs?);
    }

    // Extra headers
    for kv in &args.header {
        let (k, v) = kv
            .split_once(':')
            .ok_or_else(|| Error::Usage(format!("--header expects KEY:VALUE, got: {kv}")))?;
        req = req.header(k.trim(), v.trim_start());
    }

    // Body
    if let Some(data) = &args.data {
        let bytes = read_body(data)?;
        // If it parses as JSON, send with application/json; otherwise raw bytes
        match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(v) => {
                req = req.json(&v);
            }
            Err(_) => {
                req = req
                    .header("content-type", "application/octet-stream")
                    .body(bytes);
            }
        }
    }

    let is_idempotent = matches!(method, Method::GET | Method::HEAD | Method::OPTIONS);
    let resp = client.send(req, is_idempotent)?;
    let resp = check_status(resp)?;

    let bytes = resp.bytes()?.to_vec();
    if args.raw_body {
        out.write_all(&bytes)?;
        return Ok(());
    }

    // Try to parse JSON for pretty-print + --fields projection; fall back to raw bytes
    match serde_json::from_slice::<serde_json::Value>(&bytes) {
        Ok(v) => {
            let renames = cfg.effective_renames(client)?;
            let fields = g.field_list();
            crate::output::emit_value(
                out,
                v,
                &g.output_options_with_renames(
                    crate::output::Format::Json,
                    fields.as_deref(),
                    Some(&renames),
                ),
            )
        }
        Err(_) => {
            out.write_all(&bytes)?;
            Ok(())
        }
    }
}

fn read_body(arg: &str) -> Result<Vec<u8>> {
    use std::io::Read;
    if arg == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        return Ok(buf);
    }
    if let Some(path) = arg.strip_prefix('@') {
        return Ok(std::fs::read(path)?);
    }
    Ok(arg.as_bytes().to_vec())
}
