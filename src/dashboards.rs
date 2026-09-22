//! Dashboard definitions: built-in YAML files embedded from `src/dashboards`
//! at build time, plus user-supplied YAML files from
//! `~/.otel-viewer/dashboards/` (rescanned when the directory changes).
//!
//! A dashboard document is passed through to the UI as-is (parsed YAML →
//! JSON); the only fields interpreted here are the list metadata
//! (`name`, `title`, `description`).

use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

use rust_embed::RustEmbed;
use serde::Serialize;
use serde_json::Value as Json;

#[derive(RustEmbed)]
#[folder = "src/dashboards"]
struct EmbeddedDashboards;

#[derive(Serialize, Clone, Debug)]
pub struct DashboardMeta {
    pub id: String,
    pub source: &'static str,
    pub name: String,
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct DashboardFull {
    pub id: String,
    pub source: &'static str,
    pub name: String,
    pub title: String,
    pub description: String,
    /// Full parsed YAML document (inputs, panels, …) as JSON.
    pub spec: Json,
}

#[derive(Serialize)]
pub struct DashboardsResponse {
    pub dashboards: Vec<DashboardMeta>,
}

struct Cache {
    map: BTreeMap<String, DashboardFull>,
    user_dir_mtime: Option<SystemTime>,
}

static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

fn user_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let dir = PathBuf::from(home).join(".otel-viewer").join("dashboards");
    dir.is_dir().then_some(dir)
}

fn parse_doc(id: &str, source: &'static str, bytes: &[u8]) -> Option<DashboardFull> {
    let spec: Json = serde_yaml::from_slice(bytes).ok()?;
    let pick = |k: &str| {
        spec.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let name = {
        let n = pick("name");
        if n.is_empty() { id.to_string() } else { n }
    };
    let title = {
        let t = pick("title");
        if t.is_empty() { name.clone() } else { t }
    };
    Some(DashboardFull {
        id: id.to_string(),
        source,
        name,
        title,
        description: pick("description"),
        spec,
    })
}

fn embedded() -> impl Iterator<Item = DashboardFull> {
    EmbeddedDashboards::iter().filter_map(|f| {
        let id = f
            .trim_end_matches(".yml")
            .trim_end_matches(".yaml")
            .to_string();
        let id = id.rsplit('/').next().unwrap_or(&id).to_string();
        EmbeddedDashboards::get(f.as_ref()).and_then(|e| parse_doc(&id, "builtin", &e.data))
    })
}

fn user_files(dir: &PathBuf) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if (ext == "yml" || ext == "yaml") && p.is_file() {
                let id = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                if !id.is_empty() {
                    out.push((id, p));
                }
            }
        }
    }
    out.sort();
    out
}

fn build() -> BTreeMap<String, DashboardFull> {
    let mut map = BTreeMap::new();
    for d in embedded() {
        map.insert(d.id.clone(), d);
    }
    if let Some(dir) = user_dir() {
        for (id, path) in user_files(&dir) {
            if let Ok(bytes) = fs::read(&path)
                && let Some(d) = parse_doc(&id, "user", &bytes)
            {
                map.insert(id, d);
            }
        }
    }
    map
}

fn cache() -> &'static Mutex<Cache> {
    CACHE.get_or_init(|| {
        let map = build();
        let mtime = user_dir().and_then(|d| fs::metadata(d).ok().and_then(|m| m.modified().ok()));
        Mutex::new(Cache {
            map,
            user_dir_mtime: mtime,
        })
    })
}

/// List metadata for all dashboards (embedded + user).
pub fn list() -> Vec<DashboardMeta> {
    let mut c = cache().lock().unwrap();
    // Rescan when the user dashboard directory changed (files added/removed).
    let mtime = user_dir().and_then(|d| fs::metadata(d).ok().and_then(|m| m.modified().ok()));
    if mtime != c.user_dir_mtime {
        c.map = build();
        c.user_dir_mtime = mtime;
    }
    c.map
        .values()
        .map(|d| DashboardMeta {
            id: d.id.clone(),
            source: d.source,
            name: d.name.clone(),
            title: d.title.clone(),
            description: d.description.clone(),
        })
        .collect()
}

/// Full dashboard document by id.
pub fn get(id: &str) -> Option<DashboardFull> {
    let c = cache().lock().unwrap();
    c.map.get(id).cloned()
}

/// Path of the user dashboard directory, if it exists (for /api/health).
pub fn user_dashboards_dir() -> Option<String> {
    user_dir().map(|p| p.display().to_string())
}
