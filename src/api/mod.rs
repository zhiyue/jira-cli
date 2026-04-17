//! Typed, pure wrappers around Jira REST endpoints. No I/O besides the
//! HttpClient; no CLI concerns.

pub mod comment;
pub mod field;
pub mod issue;
pub mod link;
pub mod meta;
pub mod search;
pub mod transitions;
