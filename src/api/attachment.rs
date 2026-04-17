use crate::error::{Error, Result};
use crate::http::{check_status, HttpClient};
use reqwest::blocking::multipart::Form;
use reqwest::Method;
use serde_json::Value;

pub fn upload(client: &HttpClient, key: &str, paths: &[std::path::PathBuf]) -> Result<Value> {
    let mut form = Form::new();
    for p in paths {
        form = form
            .file("file", p)
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;
    }
    let path = format!("/rest/api/2/issue/{}/attachments", urlenc(key));
    let req = client.request_builder(Method::POST, &path)?.multipart(form);
    let resp = client.send(req, false)?;
    let resp = check_status(resp)?;
    Ok(resp.json()?)
}

pub fn delete(client: &HttpClient, id: &str) -> Result<()> {
    let path = format!("/rest/api/2/attachment/{id}");
    client.delete(&path)
}

pub fn meta(client: &HttpClient, id: &str) -> Result<Value> {
    client.get_json(&format!("/rest/api/2/attachment/{id}"))
}

/// Download raw bytes from an attachment's `content` URL.
/// `content_url` may be absolute (common — Jira returns full `content` URL).
pub fn download(client: &HttpClient, content_url: &str) -> Result<Vec<u8>> {
    let req = client.request_builder(Method::GET, content_url)?;
    let resp = client.send(req, true)?;
    let resp = check_status(resp)?;
    Ok(resp.bytes()?.to_vec())
}

/// List attachments from an issue.
pub fn list_for_issue(client: &HttpClient, key: &str) -> Result<Vec<Value>> {
    use crate::api::issue;
    let opts = issue::GetOpts {
        fields: vec!["attachment".into()],
        expand: vec![],
    };
    let v = issue::get(client, key, &opts)?;
    Ok(v["fields"]["attachment"]
        .as_array()
        .cloned()
        .unwrap_or_default())
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
