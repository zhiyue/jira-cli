//! Issue CLI commands.

use crate::api::issue;
use crate::cli::args::GlobalArgs;
use crate::cli::{IssueCmd, TransitionsCmd};
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_value, Format};
use std::io::Write;

pub fn dispatch<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    cmd: &IssueCmd,
) -> Result<()> {
    match cmd {
        IssueCmd::Get(a) => get(out, client, g, a),
        IssueCmd::Create(a) => create(out, client, g, a),
        IssueCmd::Update(a) => update(out, client, a),
        IssueCmd::Delete(a) => delete(out, client, a),
        IssueCmd::Assign(a) => assign(out, client, a),
        IssueCmd::BulkCreate(a) => bulk_create(out, client, g, a),
        IssueCmd::Comment(sub) => crate::cli::commands::comment::dispatch(out, client, g, sub),
        IssueCmd::Transitions(TransitionsCmd::List { key }) => {
            let list = crate::api::transitions::list(client, key)?;
            let opts = g.output_options(Format::Jsonl, None);
            for t in &list.transitions {
                crate::output::emit_value(out, t.clone(), &opts)?;
            }
            crate::output::emit_line(
                out,
                &serde_json::json!({"summary":{"count": list.transitions.len()}}),
            )?;
            Ok(())
        }
        IssueCmd::Transition(a) => transition(out, client, a),
        IssueCmd::Link(sub) => crate::cli::commands::link::dispatch(out, client, g, sub),
        IssueCmd::Attachment(sub) => {
            crate::cli::commands::attachment::dispatch(out, client, g, sub)
        }
        IssueCmd::Worklog(sub) => crate::cli::commands::worklog::dispatch(out, client, g, sub),
    }
}

fn get<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &crate::cli::IssueGet,
) -> Result<()> {
    let opts = issue::GetOpts {
        fields: split_csv(args.jira_fields.as_deref()),
        expand: split_csv(args.expand.as_deref()),
    };
    let v = issue::get(client, &args.key, &opts)?;
    let fields = g.field_list();
    emit_value(out, v, &g.output_options(Format::Json, fields.as_deref()))
}

fn create<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &crate::cli::IssueCreate,
) -> Result<()> {
    use crate::cli::args::SetArg;
    use crate::field_resolver::FieldResolver;
    let sets = SetArg::parse_many(&args.set)?;
    let resolver = FieldResolver::new(client);
    let mut fields = serde_json::Map::new();
    fields.insert("project".into(), serde_json::json!({"key": args.project}));
    fields.insert(
        "issuetype".into(),
        serde_json::json!({"name": args.issue_type}),
    );
    fields.insert("summary".into(), serde_json::json!(args.summary));
    for set in &sets {
        let id = resolver.resolve(&set.key)?;
        let value = resolve_raw_value(&set.raw)?;
        fields.insert(id, value);
    }
    let body = serde_json::json!({ "fields": fields });
    let v = issue::create(client, &body)?;
    let fields = g.field_list();
    emit_value(
        out,
        serde_json::json!({"ok": true, "data": v}),
        &g.output_options(Format::Json, fields.as_deref()),
    )
}

fn update<W: Write>(
    out: &mut W,
    client: &HttpClient,
    args: &crate::cli::IssueUpdate,
) -> Result<()> {
    use crate::cli::args::SetArg;
    use crate::field_resolver::FieldResolver;
    let sets = SetArg::parse_many(&args.set)?;
    let resolver = FieldResolver::new(client);
    let mut fields = serde_json::Map::new();
    for set in &sets {
        let id = resolver.resolve(&set.key)?;
        let value = resolve_raw_value(&set.raw)?;
        fields.insert(id, value);
    }
    let body = serde_json::json!({ "fields": fields });
    issue::update(client, &args.key, &body)?;
    writeln!(out, "{}", serde_json::json!({"ok": true, "key": args.key}))?;
    Ok(())
}

fn delete<W: Write>(
    out: &mut W,
    client: &HttpClient,
    args: &crate::cli::IssueDelete,
) -> Result<()> {
    use crate::error::Error;
    if !args.yes {
        return Err(Error::Usage(
            "issue delete requires --yes to confirm".into(),
        ));
    }
    issue::delete(client, &args.key)?;
    writeln!(
        out,
        "{}",
        serde_json::json!({"ok": true, "deleted": args.key})
    )?;
    Ok(())
}

fn assign<W: Write>(
    out: &mut W,
    client: &HttpClient,
    args: &crate::cli::IssueAssign,
) -> Result<()> {
    let target = if args.unassign {
        None
    } else {
        args.user.as_deref()
    };
    issue::assign(client, &args.key, target)?;
    writeln!(
        out,
        "{}",
        serde_json::json!({"ok": true, "key": args.key, "assignee": target})
    )?;
    Ok(())
}

fn transition<W: Write>(
    out: &mut W,
    client: &HttpClient,
    args: &crate::cli::TransitionArgs,
) -> Result<()> {
    use crate::api::transitions;
    use crate::cli::args::SetArg;
    use crate::field_resolver::FieldResolver;
    let id = if args.to.chars().all(|c| c.is_ascii_digit()) {
        args.to.clone()
    } else {
        transitions::resolve_name(client, &args.key, &args.to)?
    };

    let fields = if args.set.is_empty() {
        None
    } else {
        let resolver = FieldResolver::new(client);
        let mut map = serde_json::Map::new();
        for raw in SetArg::parse_many(&args.set)? {
            let id = resolver.resolve(&raw.key)?;
            let value = resolve_raw_value(&raw.raw)?;
            map.insert(id, value);
        }
        Some(serde_json::Value::Object(map))
    };

    transitions::execute(client, &args.key, &id, fields)?;
    writeln!(
        out,
        "{}",
        serde_json::json!({"ok": true, "key": args.key, "transition_id": id})
    )?;
    Ok(())
}

fn resolve_raw_value(raw: &crate::cli::args::RawValue) -> Result<serde_json::Value> {
    use crate::cli::args::RawValue;
    use std::io::Read;
    match raw {
        RawValue::Scalar(s) => Ok(serde_json::Value::String(s.clone())),
        RawValue::Json(v) => Ok(v.clone()),
        RawValue::File(p) => {
            let bytes = std::fs::read(p)?;
            let s = String::from_utf8_lossy(&bytes).into_owned();
            Ok(serde_json::Value::String(s))
        }
        RawValue::Stdin => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(serde_json::Value::String(buf))
        }
    }
}

fn bulk_create<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &crate::cli::IssueBulkCreate,
) -> Result<()> {
    let bytes = read_input(&args.from_file)?;
    let inputs: Vec<serde_json::Value> = serde_json::from_slice(&bytes)?;
    let results = issue::bulk_create(client, &inputs)?;

    // JSONL: one line per created, one per error, plus summary.
    let mut opts = g.output_options(Format::Jsonl, None);
    opts.pretty = false;
    for created in &results.created {
        crate::output::emit_line(out, &serde_json::json!({"ok": true, "data": created}))?;
    }
    for err in &results.errors {
        crate::output::emit_line(out, &serde_json::json!({"ok": false, "error": err}))?;
    }
    crate::output::emit_line(
        out,
        &serde_json::json!({
            "summary": {
                "ok": results.created.len(),
                "failed": results.errors.len()
            }
        }),
    )?;
    Ok(())
}

fn read_input(path: &str) -> Result<Vec<u8>> {
    use std::io::Read;
    if path == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        Ok(std::fs::read(path)?)
    }
}

fn split_csv(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}
