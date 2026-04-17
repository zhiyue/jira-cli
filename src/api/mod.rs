//! Typed, pure wrappers around Jira REST endpoints. No I/O besides the
//! HttpClient; no CLI concerns.

pub mod agile;
pub mod attachment;
pub mod comment;
pub mod field;
pub mod issue;
pub mod link;
pub mod meta;
pub mod project;
pub mod search;
pub mod transitions;
pub mod user;
pub mod watchers;
pub mod worklog;
