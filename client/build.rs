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

fn rustc_version() -> Option<String> {
    Command::new("rustc")
        .args(["--version"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| {
            s.split_whitespace()
                .enumerate()
                .filter(|&(idx, _)| idx < 2)
                .map(|(_, s)| s)
                .collect::<Vec<&str>>()
                .join(" ")
        })
}

fn main() {
    let mut app_version = String::from(env!("CARGO_PKG_VERSION"));
    if let Some(commit_hash) = commit_hash() {
        app_version = format!(
            "v{} ({}, {}, {}, {})",
            app_version,
            commit_hash,
            Utc::now().format("%Y-%m-%d %H:%M:%S %Z"),
            rustc_version().unwrap(),
            std::env::var("TARGET").unwrap(),
        );
    }
    println!("cargo:rustc-env=APP_VERSION={app_version}");
}
