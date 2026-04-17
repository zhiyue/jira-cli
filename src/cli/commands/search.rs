use crate::api::search::{self, SearchParams};
use crate::cli::args::GlobalArgs;
use crate::cli::SearchArgs;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_line, emit_value, Format};
use std::io::Write;

pub fn run<W: Write>(
    out: &mut W,
    client: &HttpClient,
    g: &GlobalArgs,
    args: &SearchArgs,
) -> Result<()> {
    let params = SearchParams {
        jql: args.jql.clone(),
        fields: split_csv(args.jira_fields.as_deref()),
        expand: split_csv(args.expand.as_deref()),
        max: args.max,
        page_size: args.page_size,
    };
    let fields = g.field_list();
    let opts = g.output_options(Format::Jsonl, fields.as_deref());

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

fn split_csv(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect()
    })
    .unwrap_or_default()
}
