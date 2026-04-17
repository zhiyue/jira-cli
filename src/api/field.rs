//! `/rest/api/2/field` — metadata for every field (standard + custom).

use crate::error::Result;
use crate::http::HttpClient;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct FieldMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub schema: Option<Value>,
    #[serde(default, rename = "clauseNames")]
    pub clause_names: Vec<String>,
}

pub fn list(client: &HttpClient) -> Result<Vec<FieldMeta>> {
    client.get_json("/rest/api/2/field")
}
