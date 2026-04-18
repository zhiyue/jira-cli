use crate::error::Result;
use crate::http::HttpClient;
use serde_json::Value;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct PageParams {
    pub start_at: u64,
    pub page_size: u64,
    pub max: Option<u64>,
}

impl Default for PageParams {
    fn default() -> Self {
        Self {
            start_at: 0,
            page_size: 50,
            max: None,
        }
    }
}

type FetchFn<'a> = Box<dyn Fn(&HttpClient, u64, u64) -> Result<Value> + 'a>;

/// Generic paged iterator for Jira v2 and agile endpoints. Caller provides:
/// - `fetch` closure: given (start_at, page_size) returns a Result<Value> (the raw page)
/// - `items_key`: key in the response that contains the item array (e.g. "comments", "worklogs", "values", "issues")
pub struct PagedIter<'a> {
    client: &'a HttpClient,
    fetch: FetchFn<'a>,
    items_key: &'static str,
    params: PageParams,
    start_at: u64,
    total: Option<u64>,
    is_last: bool,
    buf: VecDeque<Value>,
    emitted: u64,
    done: bool,
}

impl<'a> PagedIter<'a> {
    pub fn new<F>(
        client: &'a HttpClient,
        params: PageParams,
        items_key: &'static str,
        fetch: F,
    ) -> Self
    where
        F: Fn(&HttpClient, u64, u64) -> Result<Value> + 'a,
    {
        let start = params.start_at;
        Self {
            client,
            fetch: Box::new(fetch),
            items_key,
            params,
            start_at: start,
            total: None,
            is_last: false,
            buf: VecDeque::new(),
            emitted: 0,
            done: false,
        }
    }

    pub fn total(&self) -> Option<u64> {
        self.total
    }

    pub fn emitted(&self) -> u64 {
        self.emitted
    }
}

impl<'a> Iterator for PagedIter<'a> {
    type Item = Result<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if let Some(max) = self.params.max {
            if self.emitted >= max {
                self.done = true;
                return None;
            }
        }
        if let Some(v) = self.buf.pop_front() {
            self.emitted += 1;
            return Some(Ok(v));
        }
        // Need another page
        if self.is_last {
            self.done = true;
            return None;
        }
        let page_res = (self.fetch)(self.client, self.start_at, self.params.page_size);
        match page_res {
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
            Ok(raw) => {
                // Extract total (may be missing)
                if let Some(t) = raw.get("total").and_then(|v| v.as_u64()) {
                    self.total = Some(t);
                }
                // Extract isLast (agile only)
                let is_last_flag = raw.get("isLast").and_then(|v| v.as_bool());

                let empty_vec = Vec::new();
                let items = raw
                    .get(self.items_key)
                    .and_then(|v| v.as_array())
                    .unwrap_or(&empty_vec);
                let got = items.len() as u64;
                self.buf.extend(items.iter().cloned());
                self.start_at += got;

                if got == 0 {
                    self.is_last = true;
                } else if let Some(total) = self.total {
                    if self.start_at >= total {
                        self.is_last = true;
                    }
                } else if is_last_flag == Some(true) {
                    self.is_last = true;
                }
                self.next()
            }
        }
    }
}
