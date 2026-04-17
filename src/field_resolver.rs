//! Per-invocation name↔id resolver backed by `GET /rest/api/2/field`.
//! Stateless across process runs — each CLI invocation re-fetches.

use crate::api::field::{self, FieldMeta};
use crate::error::{Error, FieldError, Result};
use crate::http::HttpClient;
use std::cell::OnceCell;
use std::collections::HashMap;

pub struct FieldResolver<'a> {
    client: &'a HttpClient,
    cache: OnceCell<Index>,
}

struct Index {
    by_id: HashMap<String, FieldMeta>,
    by_name: HashMap<String, Vec<String>>, // name → ids (may be ambiguous)
}

impl<'a> FieldResolver<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Self {
            client,
            cache: OnceCell::new(),
        }
    }

    fn index(&self) -> Result<&Index> {
        if let Some(idx) = self.cache.get() {
            return Ok(idx);
        }
        let list = field::list(self.client)?;
        let mut by_id: HashMap<String, FieldMeta> = HashMap::new();
        let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
        for meta in list {
            by_name
                .entry(meta.name.clone())
                .or_default()
                .push(meta.id.clone());
            by_id.insert(meta.id.clone(), meta);
        }
        let _ = self.cache.set(Index { by_id, by_name });
        Ok(self.cache.get().expect("just set"))
    }

    /// Given a user-supplied key (display name or raw id), return the id.
    pub fn resolve(&self, key: &str) -> Result<String> {
        // Raw ids: `summary`, `customfield_10020`, etc. pass through.
        if key.starts_with("customfield_") {
            return Ok(key.to_string());
        }
        let idx = self.index()?;
        if idx.by_id.contains_key(key) {
            return Ok(key.to_string());
        }
        match idx.by_name.get(key) {
            None => Err(Error::FieldResolve(FieldError::Unknown(key.to_string()))),
            Some(ids) if ids.len() == 1 => Ok(ids[0].clone()),
            Some(ids) => Err(Error::FieldResolve(FieldError::Ambiguous {
                name: key.to_string(),
                candidates: ids.clone(),
            })),
        }
    }

    pub fn metadata(&self, id: &str) -> Result<Option<FieldMeta>> {
        let idx = self.index()?;
        Ok(idx.by_id.get(id).cloned())
    }
}
