//! Self-describing CLI schema generation via clap introspection.
//!
//! Output shape:
//! { "version": "x.y.z", "commands": { "<name>": {
//!     "about": "...",
//!     "args":  [{ "name": "...", "required": bool, "value_type": "string|path|...", "help": "..." }],
//!     "flags": [{ "name": "--...", "short": "..", "help": "...", "value_name": "..." }],
//!     "subcommands": { ... recursive ... }
//! } } }

use clap::{Arg, ArgAction, Command};
use serde_json::{json, Value};

pub fn emit(cli: &Command) -> Value {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "commands": render_subs(cli),
    })
}

pub fn emit_sub(cli: &Command, name: &str) -> Option<Value> {
    cli.get_subcommands()
        .find(|c| c.get_name() == name)
        .map(render_one)
}

fn render_subs(c: &Command) -> Value {
    let mut map = serde_json::Map::new();
    for sub in c.get_subcommands() {
        map.insert(sub.get_name().to_string(), render_one(sub));
    }
    Value::Object(map)
}

fn render_one(c: &Command) -> Value {
    let (positional, optional): (Vec<&Arg>, Vec<&Arg>) =
        c.get_arguments().partition(|a| a.is_positional());

    let args: Vec<Value> = positional
        .iter()
        .map(|a| {
            json!({
                "name": a.get_id().to_string(),
                "required": a.is_required_set(),
                "help": a.get_help().map(|h| h.to_string()).unwrap_or_default(),
                "value_type": value_type(a),
            })
        })
        .collect();

    let flags: Vec<Value> = optional
        .iter()
        .map(|a| {
            json!({
                "name": a.get_long().map(|s| format!("--{s}")).unwrap_or_default(),
                "short": a.get_short().map(String::from),
                "required": a.is_required_set(),
                "takes_value": !matches!(a.get_action(), ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count | ArgAction::Help | ArgAction::Version),
                "value_name": a.get_value_names().and_then(|v| v.first().map(|s| s.to_string())),
                "help": a.get_help().map(|h| h.to_string()).unwrap_or_default(),
                "value_type": value_type(a),
            })
        })
        .collect();

    json!({
        "about": c.get_about().map(|h| h.to_string()).unwrap_or_default(),
        "args": args,
        "flags": flags,
        "subcommands": render_subs(c),
    })
}

fn value_type(a: &Arg) -> &'static str {
    match a.get_action() {
        ArgAction::SetTrue | ArgAction::SetFalse => "bool",
        ArgAction::Count => "count",
        _ => "string",
    }
}
