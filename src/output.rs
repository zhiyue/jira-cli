//! Stdout emission: JSON (single object or pretty) and JSONL (streaming one
//! record per line). Supports `--fields` dot-path projection.

use crate::error::Result;
use serde::Serialize;
use serde_json::Value;
use std::io::Write;

/// Format selector (mirrors CLI flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Jsonl,
}

pub struct OutputOptions<'a> {
    pub format: Format,
    pub pretty: bool,
    pub fields: Option<&'a [String]>,
    /// If set, keys in the JSON response are renamed before field projection.
    pub renames: Option<&'a std::collections::HashMap<String, String>>,
}

pub fn parse_field_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Project `value` to contain only the dot-paths listed in `fields`.
/// Missing paths are simply omitted from the output (no error).
pub fn project_fields(value: &Value, fields: &[impl AsRef<str>]) -> Value {
    let mut out = Value::Object(Default::default());
    for path in fields {
        copy_path(value, path.as_ref(), &mut out);
    }
    out
}

fn copy_path(src: &Value, path: &str, dest: &mut Value) {
    let Some(found) = fetch_path(src, path) else {
        return;
    };
    set_path(dest, path, found.clone());
}

fn fetch_path<'a>(mut v: &'a Value, path: &str) -> Option<&'a Value> {
    for seg in path.split('.') {
        v = v.get(seg)?;
    }
    Some(v)
}

fn set_path(dest: &mut Value, path: &str, value: Value) {
    let segments: Vec<&str> = path.split('.').collect();
    let mut cursor = dest;
    for (i, seg) in segments.iter().enumerate() {
        if i == segments.len() - 1 {
            cursor
                .as_object_mut()
                .expect("dest must be object")
                .insert((*seg).into(), value);
            return;
        }
        let obj = cursor.as_object_mut().expect("dest must be object");
        cursor = obj
            .entry((*seg).to_string())
            .or_insert_with(|| Value::Object(Default::default()));
        if !cursor.is_object() {
            *cursor = Value::Object(Default::default());
        }
    }
}

pub fn emit_json<W: Write, T: Serialize>(w: &mut W, value: &T, pretty: bool) -> Result<()> {
    if pretty {
        serde_json::to_writer_pretty(&mut *w, value)?;
    } else {
        serde_json::to_writer(&mut *w, value)?;
    }
    w.write_all(b"\n")?;
    Ok(())
}

pub fn emit_line<W: Write, T: Serialize>(w: &mut W, value: &T) -> Result<()> {
    // JSONL is always compact (one record per line).
    serde_json::to_writer(&mut *w, value)?;
    w.write_all(b"\n")?;
    Ok(())
}

/// Recursively rewrite JSON object keys using the rename map. Applied in-place on `Value`
/// before emit. Nested objects and arrays are traversed. Non-matching keys are left alone.
pub fn rename_keys(value: &mut Value, renames: &std::collections::HashMap<String, String>) {
    if renames.is_empty() {
        return;
    }
    match value {
        Value::Object(map) => {
            // Collect keys to rename first to avoid borrow conflicts.
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                if let Some(new_key) = renames.get(&k) {
                    let v = map.remove(&k).unwrap();
                    map.insert(new_key.clone(), v);
                }
            }
            // Recurse into children AFTER renaming this level's keys.
            for v in map.values_mut() {
                rename_keys(v, renames);
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                rename_keys(item, renames);
            }
        }
        _ => {}
    }
}

/// High-level helper for single-object emissions. Applies renames then field projection.
pub fn emit_value<W: Write>(w: &mut W, mut value: Value, opts: &OutputOptions<'_>) -> Result<()> {
    if let Some(renames) = opts.renames {
        rename_keys(&mut value, renames);
    }
    let projected = match opts.fields {
        Some(fs) if !fs.is_empty() => project_fields(&value, fs),
        _ => value,
    };
    match opts.format {
        Format::Json => emit_json(w, &projected, opts.pretty),
        Format::Jsonl => emit_line(w, &projected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rename_keys_simple() {
        let mut v = json!({"customfield_10006": 5, "summary": "x"});
        let mut renames = std::collections::HashMap::new();
        renames.insert("customfield_10006".into(), "story_points".into());
        rename_keys(&mut v, &renames);
        assert_eq!(v["story_points"], 5);
        assert!(v.get("customfield_10006").is_none());
        assert_eq!(v["summary"], "x");
    }

    #[test]
    fn rename_keys_nested() {
        let mut v = json!({
            "key": "MGX-1",
            "fields": {
                "customfield_10006": 5,
                "customfield_11407": {"value": "一般"}
            }
        });
        let mut renames = std::collections::HashMap::new();
        renames.insert("customfield_10006".into(), "story_points".into());
        renames.insert("customfield_11407".into(), "bug_severity".into());
        rename_keys(&mut v, &renames);
        assert_eq!(v["fields"]["story_points"], 5);
        assert_eq!(v["fields"]["bug_severity"]["value"], "一般");
    }

    #[test]
    fn rename_keys_empty_map_noop() {
        let mut v = json!({"customfield_10006": 5});
        let renames = std::collections::HashMap::new();
        rename_keys(&mut v, &renames);
        assert_eq!(v["customfield_10006"], 5);
    }

    #[test]
    fn rename_keys_array_walk() {
        let mut v = json!([
            {"customfield_10006": 1},
            {"customfield_10006": 2}
        ]);
        let mut renames = std::collections::HashMap::new();
        renames.insert("customfield_10006".into(), "story_points".into());
        rename_keys(&mut v, &renames);
        assert_eq!(v[0]["story_points"], 1);
        assert_eq!(v[1]["story_points"], 2);
    }

    #[test]
    fn emit_value_applies_renames_before_projection() {
        let v = json!({"customfield_10006": 5, "summary": "hello"});
        let mut renames = std::collections::HashMap::new();
        renames.insert("customfield_10006".into(), "story_points".into());
        let mut buf = Vec::new();
        let fields = vec!["story_points".to_string()];
        let opts = OutputOptions {
            format: Format::Json,
            pretty: false,
            fields: Some(&fields),
            renames: Some(&renames),
        };
        emit_value(&mut buf, v, &opts).unwrap();
        let out: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(out["story_points"], 5);
        assert!(out.get("customfield_10006").is_none());
        assert!(out.get("summary").is_none()); // projected out
    }

    #[test]
    fn project_flat_field() {
        let v = json!({"key": "MGX-1", "id": "100", "fields": {"summary": "a"}});
        let out = project_fields(&v, &["key"]);
        assert_eq!(out, json!({"key": "MGX-1"}));
    }

    #[test]
    fn project_dot_path() {
        let v = json!({"key": "MGX-1", "fields": {"summary": "hello", "status": {"name": "Open"}}});
        let out = project_fields(&v, &["key", "fields.status.name"]);
        assert_eq!(
            out,
            json!({"key":"MGX-1","fields":{"status":{"name":"Open"}}})
        );
    }

    #[test]
    fn project_missing_key_is_absent() {
        let v = json!({"key": "MGX-1"});
        let out = project_fields(&v, &["fields.summary"]);
        assert_eq!(out, json!({}));
    }

    #[test]
    fn parse_fields_csv() {
        let got = parse_field_list("key, fields.summary,fields.status.name ,");
        assert_eq!(got, vec!["key", "fields.summary", "fields.status.name"]);
    }

    #[test]
    fn emit_json_pretty_vs_compact() {
        let v = json!({"a": 1});
        let mut buf = Vec::new();
        emit_json(&mut buf, &v, false).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "{\"a\":1}\n");

        let mut buf = Vec::new();
        emit_json(&mut buf, &v, true).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\n  \"a\": 1"));
    }
}
