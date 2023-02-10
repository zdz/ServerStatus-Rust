use chrono::Utc;
use std::process::Command;

fn commit_hash() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|hash| hash.trim().into())
}

fn main() {
    let mut app_version = String::from(env!("CARGO_PKG_VERSION"));
    if let Some(commit_hash) = commit_hash() {
        app_version = format!(
            "v{} ({}, {})",
            app_version,
            commit_hash,
            Utc::now().format("%Y-%m-%d %H:%M:%S %Z")
        );
    }
    println!("cargo:rustc-env=APP_VERSION={app_version}");

    #[cfg(not(target_env = "msvc"))]
    std::env::set_var("PROTOC", protobuf_src::protoc());

    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile(&["proto/server_status.proto"], &["proto"])
        .unwrap();
}
