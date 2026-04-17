//! Agile REST API 1.0: boards, sprints, epics, backlog.

use crate::error::Result;
use crate::http::HttpClient;
use serde::{Deserialize, Serialize};
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

// ---- Sprint ----

pub const SPRINT_MOVE_BATCH: usize = 50;

#[derive(Serialize)]
pub struct SprintCreate<'a> {
    pub name: &'a str,
    #[serde(rename = "originBoardId")]
    pub origin_board_id: u64,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    pub start_date: Option<&'a str>,
    #[serde(rename = "endDate", skip_serializing_if = "Option::is_none")]
    pub end_date: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<&'a str>,
}

pub fn list_sprints(client: &HttpClient, board_id: u64, states: &[&str]) -> Result<Page<Value>> {
    let mut query: Vec<(&str, String)> = Vec::new();
    if !states.is_empty() {
        query.push(("state", states.join(",")));
    }
    client.get_json_query(&format!("/rest/agile/1.0/board/{board_id}/sprint"), &query)
}

pub fn get_sprint(client: &HttpClient, id: u64) -> Result<Value> {
    client.get_json(&format!("/rest/agile/1.0/sprint/{id}"))
}

pub fn create_sprint(
    client: &HttpClient,
    board_id: u64,
    name: &str,
    start: Option<&str>,
    end: Option<&str>,
    goal: Option<&str>,
) -> Result<Value> {
    let body = SprintCreate {
        name,
        origin_board_id: board_id,
        start_date: start,
        end_date: end,
        goal,
    };
    client.post_json("/rest/agile/1.0/sprint", &body)
}

pub fn update_sprint(client: &HttpClient, id: u64, body: &Value) -> Result<Value> {
    // Partial update via POST on the sprint id (not PUT, which is a full replace).
    client.post_json(&format!("/rest/agile/1.0/sprint/{id}"), body)
}

pub fn delete_sprint(client: &HttpClient, id: u64) -> Result<()> {
    client.delete(&format!("/rest/agile/1.0/sprint/{id}"))
}

pub fn sprint_issues(client: &HttpClient, id: u64) -> Result<Value> {
    client.get_json(&format!("/rest/agile/1.0/sprint/{id}/issue"))
}

pub fn move_issues_to_sprint(client: &HttpClient, id: u64, keys: &[String]) -> Result<()> {
    for chunk in keys.chunks(SPRINT_MOVE_BATCH) {
        let body = serde_json::json!({ "issues": chunk });
        client.post_empty(&format!("/rest/agile/1.0/sprint/{id}/issue"), &body)?;
    }
    Ok(())
}
