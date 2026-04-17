use crate::error::Result;
use crate::http::{check_status, HttpClient};
use reqwest::Method;
use serde_json::Value;

pub fn list(client: &HttpClient, key: &str) -> Result<Value> {
    client.get_json(&format!("/rest/api/2/issue/{}/watchers", urlenc(key)))
}

pub fn add(client: &HttpClient, key: &str, user: &str) -> Result<()> {
    let body = Value::String(user.to_string());
    let path = format!("/rest/api/2/issue/{}/watchers", urlenc(key));
    let req = client.request_builder(Method::POST, &path)?.json(&body);
    let resp = client.send(req, false)?;
    check_status(resp)?;
    Ok(())
}

pub fn remove(client: &HttpClient, key: &str, user: &str) -> Result<()> {
    let path = format!("/rest/api/2/issue/{}/watchers", urlenc(key));
    let req = client
        .request_builder(Method::DELETE, &path)?
        .query(&[("username", user)]);
    let resp = client.send(req, false)?;
    check_status(resp)?;
    Ok(())
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
