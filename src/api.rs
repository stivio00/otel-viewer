//! REST API + static web UI serving.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rust_embed::Embed;
use serde::Deserialize;
use tower_http::cors::CorsLayer;

use crate::db::Db;
use crate::queries::{self, MetricDetailParams, MetricsParams, TracesParams};

#[derive(Embed)]
#[folder = "web/dist"]
struct Assets;

/// Extra, non-DB metadata exposed by the API (bind addresses etc.).
#[derive(Clone)]
pub struct Meta {
    pub otlp_addr: Option<String>,
}

pub fn router(db: Arc<Db>, meta: Meta) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/stats", get(stats))
        .route("/api/schema", get(schema))
        .route("/api/services", get(services))
        .route("/api/traces", get(traces))
        .route("/api/traces/{trace_id}", get(trace_detail))
        .route("/api/logs", get(logs))
        .route("/api/metrics", get(metrics))
        .route("/api/metrics/{name}", get(metric_detail))
        .route("/api/query", post(query))
        .route("/api/reset", post(reset))
        .fallback(not_found_or_static)
        .layer(CorsLayer::very_permissive())
        .layer(axum::Extension(meta))
        .with_state(db)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health(
    State(db): State<Arc<Db>>,
    axum::Extension(meta): axum::Extension<Meta>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "db_file": db.db_file(),
        "otlp_addr": meta.otlp_addr,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn stats(State(db): State<Arc<Db>>) -> Result<Json<queries::StatsResponse>, ApiError> {
    let db_file = db.db_file().map(str::to_string);
    let res = db
        .read(move |conn| queries::stats(conn, db_file.as_deref()))
        .await?;
    Ok(Json(res))
}

async fn schema(State(db): State<Arc<Db>>) -> Result<Json<queries::SchemaResponse>, ApiError> {
    let res = db.read(queries::schema).await?;
    Ok(Json(res))
}

async fn services(
    State(db): State<Arc<Db>>,
) -> Result<Json<queries::ServicesResponse>, ApiError> {
    let res = db.read(queries::services).await?;
    Ok(Json(res))
}

async fn traces(
    State(db): State<Arc<Db>>,
    Query(p): Query<TracesParams>,
) -> Result<Json<queries::TracesResponse>, ApiError> {
    let res = db.read(move |conn| queries::traces_list(conn, &p)).await?;
    Ok(Json(res))
}

async fn trace_detail(
    State(db): State<Arc<Db>>,
    Path(trace_id): Path<String>,
) -> Result<Json<queries::TraceDetail>, ApiError> {
    let res = db
        .read(move |conn| queries::trace_detail(conn, &trace_id))
        .await?;
    match res {
        Some(t) => Ok(Json(t)),
        None => Err(ApiError::NotFound("trace not found".into())),
    }
}

async fn logs(
    State(db): State<Arc<Db>>,
    Query(p): Query<queries::LogsParams>,
) -> Result<Json<queries::LogsResponse>, ApiError> {
    let res = db.read(move |conn| queries::logs_list(conn, &p)).await?;
    Ok(Json(res))
}

async fn metrics(
    State(db): State<Arc<Db>>,
    Query(p): Query<MetricsParams>,
) -> Result<Json<queries::MetricsResponse>, ApiError> {
    let res = db.read(move |conn| queries::metrics_list(conn, &p)).await?;
    Ok(Json(res))
}

async fn metric_detail(
    State(db): State<Arc<Db>>,
    Path(name): Path<String>,
    Query(p): Query<MetricDetailParams>,
) -> Result<Json<queries::MetricDetail>, ApiError> {
    let res = db
        .read(move |conn| queries::metric_detail(conn, &name, &p))
        .await?;
    match res {
        Some(m) => Ok(Json(m)),
        None => Err(ApiError::NotFound("metric not found".into())),
    }
}

#[derive(Deserialize)]
struct QueryBody {
    sql: String,
    #[serde(default)]
    limit: Option<usize>,
}

async fn query(
    State(db): State<Arc<Db>>,
    Json(body): Json<QueryBody>,
) -> Result<Json<queries::QueryResponse>, ApiError> {
    let max_rows = body.limit.unwrap_or(500).clamp(1, 10_000);
    let res = db
        .read(move |conn| queries::run_query(conn, &body.sql, max_rows))
        .await?;
    Ok(Json(res))
}

/// Delete all telemetry data (spans, logs, metric points).
async fn reset(State(db): State<Arc<Db>>) -> Result<Json<serde_json::Value>, ApiError> {
    db.reset().await.map_err(|e| ApiError::Bad(anyhow::anyhow!(e)))?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

pub enum ApiError {
    NotFound(String),
    Bad(anyhow::Error),
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        Self::Bad(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response(),
            ApiError::Bad(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response(),
        }
    }
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "not found" })),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Static web UI
// ---------------------------------------------------------------------------

async fn not_found_or_static(uri: Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/api/") {
        return not_found();
    }
    serve_asset(path).unwrap_or_else(|| index_response().unwrap_or_else(not_found_msg))
}

fn serve_asset(path: &str) -> Option<Response> {
    let name = path.trim_start_matches('/');
    let name = if name.is_empty() { "index.html" } else { name };
    let asset = Assets::get(name)?;
    let mime = mime_guess::from_path(name).first_or_octet_stream();
    let immutable = name.starts_with("assets/");
    let headers = [
        (header::CONTENT_TYPE, mime.as_ref().to_string()),
        (
            header::CACHE_CONTROL,
            if immutable {
                "public, max-age=31536000, immutable".to_string()
            } else {
                "no-cache".to_string()
            },
        ),
    ];
    (StatusCode::OK, headers, asset.data.into_owned()).into_response().into()
}

fn index_response() -> Option<Response> {
    let asset = Assets::get("index.html")?;
    let headers = [
        (header::CONTENT_TYPE, "text/html".to_string()),
        (header::CACHE_CONTROL, "no-cache".to_string()),
    ];
    Some((StatusCode::OK, headers, asset.data.into_owned()).into_response())
}

fn not_found_msg() -> Response {
    (
        StatusCode::NOT_FOUND,
        "web UI not built: run `pnpm install && pnpm build` in web/".to_string(),
    )
        .into_response()
}
