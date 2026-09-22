use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "otel-viewer",
    version,
    about = "Small OTLP/gRPC collector that stores traces, metrics and logs in DuckDB, with a built-in web UI and REST query API.",
    disable_help_flag = true
)]
pub struct Cli {
    /// DuckDB database file. When not set, an in-memory database is used.
    #[arg(short, long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Host and port used to serve the web UI and REST API.
    #[arg(short = 'h', long = "host", default_value = "localhost:6666", value_name = "HOST:PORT")]
    pub host: String,

    /// Listen address for the OTLP gRPC receiver.
    #[arg(short = 'o', long = "otlp", default_value = "0.0.0.0:4317", value_name = "HOST:PORT")]
    pub otlp: String,

    /// Seed the database with demo telemetry on startup (ignored when the
    /// database already has data).
    #[arg(long)]
    pub seed_demo: bool,

    /// Increase log verbosity (-v: debug, -vv: trace).
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    /// Print help.
    #[arg(long, action = ArgAction::Help)]
    help: Option<bool>,
}
