//! Reusable product logic for the `pixi-sandbox` CLI.
//!
//! Repository automation depends on these APIs instead of spawning the binary through a nested
//! `cargo run`, so generated artifacts have one implementation shared by the CLI, tests, and
//! `crates/xtask`.

pub mod branch_docs;
pub mod build_tool_fetch;
pub mod embedded_tools;
pub mod generated;
pub mod github;
pub mod host_probe;
pub mod native_pack;
pub mod nested;
pub mod pack;
pub mod pipeline;
pub mod pixi_lock;
pub mod prefix;
pub mod prepare;
pub mod rattler_build;
pub mod release;
pub mod self_update;
pub mod standalone;
pub mod upgrade;
pub mod user_tools;
pub mod vendor;
pub mod wrap_payload;
