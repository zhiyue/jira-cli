//! POST /rest/api/2/search with streaming pagination.

use crate::error::Result;
use crate::http::HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct SearchParams {
    pub jql: String,
    pub fields: Vec<String>,
    pub expand: Vec<String>,
    pub max: Option<u64>,
    pub page_size: u64,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            jql: String::new(),
            fields: Vec::new(),
            expand: Vec::new(),
            max: None,
            page_size: 100,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Page {
    #[serde(rename = "startAt")]
    start_at: u64,
    #[allow(dead_code)]
    #[serde(rename = "maxResults")]
    max_results: u64,
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    issues: Vec<Value>,
}

#[derive(Debug, Serialize)]
struct Request<'a> {
    jql: &'a str,
    #[serde(rename = "startAt")]
    start_at: u64,
    #[serde(rename = "maxResults")]
    max_results: u64,
    #[serde(skip_serializing_if = "slice_is_empty")]
    fields: &'a [String],
    #[serde(skip_serializing_if = "slice_is_empty")]
    expand: &'a [String],
}

fn slice_is_empty(s: &&[String]) -> bool {
    s.is_empty()
}

pub struct SearchIter<'a> {
    client: &'a HttpClient,
    params: SearchParams,
    start_at: u64,
    total: Option<u64>,
    buf: VecDeque<Value>,
    emitted: u64,
    /// True once we know no further HTTP request should be made.
    no_more_pages: bool,
}

pub fn iter(client: &HttpClient, params: SearchParams) -> SearchIter<'_> {
    SearchIter {
        client,
        params,
        start_at: 0,
        total: None,
        buf: VecDeque::new(),
        emitted: 0,
        no_more_pages: false,
    }
}

impl<'a> SearchIter<'a> {
    pub fn total(&self) -> Option<u64> {
        self.total
    }

    pub fn emitted(&self) -> u64 {
        self.emitted
    }
}

impl<'a> Iterator for SearchIter<'a> {
    type Item = Result<Value>;
    fn next(&mut self) -> Option<Self::Item> {
        // Hard cap: caller asked for at most N items.
        if let Some(max) = self.params.max {
            if self.emitted >= max {
                return None;
            }
        }

        // Serve from buffer if available.
        if let Some(issue) = self.buf.pop_front() {
            self.emitted += 1;
            return Some(Ok(issue));
        }

        // Buffer empty. If we already know there are no more pages, stop.
        if self.no_more_pages {
            return None;
        }

        // Fetch next page.
        let req = Request {
            jql: &self.params.jql,
            start_at: self.start_at,
            max_results: self.params.page_size,
            fields: &self.params.fields,
            expand: &self.params.expand,
        };
        let page: Result<Page> = self.client.post_json("/rest/api/2/search", &req);
        match page {
            Err(e) => {
                self.no_more_pages = true;
                Some(Err(e))
            }
            Ok(p) => {
                self.total = p.total;
                let got = p.issues.len() as u64;
                self.buf.extend(p.issues);
                self.start_at = p.start_at + got;
                if got == 0 || self.start_at >= self.total.unwrap_or(u64::MAX) {
                    self.no_more_pages = true;
                }
                if self.buf.is_empty() {
                    return None;
                }
                // Tail-recurse to pop from the freshly-filled buffer.
                self.next()
            }
        }
    }
}
