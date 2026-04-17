//! Exponential-backoff retry for idempotent requests. Real body lands in
//! Task 6; here we forward so the HttpClient compiles.

use crate::error::Result;
use reqwest::blocking::{RequestBuilder, Response};

pub fn send(
    _client: &super::HttpClient,
    req: RequestBuilder,
    _idempotent: bool,
) -> Result<Response> {
    let resp = req.send()?;
    Ok(resp)
}
