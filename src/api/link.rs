use crate::api::issue;
use crate::error::Result;
use crate::http::HttpClient;
use serde_json::{json, Value};

pub fn create(
    client: &HttpClient,
    outward_key: &str,
    inward_key: &str,
    link_type: &str,
) -> Result<()> {
    let body = json!({
        "type": {"name": link_type},
        "inwardIssue": {"key": inward_key},
        "outwardIssue": {"key": outward_key}
    });
    client.post_empty("/rest/api/2/issueLink", &body)
}

pub fn delete(client: &HttpClient, link_id: &str) -> Result<()> {
    let path = format!("/rest/api/2/issueLink/{link_id}");
    client.delete(&path)
}

/// List links embedded in an issue's `fields.issuelinks`.
pub fn list_for_issue(client: &HttpClient, key: &str) -> Result<Vec<Value>> {
    let opts = issue::GetOpts {
        fields: vec!["issuelinks".into()],
        expand: vec![],
    };
    let v = issue::get(client, key, &opts)?;
    Ok(v["fields"]["issuelinks"]
        .as_array()
        .cloned()
        .unwrap_or_default())
}
