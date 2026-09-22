//! otel-viewer: a small OTLP/gRPC collector that stores traces, metrics and
//! logs in DuckDB and serves a web UI plus REST query API.

pub mod api;
pub mod cli;
pub mod convert;
pub mod db;
pub mod demo;
pub mod grpc;
pub mod proto;
pub mod queries;
pub mod rows;
pub mod sqlguard;

use std::net::TcpListener;
use std::time::Duration;

use anyhow::Context;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_file: Option<String>,
    pub seed_demo: bool,
}

/// Run the collector: OTLP gRPC receiver + HTTP (REST API + web UI).
/// Returns when `shutdown` is cancelled or a server errors.
pub async fn run(
    cfg: &Config,
    http_listener: TcpListener,
    grpc_listener: TcpListener,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let db = std::sync::Arc::new(db::Db::new(cfg.db_file.as_deref()).await?);

    if cfg.seed_demo {
        let n = demo::seed_if_empty(&db).await?;
        if n > 0 {
            tracing::info!("seeded {n} demo rows");
        }
    }

    run_with_db(db, http_listener, grpc_listener, shutdown).await
}

/// Like [`run`], but with an existing database handle (seeding is the
/// caller's job). The desktop shell uses this so it can also serve the
/// REST router to its webview in-process, sharing the same database.
pub async fn run_with_db(
    db: std::sync::Arc<db::Db>,
    http_listener: TcpListener,
    grpc_listener: TcpListener,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let http_listener = {
        let l = http_listener;
        l.set_nonblocking(true)?;
        tokio::net::TcpListener::from_std(l)?
    };
    let otlp_addr = grpc_listener.local_addr().ok().map(|a| a.to_string());
    let grpc_listener = {
        let l = grpc_listener;
        l.set_nonblocking(true)?;
        tokio::net::TcpListener::from_std(l)?
    };

    let mut tasks = tokio::task::JoinSet::new();

    {
        let shutdown = shutdown.clone();
        let db = db.clone();
        tasks.spawn(async move {
            let router = grpc::router(db);
            tokio::select! {
                r = router.serve_with_incoming(TcpListenerStream::new(grpc_listener)) => {
                    r.context("gRPC server failed")
                }
                _ = shutdown.cancelled() => Ok(()),
            }
        });
    }

    {
        let shutdown = shutdown.clone();
        let db = db.clone();
        tasks.spawn(async move {
            let app = api::router(db, api::Meta { otlp_addr });
            tokio::select! {
                r = axum::serve(http_listener, app) => {
                    r.context("HTTP server failed")
                }
                _ = shutdown.cancelled() => Ok(()),
            }
        });
    }

    let mut result: anyhow::Result<()> = Ok(());
    tokio::select! {
        joined = tasks.join_next() => {
            shutdown.cancel();
            result = match joined {
                Some(Ok(r)) => r,
                Some(Err(e)) => Err(anyhow::anyhow!("server task panicked: {e}")),
                None => Ok(()),
            };
        }
        _ = shutdown.cancelled() => {}
    }
    shutdown.cancel();

    // Drain: tasks exit promptly once cancelled; don't hang forever.
    let _ = tokio::time::timeout(Duration::from_secs(3), async {
        while tasks.join_next().await.is_some() {}
    })
    .await;

    result
}
