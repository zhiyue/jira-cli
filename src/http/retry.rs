//! Retry with exponential backoff for idempotent requests and for network
//! errors on writes. Spec §6.4.
//!
//! `Retry-After` header parsing: only integer-second values are supported
//! (e.g. `Retry-After: 1`). HTTP-date format is not parsed; if an HTTP-date
//! is received the header is ignored and the standard backoff delay is used
//! instead. Jira 8.x and Jira Cloud consistently return integer-second values.

use crate::error::{Error, Result};
use reqwest::blocking::{RequestBuilder, Response};
use reqwest::StatusCode;
use std::thread::sleep;
use std::time::Duration;

const BACKOFFS_MS: [u64; 3] = [100, 400, 1_600];
// Cap to honour Retry-After but never wait longer than this.
const MAX_RETRY_AFTER_MS: u64 = 10_000;

pub fn send(
    client: &super::HttpClient,
    req: RequestBuilder,
    is_idempotent: bool,
) -> Result<Response> {
    let mut attempt = 0usize;
    let retry_writes = client.retry_writes_enabled();
    loop {
        let try_req = req
            .try_clone()
            .ok_or_else(|| Error::Config("request body is not cloneable; cannot retry".into()))?;

        let send_result = try_req.send();

        match send_result {
            Ok(resp) => {
                let status = resp.status();
                if should_retry_response(status, is_idempotent, retry_writes)
                    && attempt < BACKOFFS_MS.len()
                {
                    let wait = retry_after_from(&resp)
                        .unwrap_or_else(|| Duration::from_millis(BACKOFFS_MS[attempt]));
                    log_retry(attempt + 1, wait, &format!("status {status}"));
                    sleep(wait);
                    attempt += 1;
                    continue;
                }
                return Ok(resp);
            }
            Err(e) => {
                let transient = is_transient_network_error(&e);
                let retry_ok = is_idempotent || retry_writes;
                if transient && retry_ok && attempt < BACKOFFS_MS.len() {
                    let wait = Duration::from_millis(BACKOFFS_MS[attempt]);
                    log_retry(attempt + 1, wait, &format!("network error: {e}"));
                    sleep(wait);
                    attempt += 1;
                    continue;
                }
                return Err(Error::Network(e));
            }
        }
    }
}

fn should_retry_response(status: StatusCode, is_idempotent: bool, retry_writes: bool) -> bool {
    let retryable = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
    retryable && (is_idempotent || retry_writes)
}

fn retry_after_from(resp: &Response) -> Option<Duration> {
    let header = resp.headers().get(reqwest::header::RETRY_AFTER)?;
    let secs: u64 = header.to_str().ok()?.trim().parse().ok()?;
    let ms = (secs.saturating_mul(1000)).min(MAX_RETRY_AFTER_MS);
    Some(Duration::from_millis(ms))
}

fn is_transient_network_error(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect() || e.is_request()
}

fn log_retry(attempt: usize, wait: Duration, reason: &str) {
    tracing::info!(
        target: "jira_cli::retry",
        attempt,
        wait_ms = wait.as_millis() as u64,
        reason,
        "retrying request"
    );
}
