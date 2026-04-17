use crate::error::Result;
use crate::http::HttpClient;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct CommentPage {
    #[serde(default)]
    pub comments: Vec<Value>,
    #[serde(default)]
    pub total: Option<u64>,
}

pub fn list(client: &HttpClient, key: &str) -> Result<CommentPage> {
    let path = format!("/rest/api/2/issue/{}/comment", urlenc(key));
    client.get_json(&path)
}

pub fn add(client: &HttpClient, key: &str, body: &str) -> Result<Value> {
    let path = format!("/rest/api/2/issue/{}/comment", urlenc(key));
    client.post_json(&path, &json!({ "body": body }))
}

pub fn update(client: &HttpClient, key: &str, id: &str, body: &str) -> Result<Value> {
    let path = format!("/rest/api/2/issue/{}/comment/{}", urlenc(key), urlenc(id));
    client.put_json(&path, &json!({ "body": body }))
}

pub fn delete(client: &HttpClient, key: &str, id: &str) -> Result<()> {
    let path = format!("/rest/api/2/issue/{}/comment/{}", urlenc(key), urlenc(id));
    client.delete(&path)
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
