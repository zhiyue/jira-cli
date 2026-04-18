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

pub fn list_paged<'a>(
    client: &'a HttpClient,
    key: &str,
    params: crate::api::paging::PageParams,
) -> crate::api::paging::PagedIter<'a> {
    let key = key.to_string();
    crate::api::paging::PagedIter::new(client, params, "worklogs", move |client, start, size| {
        let path = format!(
            "/rest/api/2/issue/{}/worklog?startAt={}&maxResults={}",
            urlenc(&key),
            start,
            size
        );
        client.get_json(&path)
    })
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
