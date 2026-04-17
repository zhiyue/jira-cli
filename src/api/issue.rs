//! `/rest/api/2/issue/*` — get + CRUD + bulk.

use crate::error::{ApiErrorBody, Error, Result};
use crate::http::HttpClient;
use serde_json::Value;

#[derive(Debug, Default, Clone)]
pub struct GetOpts {
    pub fields: Vec<String>,
    pub expand: Vec<String>,
}

pub fn get(client: &HttpClient, key: &str, opts: &GetOpts) -> Result<Value> {
    let path = format!("/rest/api/2/issue/{}", urlencoding(key));
    let mut query: Vec<(&str, String)> = Vec::new();
    if !opts.fields.is_empty() {
        query.push(("fields", opts.fields.join(",")));
    }
    if !opts.expand.is_empty() {
        query.push(("expand", opts.expand.join(",")));
    }
    map_issue_err(client.get_json_query(&path, &query), key)
}

/// Convert a generic 404 into `Error::NotFound { resource: "issue", key }`.
pub(crate) fn map_issue_err<T>(result: Result<T>, key: &str) -> Result<T> {
    match result {
        Err(Error::Api(ApiErrorBody { status: 404, .. })) => Err(Error::NotFound {
            resource: "issue",
            key: key.to_string(),
        }),
        other => other,
    }
}

pub fn create(client: &HttpClient, body: &Value) -> Result<Value> {
    client.post_json("/rest/api/2/issue", body)
}

pub fn update(client: &HttpClient, key: &str, body: &Value) -> Result<()> {
    let path = format!("/rest/api/2/issue/{}", urlencoding(key));
    let req = client
        .request_builder(reqwest::Method::PUT, &path)?
        .json(body);
    let resp = client.send(req, false)?;
    map_issue_err(crate::http::check_status(resp).map(|_| ()), key)
}

pub fn delete(client: &HttpClient, key: &str) -> Result<()> {
    let path = format!("/rest/api/2/issue/{}", urlencoding(key));
    map_issue_err(client.delete(&path), key)
}

pub fn assign(client: &HttpClient, key: &str, assignee: Option<&str>) -> Result<()> {
    let path = format!("/rest/api/2/issue/{}/assignee", urlencoding(key));
    let body = serde_json::json!({ "name": assignee });
    let req = client
        .request_builder(reqwest::Method::PUT, &path)?
        .json(&body);
    let resp = client.send(req, false)?;
    map_issue_err(crate::http::check_status(resp).map(|_| ()), key)
}

fn urlencoding(s: &str) -> String {
    // Jira issue keys are ASCII [A-Z]+-\d+, but url-encode defensively.
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
