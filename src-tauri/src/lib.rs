//! otel-viewer desktop shell.
//!
//! Starts the collector in-process (OTLP/gRPC receiver + REST API + embedded
//! web UI, all from the `otel-viewer` library crate) and shows it in a native
//! window pointed at the local HTTP server — same origin for UI and API, so
//! no CORS or Tauri-IPC plumbing is needed.

use std::net::TcpListener;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Context;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tokio_util::sync::CancellationToken;

/// Web UI / REST API port candidates (6666 matches the CLI default).
const HTTP_PORTS: &[u16] = &[6666, 6667, 6668, 6669, 6670];
/// OTLP/gRPC port candidates (4317 is the OTLP standard port).
const OTLP_PORTS: &[u16] = &[4317, 4318, 4319, 4320, 4321];

struct Server {
    shutdown: Option<CancellationToken>,
    runtime: Option<tokio::runtime::Runtime>,
}

/// Bind the first free port from `ports`; fall back to an OS-assigned port so
/// the app always starts (the actual addresses are reported via /api/health).
fn bind_with_fallback(ports: &[u16]) -> anyhow::Result<TcpListener> {
    for port in ports {
        if let Ok(l) = TcpListener::bind(("127.0.0.1", *port)) {
            return Ok(l);
        }
    }
    TcpListener::bind(("127.0.0.1", 0)).context("no free listen port")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .setup(|app| {
            // Persistent DuckDB database in the OS app-data directory.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_file = data_dir.join("otel-viewer.duckdb");

            // Bind before anything slow so the window never races the server:
            // queued connections are accepted once the server task spins up.
            let http = bind_with_fallback(HTTP_PORTS)?;
            let grpc = bind_with_fallback(OTLP_PORTS)?;
            let ui_port = http.local_addr()?.port();

            let shutdown = CancellationToken::new();
            let cfg = otel_viewer::Config {
                db_file: Some(db_file.to_string_lossy().into_owned()),
                seed_demo: true,
            };

            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let token = shutdown.clone();
            runtime.spawn(async move {
                if let Err(e) = otel_viewer::run(&cfg, http, grpc, token).await {
                    eprintln!("[otel-viewer] server error: {e:#}");
                }
            });

            app.manage(Mutex::new(Server {
                shutdown: Some(shutdown),
                runtime: Some(runtime),
            }));

            let url = url::Url::parse(&format!("http://127.0.0.1:{ui_port}"))?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("otel-viewer")
                .inner_size(1440.0, 900.0)
                .min_inner_size(960.0, 600.0)
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<Mutex<Server>>();
                let mut server = state.lock().unwrap();
                if let Some(token) = server.shutdown.take() {
                    token.cancel();
                }
                if let Some(rt) = server.runtime.take() {
                    // Let the collector drain (DuckDB flush) before exit.
                    rt.shutdown_timeout(Duration::from_secs(2));
                }
            }
        });
}
