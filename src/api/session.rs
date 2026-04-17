use crate::error::Result;
use crate::http::HttpClient;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
struct NewSessionResponse {
    session: SessionInfo,
}

impl SessionInfo {
    pub fn cookie_header(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

pub fn new(client: &HttpClient, username: &str, password: &str) -> Result<SessionInfo> {
    let body = serde_json::json!({"username": username, "password": password});
    let r: NewSessionResponse = client.post_json("/rest/auth/1/session", &body)?;
    Ok(r.session)
}
