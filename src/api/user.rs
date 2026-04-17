use crate::error::Result;
use crate::http::HttpClient;
use serde_json::Value;

pub fn get(client: &HttpClient, name: &str) -> Result<Value> {
    client.get_json_query("/rest/api/2/user", &[("username", name)])
}

pub fn search(client: &HttpClient, query: &str) -> Result<Vec<Value>> {
    client.get_json_query("/rest/api/2/user/search", &[("username", query)])
}
