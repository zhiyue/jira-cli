use clap::Parser;
use jira_cli::cli::{dispatch, Cli};
use jira_cli::config::JiraConfig;
use jira_cli::http::HttpClient;
use std::io::Write;

fn main() {
    let cli = Cli::parse();

    init_tracing(cli.global.verbose);

    let code = match try_main(&cli) {
        Ok(()) => 0,
        Err(err) => err.emit_stderr(),
    };
    std::process::exit(code);
}

fn try_main(cli: &Cli) -> jira_cli::Result<()> {
    let mut cfg = JiraConfig::from_env()?;
    if let Some(t) = cli.global.timeout {
        cfg.timeout_secs = t;
    }
    if cli.global.insecure {
        cfg.insecure = true;
    }
    let client = HttpClient::new(&cfg)?;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    dispatch::run(&mut lock, &cfg, &client, cli)?;
    lock.flush()?;
    Ok(())
}

fn init_tracing(verbosity: u8) {
    use tracing_subscriber::EnvFilter;
    let level = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("jira_cli={level}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .json()
        .init();
}
