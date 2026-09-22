#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    otel_viewer_tauri::run()
}
