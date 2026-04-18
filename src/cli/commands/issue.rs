//! Issue CLI commands.

use crate::api::issue;
use crate::cli::args::GlobalArgs;
use crate::cli::{IssueCmd, TransitionsCmd};
use crate::config::JiraConfig;
use crate::error::Result;
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
    cmd: &IssueCmd,
) -> Result<()> {
    match cmd {
        IssueCmd::Get(a) => get(out, cfg, client, g, a),
        IssueCmd::Create(a) => create(out, cfg, client, g, a),
        IssueCmd::Update(a) => update(out, cfg, client, g, a),
        IssueCmd::Delete(a) => delete(out, client, a),
        IssueCmd::Assign(a) => assign(out, client, a),
        IssueCmd::BulkCreate(a) => bulk_create(out, client, g, a),
        IssueCmd::Comment(sub) => crate::cli::commands::comment::dispatch(out, cfg, client, g, sub),
        IssueCmd::Transitions(TransitionsCmd::List { key }) => {
            let list = crate::api::transitions::list(client, key)?;
            let fields = g.field_list();
            let renames = cfg.effective_renames(client)?;
            let opts =
                g.output_options_with_renames(Format::Jsonl, fields.as_deref(), Some(&renames));
            for t in &list.transitions {
                crate::output::emit_value(out, t.clone(), &opts)?;
            }
            crate::output::emit_line(
                out,
                &serde_json::json!({"summary":{"count": list.transitions.len()}}),
            )?;
            Ok(())
        }
        IssueCmd::Transition(a) => transition(out, cfg, client, g, a),
        IssueCmd::Link(sub) => crate::cli::commands::link::dispatch(out, cfg, client, g, sub),
        IssueCmd::Attachment(sub) => {
            crate::cli::commands::attachment::dispatch(out, cfg, client, g, sub)
        }
        IssueCmd::Worklog(sub) => crate::cli::commands::worklog::dispatch(out, cfg, client, g, sub),
        IssueCmd::Watchers(sub) => {
            crate::cli::commands::watchers::dispatch(out, cfg, client, g, sub)
        }
        IssueCmd::Changelog(a) => changelog(out, cfg, client, g, a),
    }
}

fn get<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &crate::cli::IssueGet,
) -> Result<()> {
    use crate::cli::commands::search::resolve_default_jira_fields;
    let jira_fields =
        resolve_default_jira_fields(args.jira_fields.as_deref(), &cfg.defaults.issue_get_fields);
    let opts = issue::GetOpts {
        fields: jira_fields,
        expand: split_csv(args.expand.as_deref()),
    };
    let v = issue::get(client, &args.key, &opts)?;
    let fields = g.field_list();
    let renames = cfg.effective_renames(client)?;
    emit_value(
        out,
        v,
        &g.output_options_with_renames(Format::Json, fields.as_deref(), Some(&renames)),
    )
}

fn create<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &crate::cli::IssueCreate,
) -> Result<()> {
    use crate::cli::args::SetArg;
    use crate::field_resolver::FieldResolver;
    let sets = SetArg::parse_many(&args.set)?;
    let aliases = merged_field_aliases(cfg, g)?;
    let resolver = FieldResolver::new(client).with_aliases(aliases);
    let mut fields = serde_json::Map::new();
    fields.insert("project".into(), serde_json::json!({"key": args.project}));
    fields.insert(
        "issuetype".into(),
        serde_json::json!({"name": args.issue_type}),
    );
    fields.insert("summary".into(), serde_json::json!(args.summary));
    // --component flags: insert before --set so that --set "components=..." overrides
    if !args.components.is_empty() {
        let arr: Vec<serde_json::Value> = args
            .components
            .iter()
            .map(|n| serde_json::json!({"name": n}))
            .collect();
        fields.insert("components".into(), serde_json::Value::Array(arr));
    }
    for set in &sets {
        let id = resolver.resolve(&set.key)?;
        let value = resolve_raw_value(&set.raw)?;
        fields.insert(id, value);
    }
    let body = serde_json::json!({ "fields": fields });
    let v = issue::create(client, &body)?;
    let out_fields = g.field_list();
    let renames = cfg.effective_renames(client)?;
    emit_value(
        out,
        serde_json::json!({"ok": true, "data": v}),
        &g.output_options_with_renames(Format::Json, out_fields.as_deref(), Some(&renames)),
    )
}

fn update<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &crate::cli::IssueUpdate,
) -> Result<()> {
    use crate::cli::args::SetArg;
    use crate::field_resolver::FieldResolver;
    let sets = SetArg::parse_many(&args.set)?;
    let aliases = merged_field_aliases(cfg, g)?;
    let resolver = FieldResolver::new(client).with_aliases(aliases);
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
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
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
        let aliases = merged_field_aliases(cfg, g)?;
        let resolver = FieldResolver::new(client).with_aliases(aliases);
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

fn changelog<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &crate::cli::IssueChangelog,
) -> Result<()> {
    let opts = issue::GetOpts {
        fields: vec![], // don't need fields, only changelog
        expand: vec!["changelog".into()],
    };
    let v = issue::get(client, &args.key, &opts)?;
    let entries = v["changelog"]["histories"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let renames = cfg.effective_renames(client)?;
    let fields = g.field_list();
    let emit_opts = g.output_options_with_renames(Format::Jsonl, fields.as_deref(), Some(&renames));
    let cap = args.max.unwrap_or(u64::MAX);
    let mut count = 0u64;
    for entry in entries {
        if count >= cap {
            break;
        }
        emit_value(out, entry, &emit_opts)?;
        count += 1;
    }
    emit_line(
        out,
        &serde_json::json!({"summary": {"count": count, "total": v["changelog"]["total"]}}),
    )
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
    let fields = g.field_list();
    let mut opts = g.output_options(Format::Jsonl, fields.as_deref());
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
