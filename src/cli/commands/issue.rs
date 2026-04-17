//! Issue CLI commands.

use crate::api::issue;
use crate::cli::args::GlobalArgs;
use crate::cli::IssueCmd;
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
    use crate::error::Error;
    let sets = SetArg::parse_many(&args.set)?;
    let mut fields = serde_json::Map::new();
    fields.insert("project".into(), serde_json::json!({"key": args.project}));
    fields.insert(
        "issuetype".into(),
        serde_json::json!({"name": args.issue_type}),
    );
    fields.insert("summary".into(), serde_json::json!(args.summary));
    for set in &sets {
        if !set.key.starts_with("customfield_") && set.key.chars().any(|c| c == ' ') {
            return Err(Error::Usage(format!(
                "display-name translation for --set key '{}' lands in a later task; \
                 for now use customfield_XXXXX or a Jira schema field id",
                set.key
            )));
        }
        let value = resolve_raw_value(&set.raw)?;
        fields.insert(set.key.clone(), value);
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
    let sets = SetArg::parse_many(&args.set)?;
    let mut fields = serde_json::Map::new();
    for set in &sets {
        let value = resolve_raw_value(&set.raw)?;
        fields.insert(set.key.clone(), value);
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

fn split_csv(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}
