//! Reusable product logic for the `pixi-sandbox` CLI.
//!
//! Repository automation depends on these APIs instead of spawning the binary through a nested
//! `cargo run`, so generated artifacts have one implementation shared by the CLI, tests, and
//! `crates/xtask`.

pub mod generated;
