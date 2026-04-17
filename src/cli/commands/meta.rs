//! `ping`, `whoami`, `config show`.

use crate::api::meta;
use crate::cli::args::GlobalArgs;
use crate::cli::ConfigCmd;
use crate::config::JiraConfig;
use crate::error::Result;
use crate::http::HttpClient;
use crate::output::{emit_value, Format};
use std::io::Write;

pub fn ping<W: Write>(out: &mut W, client: &HttpClient, g: &GlobalArgs) -> Result<()> {
    let value = meta::server_info(client)?;
    let fields = g.field_list();
    emit_value(
        out,
        value,
        &g.output_options(Format::Json, fields.as_deref()),
    )
}

pub fn whoami<W: Write>(out: &mut W, client: &HttpClient, g: &GlobalArgs) -> Result<()> {
    let value = meta::myself(client)?;
    let fields = g.field_list();
    emit_value(
        out,
        value,
        &g.output_options(Format::Json, fields.as_deref()),
    )
}

pub fn config_show<W: Write>(out: &mut W, cfg: &JiraConfig, g: &GlobalArgs) -> Result<()> {
    let fields = g.field_list();
    emit_value(
        out,
        cfg.redacted_json(),
        &g.output_options(Format::Json, fields.as_deref()),
    )
}

pub fn config(
    out: &mut impl Write,
    cfg: &JiraConfig,
    g: &GlobalArgs,
    cmd: &ConfigCmd,
) -> Result<()> {
    match cmd {
        ConfigCmd::Show => config_show(out, cfg, g),
    }
}
