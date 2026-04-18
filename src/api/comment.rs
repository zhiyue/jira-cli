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

pub fn list_paged<'a>(
    client: &'a HttpClient,
    key: &str,
    params: crate::api::paging::PageParams,
) -> crate::api::paging::PagedIter<'a> {
    let key = key.to_string();
    crate::api::paging::PagedIter::new(client, params, "comments", move |client, start, size| {
        let path = format!(
            "/rest/api/2/issue/{}/comment?startAt={}&maxResults={}",
            urlenc(&key),
            start,
            size
        );
        client.get_json(&path)
    })
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
