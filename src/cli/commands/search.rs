use crate::api::search::{self, SearchParams};
use crate::cli::args::GlobalArgs;
use crate::cli::SearchArgs;
use crate::config::JiraConfig;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn run<W: Write>(
    out: &mut W,
    cfg: &JiraConfig,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &SearchArgs,
) -> Result<()> {
    let jira_fields =
        resolve_default_jira_fields(args.jira_fields.as_deref(), &cfg.defaults.search_fields);
    let params = SearchParams {
        jql: args.jql.clone(),
        fields: jira_fields,
        expand: split_csv(args.expand.as_deref()),
        max: args.max,
        page_size: args.page_size,
    };
    let fields = g.field_list();
    let renames = cfg.effective_renames(client)?;
    let opts = g.output_options_with_renames(Format::Jsonl, fields.as_deref(), Some(&renames));

    let mut iter = search::iter(client, params);
    let mut count = 0u64;
    for next in iter.by_ref() {
        let issue = next?;
        emit_value(out, issue, &opts)?;
        count += 1;
    }
    emit_line(
        out,
        &serde_json::json!({
            "summary": {"count": count, "total": iter.total()}
        }),
    )
}

/// Resolve the effective `jira-fields` list:
/// - `--jira-fields ""` (empty string literal) → return empty list (no fields= param to Jira)
/// - `--jira-fields "a,b"` (non-empty) → parse csv, return list
/// - no flag → use config defaults if any, else empty (fall through to Jira default behavior)
pub fn resolve_default_jira_fields(flag: Option<&str>, default: &[String]) -> Vec<String> {
    match flag {
        Some("") => Vec::new(),
        Some(s) => split_csv(Some(s)),
        None => default.to_vec(),
    }
}

pub fn split_csv(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect()
    })
    .unwrap_or_default()
}
