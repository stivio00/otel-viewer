//! otel-viewer desktop shell.
//!
//! Starts the collector in-process (OTLP/gRPC receiver + REST API + web UI,
//! all from the `otel-viewer` library crate) and shows it in a native window.
//!
//! The window is served through the custom `otelview://` URI scheme: scheme
//! requests are answered by calling the REST router in-process, so the
//! webview never makes a loopback HTTP connection. That sidesteps macOS
//! App Transport Security and the macOS 26 (Tahoe) Local Network privacy
//! controls, which silently block WKWebView navigations to plain http://
//! 127.0.0.1:<port> even with ATS exceptions in place. The same router still
//! listens on a real TCP port (127.0.0.1:6666 with fallbacks) for external
//! use — same UI, same API, e.g. from a browser — and the OTLP/gRPC receiver
//! binds 4317 (with fallbacks).

use std::borrow::Cow;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use axum::body::Body;
use tauri::http as tauri_http;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tokio_util::sync::CancellationToken;

/// Web UI / REST API port candidates (6666 matches the CLI default).
const HTTP_PORTS: &[u16] = &[6666, 6667, 6668, 6669, 6670];
/// OTLP/gRPC port candidates (4317 is the OTLP standard port).
const OTLP_PORTS: &[u16] = &[4317, 4318, 4319, 4320, 4321];
/// Custom URI scheme under which the webview sees the REST router.
const UI_SCHEME: &str = "otelview";

struct Server {
    shutdown: Option<CancellationToken>,
    runtime: Option<tokio::runtime::Runtime>,
}

/// The REST router, served to the webview via the `otelview://` scheme.
#[derive(Clone)]
struct UiRouter(axum::Router);

static FIRST_SCHEME_REQUEST: AtomicBool = AtomicBool::new(true);

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

/// Window URL for the custom scheme. Windows (WebView2) cannot serve
/// non-standard protocols directly, so wry intercepts them via
/// WebResourceRequested as `{http|https}://<scheme>.localhost/` — with `http`
/// by default; `use_https_scheme(true)` on the window builder opts into the
/// `https` form (secure context). macOS/Linux use the scheme directly.
fn scheme_url() -> url::Url {
    if cfg!(windows) {
        url::Url::parse(&format!("https://{UI_SCHEME}.localhost/"))
    } else {
        url::Url::parse(&format!("{UI_SCHEME}://localhost/"))
    }
    .expect("valid scheme url")
}

/// Answer one `otelview://` request by calling the in-process REST router.
async fn proxy_request(
    router: axum::Router,
    request: tauri_http::Request<Vec<u8>>,
) -> tauri_http::Response<Cow<'static, [u8]>> {
    use tower::ServiceExt;

    if FIRST_SCHEME_REQUEST.swap(false, Ordering::Relaxed) {
        eprintln!("[otel-viewer] webview is being served via the {UI_SCHEME}: scheme");
    }

    let uri = request
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_owned())
        .unwrap_or_else(|| "/".to_owned());
    let mut builder = axum::http::Request::builder()
        .method(request.method().clone())
        .uri(uri);
    for (name, value) in request.headers().iter() {
        builder = builder.header(name, value);
    }
    let req = match builder.body(Body::from(request.into_body())) {
        Ok(r) => r,
        Err(e) => return error_response(400, format!("bad request: {e}")),
    };

    match router.oneshot(req).await {
        Ok(resp) => {
            let (parts, body) = resp.into_parts();
            let bytes = axum::body::to_bytes(body, usize::MAX)
                .await
                .unwrap_or_default();
            let mut b = tauri_http::Response::builder().status(parts.status);
            for (name, value) in parts.headers.iter() {
                b = b.header(name, value);
            }
            b.body(bytes.as_ref().to_vec().into())
                .unwrap_or_else(|_| tauri_http::Response::new(Cow::Borrowed(&[] as &[u8])))
        }
        Err(e) => error_response(500, format!("router error: {e}")),
    }
}

fn error_response(status: u16, msg: String) -> tauri_http::Response<Cow<'static, [u8]>> {
    tauri_http::Response::builder()
        .status(status)
        .body(msg.into_bytes().into())
        .unwrap_or_else(|_| tauri_http::Response::new(Cow::Borrowed(&[] as &[u8])))
}

/// Open the app database and seed demo data if it is fresh.
async fn open_db(db_file: &std::path::Path) -> anyhow::Result<Arc<otel_viewer::db::Db>> {
    let db = Arc::new(otel_viewer::db::Db::new(db_file.to_str()).await?);
    let n = otel_viewer::demo::seed_if_empty(&db).await?;
    if n > 0 {
        eprintln!("[otel-viewer] seeded {n} demo rows");
    }
    anyhow::Ok(db)
}

/// DuckDB raises an IO "lock" error when another process (typically the
/// otel-viewer CLI or a second copy of the desktop app) already has the
/// database file open — recoverable once that process exits.
fn is_lock_conflict(e: &anyhow::Error) -> bool {
    format!("{e:#}").to_lowercase().contains("lock")
}

/// Blocking native alert for the lock-conflict case: Retry re-attempts the
/// open (after the user closed the other otel-viewer process), Quit exits.
fn lock_conflict_dialog(e: &anyhow::Error) -> bool {
    matches!(
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::OkCancelCustom(
                "Retry".into(),
                "Quit".into()
            ))
            .set_title("otel-viewer — database is locked")
            .set_description(format!(
                "Another otel-viewer process (e.g. the otel-viewer CLI or a \
                 second copy of this app) already has the database open.\n\n\
                 Close that process, then press Retry.\n\n{e:#}"
            ))
            .show(),
        rfd::MessageDialogResult::Custom(button) if button == "Retry"
    )
}

/// Blocking native alert for unrecoverable setup failures; the app exits
/// once it is dismissed.
fn fatal_dialog(title: &str, description: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .set_title(title)
        .set_description(description)
        .show();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .register_asynchronous_uri_scheme_protocol(UI_SCHEME, |ctx, request, responder| {
            let router = ctx.app_handle().state::<UiRouter>().0.clone();
            tauri::async_runtime::spawn(async move {
                let response = proxy_request(router, request).await;
                responder.respond(response);
            });
        })
        .setup(|app| {
            // Persistent DuckDB database in the OS app-data directory.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_file = data_dir.join("otel-viewer.duckdb");

            // Bind before anything slow so external clients never race the
            // servers; queued connections are accepted once the tasks spin up.
            let http = bind_with_fallback(HTTP_PORTS)?;
            let grpc = bind_with_fallback(OTLP_PORTS)?;
            let otlp_addr = grpc.local_addr().ok().map(|a| a.to_string());

            let shutdown = CancellationToken::new();
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;

            // Open the database and build the REST router before the window
            // loads, so the first scheme request is always answerable. A
            // lock conflict (another otel-viewer process holds the file) is
            // recoverable: ask the user to close it and retry instead of
            // panicking into a crash report.
            let db = loop {
                match runtime.block_on(open_db(&db_file)) {
                    Ok(db) => break db,
                    Err(e) if is_lock_conflict(&e) => {
                        eprintln!("[otel-viewer] database is locked: {e:#}");
                        if !lock_conflict_dialog(&e) {
                            app.handle().exit(1);
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        eprintln!("[otel-viewer] cannot open database: {e:#}");
                        fatal_dialog(
                            "otel-viewer — cannot open database",
                            &format!("Failed to open {}:\n\n{e:#}", db_file.display()),
                        );
                        app.handle().exit(1);
                        return Ok(());
                    }
                }
            };
            app.manage(UiRouter(otel_viewer::api::router(
                db.clone(),
                otel_viewer::api::Meta { otlp_addr },
            )));

            let token = shutdown.clone();
            runtime.spawn(async move {
                if let Err(e) = otel_viewer::run_with_db(db, http, grpc, token).await {
                    eprintln!("[otel-viewer] server error: {e:#}");
                }
            });

            app.manage(Mutex::new(Server {
                shutdown: Some(shutdown),
                runtime: Some(runtime),
            }));

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(scheme_url()))
                .title("otel-viewer")
                .inner_size(1440.0, 900.0)
                .min_inner_size(960.0, 600.0)
                // Windows: make the webview intercept the custom scheme in
                // its `https://otelview.localhost/` form (see scheme_url).
                .use_https_scheme(true)
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Early-exit paths (fatal dialog before setup finished)
                // never manage Server.
                if let Some(state) = app.try_state::<Mutex<Server>>() {
                    let mut server = state.lock().unwrap();
                    if let Some(token) = server.shutdown.take() {
                        token.cancel();
                    }
                    if let Some(rt) = server.runtime.take() {
                        // Let the collector drain (DuckDB flush) before exit.
                        rt.shutdown_timeout(Duration::from_secs(2));
                    }
                }
            }
        });
}
