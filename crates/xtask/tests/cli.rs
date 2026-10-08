//! The clap dispatch glue in `main.rs` — genuinely private to the binary target, so it is
//! covered black-box by driving `xtask` (the AGENTS.md route). Moved from `main.rs`'s inline
//! `#[cfg(test)]` (TASK-83): test name kept, behaviour identical — the same subcommand with
//! the same arguments must reach the downloader's platform validation and report its refusal
//! through the `::error::` line `main` prints.

use std::process::Command;

#[test]
fn airlock_self_bin_subcommand_reaches_the_downloader_validation() {
    let root = tempfile::tempdir().expect("tempdir");
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "--root",
            root.path().to_str().expect("utf-8 root"),
            "airlock-self-bin",
            "--repo",
            "owner/repo",
            "--tag",
            "v0.4.0",
            "--platform",
            "freebsd-64",
            "--out",
            "pixi-sandbox",
        ])
        .output()
        .expect("run xtask");

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no static pixi-sandbox release asset"),
        "{stderr}"
    );
}
