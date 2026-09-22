use std::net::{SocketAddr, TcpListener, ToSocketAddrs};

use anyhow::Context;
use clap::Parser;
use otel_viewer::{Config, cli::Cli, run};
use tokio_util::sync::CancellationToken;

fn resolve_addr(addr: &str) -> anyhow::Result<SocketAddr> {
    addr.to_socket_addrs()
        .with_context(|| format!("invalid address: {addr}"))?
        .next()
        .with_context(|| format!("address did not resolve: {addr}"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| level.into()),
        )
        .init();

    let http_addr = resolve_addr(&cli.host)?;
    let grpc_addr = resolve_addr(&cli.otlp)?;
    let http_listener = TcpListener::bind(http_addr).with_context(|| {
        format!("cannot bind web host {http_addr} (is another otel-viewer running?)")
    })?;
    let grpc_listener = TcpListener::bind(grpc_addr)
        .with_context(|| format!("cannot bind OTLP receiver {grpc_addr}"))?;

    let db_display = cli
        .file
        .as_ref()
        .map(|f| f.display().to_string())
        .unwrap_or_else(|| "in-memory".to_string());
    println!(
        "\
otel-viewer {}
  web UI + REST API : http://{}
  OTLP gRPC receiver: {} (traces/metrics/logs)
  DuckDB            : {}
  REST query console: POST /api/query  (read-only SQL)
",
        env!("CARGO_PKG_VERSION"),
        http_addr,
        grpc_addr,
        db_display,
    );

    let shutdown = CancellationToken::new();
    {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                let term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate());
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = async {
                        match term {
                            Ok(mut s) => { s.recv().await; }
                            Err(_) => std::future::pending::<()>().await,
                        }
                    } => {}
                }
            }
            #[cfg(not(unix))]
            let _ = tokio::signal::ctrl_c().await;
            shutdown.cancel();
        });
    }

    let cfg = Config {
        db_file: cli.file.as_ref().map(|f| f.display().to_string()),
        seed_demo: cli.seed_demo,
    };
    run(&cfg, http_listener, grpc_listener, shutdown).await
}
