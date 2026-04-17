use crate::error::Result;
use crate::http::HttpClient;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct WorklogPage {
    #[serde(default)]
    pub worklogs: Vec<Value>,
    #[serde(default)]
    pub total: Option<u64>,
}

pub fn list(client: &HttpClient, key: &str) -> Result<WorklogPage> {
    let path = format!("/rest/api/2/issue/{}/worklog", urlenc(key));
    client.get_json(&path)
}

pub fn add(
    client: &HttpClient,
    key: &str,
    time_spent: &str,
    started: Option<&str>,
    comment: Option<&str>,
) -> Result<Value> {
    let mut body = serde_json::Map::new();
    body.insert("timeSpent".into(), json!(time_spent));
    if let Some(s) = started {
        body.insert("started".into(), json!(s));
    }
    if let Some(c) = comment {
        body.insert("comment".into(), json!(c));
    }
    let path = format!("/rest/api/2/issue/{}/worklog", urlenc(key));
    client.post_json(&path, &Value::Object(body))
}

pub fn delete(client: &HttpClient, key: &str, id: &str) -> Result<()> {
    let path = format!("/rest/api/2/issue/{}/worklog/{}", urlenc(key), urlenc(id));
    client.delete(&path)
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
