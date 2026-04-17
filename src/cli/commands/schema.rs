use crate::cli::{Cli, SchemaArgs};
use crate::error::Result;
use crate::output::{emit_value, Format, OutputOptions};
use clap::CommandFactory;
use std::io::Write;

pub fn run<W: Write>(out: &mut W, args: &SchemaArgs, pretty: bool) -> Result<()> {
    let cmd = Cli::command();
    let value = match &args.subcommand {
        Some(name) => crate::schema::emit_sub(&cmd, name)
            .ok_or_else(|| crate::error::Error::Usage(format!("unknown subcommand: {name}")))?,
        None => crate::schema::emit(&cmd),
    };
    emit_value(
        out,
        value,
        &OutputOptions {
            format: Format::Json,
            pretty,
            fields: None,
        },
    )
}
