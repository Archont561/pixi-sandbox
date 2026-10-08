//! Library surface of the repository automation modules (TASK-83).
//!
//! The policy logic lives here as `pub mod` so each `tests/<module>.rs` can exercise its
//! module through the public API — the Test conventions shape — while `main.rs` keeps the clap
//! dispatch glue, which is genuinely private to the binary target and covered black-box by
//! driving `xtask` from `tests/`.

pub mod airlock;
pub mod commit_release;
pub mod conda_platforms;
pub mod prepare_release;
pub mod release_assets;
pub mod release_refs;
pub mod repo_checks;
pub mod smoke;
pub mod starter;
pub mod util;
pub mod version;
pub mod workflow;
