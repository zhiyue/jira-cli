//! `/rest/api/2/serverInfo` and `/rest/api/2/myself` — untyped Value return
//! (small bodies, agent consumes raw JSON).

use crate::error::Result;
use crate::http::HttpClient;
use serde_json::Value;

pub fn server_info(client: &HttpClient) -> Result<Value> {
    client.get_json("/rest/api/2/serverInfo")
}

pub fn myself(client: &HttpClient) -> Result<Value> {
    client.get_json("/rest/api/2/myself")
}
