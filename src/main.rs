use clap::Parser;
use jira_cli::cli::{dispatch, Cli};
use jira_cli::config::JiraConfig;
use jira_cli::http::HttpClient;

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
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();

    // Commands that don't need config/client
    use jira_cli::cli::{Command, ConfigCmd};
    match &cli.cmd {
        Command::Schema(a) => {
            jira_cli::cli::commands::schema::run(&mut lock, a, cli.global.pretty)?;
            lock.flush()?;
            return Ok(());
        }
        Command::Config(ConfigCmd::Init(args)) => {
            jira_cli::cli::commands::meta::config_init(&mut lock, args)?;
            lock.flush()?;
            return Ok(());
        }
        _ => {}
    }

    let mut cfg = JiraConfig::load()?;
    if let Some(t) = cli.global.timeout {
        cfg.timeout_secs = t;
    }
    if cli.global.insecure {
        cfg.insecure = true;
    }
    let client = HttpClient::new(&cfg)?;
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
