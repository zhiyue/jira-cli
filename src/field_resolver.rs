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
    aliases: HashMap<String, String>,
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
            aliases: HashMap::new(),
        }
    }

    /// Builder: attach an alias map (display name → field id). Takes precedence
    /// over auto-discovery lookups.
    pub fn with_aliases(mut self, aliases: HashMap<String, String>) -> Self {
        self.aliases = aliases;
        self
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
        // 1. Raw ids: `customfield_*` pass through unchanged.
        if key.starts_with("customfield_") {
            return Ok(key.to_string());
        }
        // 2. Alias table takes precedence over auto-discovery.
        if let Some(aliased) = self.aliases.get(key) {
            return Ok(aliased.clone());
        }
        // 3. Existing auto-discovery logic.
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

/// Convert a field display name to a safe snake_case ASCII identifier.
/// Non-ASCII chars become `_`. Empty result means caller should skip.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = true; // treat leading position as separator
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_sep = false;
        } else {
            if !prev_sep && !out.is_empty() {
                out.push('_');
            }
            prev_sep = true;
        }
    }
    // Trim trailing underscore
    while out.ends_with('_') {
        out.pop();
    }
    out
}

/// Build an auto-rename map: `customfield_XXXX → slug(name)`.
/// Skips fields whose slug is empty or collides with another field's slug.
/// Caller should merge with user overrides (user wins).
pub fn auto_rename_map(client: &HttpClient) -> Result<HashMap<String, String>> {
    let list = crate::api::field::list(client)?;
    let mut slug_count: HashMap<String, usize> = HashMap::new();

    for meta in &list {
        if !meta.custom {
            continue;
        }
        let slug = slugify(&meta.name);
        if slug.is_empty() {
            continue;
        }
        *slug_count.entry(slug).or_insert(0) += 1;
    }

    let mut out: HashMap<String, String> = HashMap::new();
    for meta in list {
        if !meta.custom {
            continue;
        }
        let slug = slugify(&meta.name);
        if slug.is_empty() {
            continue;
        }
        if slug_count.get(&slug).copied().unwrap_or(0) > 1 {
            continue; // collision → skip, let user resolve via [field_renames]
        }
        out.insert(meta.id, slug);
    }
    Ok(out)
}

#[cfg(test)]
mod slug_tests {
    use super::slugify;

    #[test]
    fn basic_ascii() {
        assert_eq!(slugify("Story Points"), "story_points");
        assert_eq!(slugify("Epic Link"), "epic_link");
    }

    #[test]
    fn parens_and_special() {
        assert_eq!(slugify("Fix Build Number(s)"), "fix_build_number_s");
        assert_eq!(slugify("Units (WBSGantt)"), "units_wbsgantt");
    }

    #[test]
    fn pure_non_ascii_becomes_empty() {
        assert_eq!(slugify("Bug严重等级"), "bug"); // "Bug" prefix survives
        assert_eq!(slugify("严重等级"), "");
    }

    #[test]
    fn mixed_non_ascii() {
        // "Bug 严重 等级" → "bug"
        assert_eq!(slugify("Bug 严重 等级"), "bug");
    }

    #[test]
    fn collapses_consecutive_separators() {
        assert_eq!(slugify("  Hello   World!!  "), "hello_world");
    }

    #[test]
    fn empty_and_edge_cases() {
        assert_eq!(slugify(""), "");
        assert_eq!(slugify("   "), "");
        assert_eq!(slugify("!!!"), "");
        assert_eq!(slugify("a"), "a");
    }
}
