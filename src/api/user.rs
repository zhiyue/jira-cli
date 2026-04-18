use crate::error::Result;
use crate::http::HttpClient;
use serde_json::Value;

pub fn get(client: &HttpClient, name: &str) -> Result<Value> {
    client.get_json_query("/rest/api/2/user", &[("username", name)])
}

pub fn search(client: &HttpClient, query: &str) -> Result<Vec<Value>> {
    client.get_json_query("/rest/api/2/user/search", &[("username", query)])
}

/// Like `search` but passes `maxResults` to Jira (server-side cap).
/// Note: Jira's `/user/search` returns a bare array with no pagination metadata,
/// so this only controls the server's page limit, not multi-page streaming.
pub fn search_with_max(client: &HttpClient, query: &str, max: u64) -> Result<Vec<Value>> {
    client.get_json_query(
        "/rest/api/2/user/search",
        &[("username", query), ("maxResults", &max.to_string())],
    )
}
