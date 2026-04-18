use crate::error::Result;
use crate::http::HttpClient;
use serde_json::Value;

pub fn list(client: &HttpClient) -> Result<Vec<Value>> {
    client.get_json("/rest/api/2/project")
}

pub fn get(client: &HttpClient, key: &str) -> Result<Value> {
    client.get_json(&format!("/rest/api/2/project/{}", urlenc(key)))
}

pub fn statuses(client: &HttpClient, key: &str) -> Result<Value> {
    client.get_json(&format!("/rest/api/2/project/{}/statuses", urlenc(key)))
}

pub fn components(client: &HttpClient, key: &str) -> Result<Vec<Value>> {
    client.get_json(&format!("/rest/api/2/project/{}/components", urlenc(key)))
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
