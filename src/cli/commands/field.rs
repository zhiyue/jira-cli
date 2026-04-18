//! `field list` / `field resolve`.

use crate::api::field;
use crate::cli::args::GlobalArgs;
use crate::cli::FieldCmd;
use crate::config::JiraConfig;
use crate::error::Result;
use crate::field_resolver::FieldResolver;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::collections::HashMap;
use std::io::Write;

/// Merge config-file aliases with CLI flag aliases. CLI wins per key.
// requires cfg because it constructs FieldResolver
fn merged_field_aliases(cfg: &JiraConfig, g: &GlobalArgs) -> Result<HashMap<String, String>> {
    let mut map = cfg.field_aliases.clone();
    for (k, v) in g.parse_field_aliases()? {
        map.insert(k, v);
    }
    Ok(map)
}

pub fn dispatch<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &FieldCmd,
) -> Result<()> {
    match cmd {
        FieldCmd::List => list(out, cfg, client, g),
        FieldCmd::Resolve(a) => resolve(out, cfg, client, g, &a.name),
    }
}

fn list<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
) -> Result<()> {
    let items = field::list(client)?;
    let fields = g.field_list();
    // JSONL by default for list-like output.
    let opts =
        g.output_options_with_renames(Format::Jsonl, fields.as_deref(), Some(&cfg.field_renames));
    for item in &items {
        let v = serde_json::json!({
            "id": item.id,
            "name": item.name,
            "custom": item.custom,
            "schema": item.schema,
            "clauseNames": item.clause_names,
        });
        crate::output::emit_value(out, v, &opts)?;
    }
    emit_line(out, &serde_json::json!({"summary": {"count": items.len()}}))
}

fn resolve<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    name: &str,
) -> Result<()> {
    let aliases = merged_field_aliases(cfg, g)?;
    let r = FieldResolver::new(client).with_aliases(aliases);
    let id = r.resolve(name)?;
    let meta = r.metadata(&id)?;
    emit_value(
        out,
        serde_json::json!({
            "id": id,
            "name": name,
            "meta": meta.map(|m| serde_json::json!({
                "custom": m.custom,
                "schema": m.schema,
                "clauseNames": m.clause_names,
            })),
        }),
        &crate::output::OutputOptions {
            format: Format::Json,
            pretty: false,
            fields: None,
            renames: None,
        },
    )
}
