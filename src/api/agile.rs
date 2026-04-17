//! Agile REST API 1.0: boards, sprints, epics, backlog.

use crate::error::Result;
use crate::http::HttpClient;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Page<T> {
    #[serde(default = "Vec::new")]
    pub values: Vec<T>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default, rename = "isLast")]
    pub is_last: Option<bool>,
}

// ---- Board ----

pub fn list_boards(
    client: &HttpClient,
    kind: Option<&str>,
    project_key: Option<&str>,
) -> Result<Page<Value>> {
    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(k) = kind {
        query.push(("type", k.into()));
    }
    if let Some(p) = project_key {
        query.push(("projectKeyOrId", p.into()));
    }
    client.get_json_query("/rest/agile/1.0/board", &query)
}

pub fn get_board(client: &HttpClient, id: u64) -> Result<Value> {
    client.get_json(&format!("/rest/agile/1.0/board/{id}"))
}

pub fn board_backlog(client: &HttpClient, id: u64) -> Result<Value> {
    client.get_json(&format!("/rest/agile/1.0/board/{id}/backlog"))
}
