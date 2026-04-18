//! Shared CLI arg structs.

use crate::error::{Error, Result};
use crate::output::{parse_field_list, Format, OutputOptions};
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Logging verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long = "verbose", action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Output format (default: json; list/search/bulk auto-use jsonl).
    #[arg(long, value_enum, global = true)]
    pub output: Option<FormatArg>,

    /// Pretty-print JSON output (ignored for JSONL).
    #[arg(long, global = true)]
    pub pretty: bool,

    /// Comma-separated dot-path keys to project.
    #[arg(long, global = true)]
    pub fields: Option<String>,

    /// Override JIRA_TIMEOUT in seconds.
    #[arg(long, global = true)]
    pub timeout: Option<u64>,

    /// Skip TLS verification. NOT RECOMMENDED.
    #[arg(long, global = true)]
    pub insecure: bool,

    /// Field alias in the form NAME=ID, e.g. "Story Points=customfield_10006".
    /// Repeatable. Overrides any [field_aliases] entry in the config file.
    #[arg(long = "field-alias", value_name = "NAME=ID", global = true)]
    pub field_alias: Vec<String>,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatArg {
    Json,
    Jsonl,
}

impl From<FormatArg> for Format {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Json => Format::Json,
            FormatArg::Jsonl => Format::Jsonl,
        }
    }
}

impl GlobalArgs {
    pub fn field_list(&self) -> Option<Vec<String>> {
        self.fields.as_deref().map(parse_field_list)
    }

    /// Parse `--field-alias NAME=ID` args into a HashMap. Errors on malformed entries.
    pub fn parse_field_aliases(
        &self,
    ) -> crate::error::Result<std::collections::HashMap<String, String>> {
        let mut out = std::collections::HashMap::new();
        for raw in &self.field_alias {
            let (k, v) = raw.split_once('=').ok_or_else(|| {
                crate::error::Error::Usage(format!("--field-alias expects NAME=ID, got: {raw}"))
            })?;
            let k = k.trim();
            let v = v.trim();
            if k.is_empty() || v.is_empty() {
                return Err(crate::error::Error::Usage(
                    "--field-alias NAME and ID must both be non-empty".into(),
                ));
            }
            out.insert(k.to_string(), v.to_string());
        }
        Ok(out)
    }

    pub fn output_options<'a>(
        &self,
        default_format: Format,
        fields: Option<&'a [String]>,
    ) -> OutputOptions<'a> {
        OutputOptions {
            format: self.output.map(Into::into).unwrap_or(default_format),
            pretty: self.pretty,
            fields,
        }
    }
}

/// A single `--set Key=Value` argument. Parsed eagerly; value resolution
/// (file load, JSON coercion, name→id mapping) happens later against the
/// field schema.
#[derive(Debug, Clone, PartialEq)]
pub struct SetArg {
    pub key: String,
    pub raw: RawValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawValue {
    Scalar(String),
    Json(serde_json::Value),
    File(std::path::PathBuf),
    Stdin,
}

impl SetArg {
    pub fn parse(raw: &str) -> Result<Self> {
        let (key, val) = raw
            .split_once('=')
            .ok_or_else(|| Error::Usage(format!("--set expects KEY=VALUE, got: {raw}")))?;
        let key = key.trim().to_string();
        if key.is_empty() {
            return Err(Error::Usage("--set KEY must not be empty".into()));
        }
        let raw = classify(val);
        Ok(Self { key, raw })
    }

    pub fn parse_many(args: &[String]) -> Result<Vec<Self>> {
        args.iter().map(|s| Self::parse(s)).collect()
    }
}

fn classify(val: &str) -> RawValue {
    if val == "@-" {
        return RawValue::Stdin;
    }
    if let Some(path) = val.strip_prefix('@') {
        return RawValue::File(path.into());
    }
    // JSON literal heuristic: starts with `[`, `{`, `"`, or is a bare
    // number/bool/null. Attempt parse — fall back to scalar.
    let trimmed = val.trim();
    let first = trimmed.chars().next();
    let looks_json = matches!(first, Some('[' | '{' | '"'))
        || trimmed == "true"
        || trimmed == "false"
        || trimmed == "null"
        || trimmed.parse::<f64>().is_ok();
    if looks_json {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            return RawValue::Json(v);
        }
    }
    RawValue::Scalar(val.to_string())
}

#[cfg(test)]
mod tests_set {
    use super::*;

    #[test]
    fn parse_scalar() {
        let s = SetArg::parse("Summary=hello world").unwrap();
        assert_eq!(s.key, "Summary");
        assert_eq!(s.raw, RawValue::Scalar("hello world".into()));
    }

    #[test]
    fn parse_json_literal() {
        let s = SetArg::parse(r#"Labels=["a","b"]"#).unwrap();
        assert!(matches!(s.raw, RawValue::Json(_)));
    }

    #[test]
    fn parse_file_ref() {
        let s = SetArg::parse("Description=@./desc.md").unwrap();
        assert_eq!(s.raw, RawValue::File("./desc.md".into()));
    }

    #[test]
    fn parse_stdin_ref() {
        let s = SetArg::parse("customfield_10020=@-").unwrap();
        assert_eq!(s.raw, RawValue::Stdin);
    }

    #[test]
    fn missing_equals_errors() {
        assert!(SetArg::parse("no-equals-sign").is_err());
    }

    #[test]
    fn allows_equals_in_value() {
        let s = SetArg::parse("URL=https://foo.example?a=b").unwrap();
        assert_eq!(s.key, "URL");
        assert_eq!(s.raw, RawValue::Scalar("https://foo.example?a=b".into()));
    }
}

#[cfg(test)]
mod tests_field_alias {
    use super::*;

    fn make_args(aliases: Vec<String>) -> GlobalArgs {
        GlobalArgs {
            verbose: 0,
            output: None,
            pretty: false,
            fields: None,
            timeout: None,
            insecure: false,
            field_alias: aliases,
        }
    }

    #[test]
    fn parses_valid_pairs() {
        let args = make_args(vec![
            "Story Points=customfield_10006".into(),
            " Epic Link = customfield_10000 ".into(),
        ]);
        let map = args.parse_field_aliases().unwrap();
        assert_eq!(map["Story Points"], "customfield_10006");
        assert_eq!(map["Epic Link"], "customfield_10000");
    }

    #[test]
    fn missing_equals_errors() {
        let args = make_args(vec!["bad".into()]);
        assert!(args.parse_field_aliases().is_err());
    }

    #[test]
    fn empty_name_errors() {
        let args = make_args(vec!["=x".into()]);
        assert!(args.parse_field_aliases().is_err());
    }

    #[test]
    fn empty_id_errors() {
        let args = make_args(vec!["Name=".into()]);
        assert!(args.parse_field_aliases().is_err());
    }
}
