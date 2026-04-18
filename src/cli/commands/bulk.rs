//! Client-side parallel bulk operations.

use crate::api::{comment, transitions};
use crate::cli::args::GlobalArgs;
use crate::cli::BulkCmd;
use crate::config::JiraConfig;
use crate::error::Result;
use crate::http::HttpClient;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::{mpsc, Arc};
use std::thread;

#[derive(Debug, Deserialize)]
struct TransitionInput {
    key: String,
    to: String,
    #[serde(default)]
    fields: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CommentInput {
    key: String,
    body: String,
}

pub fn dispatch<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &BulkCmd,
) -> Result<()> {
    let _ = g; // reserved for future flags
    match cmd {
        BulkCmd::Transition { file, concurrency } => {
            bulk_transition(out, cfg, client, file, *concurrency)
        }
        BulkCmd::Comment { file, concurrency } => {
            bulk_comment(out, cfg, client, file, *concurrency)
        }
    }
}

fn read_lines(path: &str) -> Result<Vec<Vec<u8>>> {
    let reader: Box<dyn Read + Send> = if path == "-" {
        Box::new(std::io::stdin())
    } else {
        Box::new(std::fs::File::open(path)?)
    };
    let mut out = Vec::new();
    for line in BufReader::new(reader).lines() {
        let line = line?;
        if !line.trim().is_empty() {
            out.push(line.into_bytes());
        }
    }
    Ok(out)
}

fn bulk_transition<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    file: &str,
    concurrency: Option<usize>,
) -> Result<()> {
    let lines = read_lines(file)?;
    let (ok, failed) = run_parallel(
        client,
        cfg,
        concurrency,
        lines,
        |client, line| -> Result<serde_json::Value> {
            let input: TransitionInput = serde_json::from_slice(&line)?;
            let id = if input.to.chars().all(|c| c.is_ascii_digit()) {
                input.to.clone()
            } else {
                transitions::resolve_name(client, &input.key, &input.to)?
            };
            transitions::execute(client, &input.key, &id, input.fields.clone())?;
            Ok(serde_json::json!({"transition_id": id}))
        },
        |line| extract_key(line).unwrap_or_default(),
        out,
    )?;
    crate::output::emit_line(
        out,
        &serde_json::json!({"summary":{"ok": ok, "failed": failed}}),
    )
}

fn bulk_comment<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    file: &str,
    concurrency: Option<usize>,
) -> Result<()> {
    let lines = read_lines(file)?;
    let (ok, failed) = run_parallel(
        client,
        cfg,
        concurrency,
        lines,
        |client, line| -> Result<serde_json::Value> {
            let input: CommentInput = serde_json::from_slice(&line)?;
            comment::add(client, &input.key, &input.body)
        },
        |line| extract_key(line).unwrap_or_default(),
        out,
    )?;
    crate::output::emit_line(
        out,
        &serde_json::json!({"summary":{"ok": ok, "failed": failed}}),
    )
}

fn extract_key(bytes: &[u8]) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|v| v["key"].as_str().map(String::from))
}

/// Generic worker pool driving a per-line operation. Emits JSONL `{ok,key,data|error}`
/// lines as results arrive and returns final counts.
fn run_parallel<W, F, K>(
    client: &HttpClient,
    cfg: &JiraConfig,
    concurrency: Option<usize>,
    lines: Vec<Vec<u8>>,
    op: F,
    key_of: K,
    out: &mut W,
) -> Result<(usize, usize)>
where
    W: Write,
    F: Fn(&HttpClient, Vec<u8>) -> Result<serde_json::Value> + Send + Sync + 'static,
    K: Fn(&Vec<u8>) -> String + Send + Sync + 'static,
{
    let workers = concurrency.unwrap_or(cfg.concurrency).clamp(1, 16);
    let (tx, rx) = mpsc::channel::<(
        Vec<u8>,
        std::result::Result<serde_json::Value, crate::error::Error>,
    )>();
    let op = Arc::new(op);
    let key_of = Arc::new(key_of);

    let client_for_threads = client.clone();
    thread::scope(|s| {
        let chunks = chunkify(lines, workers);
        for chunk in chunks {
            let tx = tx.clone();
            let op = Arc::clone(&op);
            let key_of = Arc::clone(&key_of);
            let client = client_for_threads.clone();
            s.spawn(move || {
                for line in chunk {
                    let key = key_of(&line);
                    let result = op(&client, line.clone());
                    let _ = tx.send((
                        line,
                        result
                            .map(|data| serde_json::json!({"ok": true, "key": key, "data": data})),
                    ));
                }
            });
        }
        drop(tx);
    });

    let renames = &cfg.field_renames;
    let mut ok = 0usize;
    let mut failed = 0usize;
    for (_line, result) in rx {
        match result {
            Ok(mut v) => {
                crate::output::rename_keys(&mut v, renames);
                crate::output::emit_line(out, &v)?;
                ok += 1;
            }
            Err(e) => {
                let v = serde_json::json!({
                    "ok": false,
                    "error": e.to_stderr_json()["error"],
                });
                crate::output::emit_line(out, &v)?;
                failed += 1;
            }
        }
    }
    Ok((ok, failed))
}

fn chunkify<T: Send>(items: Vec<T>, workers: usize) -> Vec<Vec<T>> {
    let n = items.len();
    let _size = (n + workers - 1) / workers.max(1);
    let mut out: Vec<Vec<T>> = (0..workers).map(|_| Vec::new()).collect();
    for (i, item) in items.into_iter().enumerate() {
        out[i % workers].push(item);
    }
    // drop empties
    out.into_iter().filter(|c| !c.is_empty()).collect()
}
