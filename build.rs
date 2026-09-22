//! Compiles the vendored OpenTelemetry protos (from
//! https://github.com/open-telemetry/opentelemetry-proto, tag v1.11.0) into
//! Rust modules with tonic server/client codegen.

use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use a vendored protoc when none is installed on the system.
    if env::var_os("PROTOC").is_none()
        && let Ok(protoc) = protoc_bin_vendored::protoc_bin_path()
    {
        // build scripts are single threaded before code generation
        unsafe { env::set_var("PROTOC", protoc) };
    }

    let out = PathBuf::from(env::var("OUT_DIR")?);

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out.join("otelv.bin"))
        .compile_protos(
            &[
                "proto/opentelemetry/proto/collector/trace/v1/trace_service.proto",
                "proto/opentelemetry/proto/collector/metrics/v1/metrics_service.proto",
                "proto/opentelemetry/proto/collector/logs/v1/logs_service.proto",
            ],
            &["proto"],
        )?;

    for entry in walk(proto_dir()) {
        if entry.extension().is_some_and(|e| e == "proto") {
            println!("cargo:rerun-if-changed={}", entry.display());
        }
    }

    // rust-embed fails to compile when web/dist is missing (e.g. on a fresh
    // clone before the frontend has been built). Ensure the folder exists with
    // a minimal placeholder; `pnpm build` replaces it with the real bundle.
    let dist = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("web/dist");
    if !dist.exists() {
        std::fs::create_dir_all(&dist)?;
        std::fs::write(
            dist.join("index.html"),
            "<!doctype html><title>otel-viewer</title>\
             <p>UI not built. Run <code>pnpm --dir web build</code> and restart.</p>",
        )?;
    }

    Ok(())
}

fn proto_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("proto")
}

fn walk(dir: PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                out.extend(walk(p));
            } else {
                out.push(p);
            }
        }
    }
    out
}
