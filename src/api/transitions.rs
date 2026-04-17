use crate::error::{Error, Result};
use crate::http::HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct TransitionList {
    #[serde(default)]
    pub transitions: Vec<Value>,
}

pub fn list(client: &HttpClient, key: &str) -> Result<TransitionList> {
    let path = format!("/rest/api/2/issue/{}/transitions", urlenc(key));
    client.get_json(&path)
}

pub fn resolve_name(client: &HttpClient, key: &str, name: &str) -> Result<String> {
    let list = list(client, key)?;
    let mut matches: Vec<String> = list
        .transitions
        .iter()
        .filter(|t| t.get("name").and_then(|v| v.as_str()) == Some(name))
        .filter_map(|t| t.get("id").and_then(|v| v.as_str()).map(String::from))
        .collect();
    match matches.len() {
        0 => Err(Error::Usage(format!(
            "transition '{name}' is not available for {key}"
        ))),
        1 => Ok(matches.remove(0)),
        _ => Err(Error::Usage(format!(
            "transition name '{name}' is ambiguous; use id"
        ))),
    }
}

pub fn execute(client: &HttpClient, key: &str, id: &str, fields: Option<Value>) -> Result<()> {
    #[derive(Serialize)]
    struct Body<'a> {
        transition: Transition<'a>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<Value>,
    }
    #[derive(Serialize)]
    struct Transition<'a> {
        id: &'a str,
    }
    let path = format!("/rest/api/2/issue/{}/transitions", urlenc(key));
    let body = Body {
        transition: Transition { id },
        fields,
    };
    client.post_empty(&path, &body)
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
