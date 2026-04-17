//! Shared CLI arg structs.

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
