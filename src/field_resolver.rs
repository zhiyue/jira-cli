//! Per-invocation name↔id resolver. Full body populated in Task 14 alongside
//! the `api::field` module.

use crate::http::HttpClient;
use std::cell::OnceCell;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub id: String,
    pub name: String,
    pub custom: bool,
    pub schema: Option<serde_json::Value>,
}

#[allow(dead_code)]
pub struct FieldResolver<'a> {
    client: &'a HttpClient,
    cache: OnceCell<HashMap<String, Vec<FieldInfo>>>,
}

impl<'a> FieldResolver<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Self {
            client,
            cache: OnceCell::new(),
        }
    }
    // Methods added in Task 14.
}

// Suppress unused-field warning until the resolver is fleshed out.
#[allow(dead_code)]
fn _touch(resolver: FieldResolver<'_>) {
    let _ = resolver.client;
}
