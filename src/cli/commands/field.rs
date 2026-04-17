//! `field list` / `field resolve`.

use crate::api::field;
use crate::cli::args::GlobalArgs;
use crate::cli::FieldCmd;
use crate::error::Result;
use crate::field_resolver::FieldResolver;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &FieldCmd,
) -> Result<()> {
    match cmd {
        FieldCmd::List => list(out, client, g),
        FieldCmd::Resolve(a) => resolve(out, client, &a.name),
    }
}

fn list<W: Write>(out: &mut W, client: &HttpClient, g: &GlobalArgs) -> Result<()> {
    let items = field::list(client)?;
    let fields = g.field_list();
    // JSONL by default for list-like output.
    let opts = g.output_options(Format::Jsonl, fields.as_deref());
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

fn resolve<W: Write>(out: &mut W, client: &HttpClient, name: &str) -> Result<()> {
    let r = FieldResolver::new(client);
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
        },
    )
}
