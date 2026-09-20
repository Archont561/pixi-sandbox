//! Command implementations.
//!
//! `pack`, `publish`, `restore`, `unpack`, `doctor`, `plan`, and `tools list` share one
//! manifest/config contract. `unpack` is the single-environment primitive that `restore` drives once per env,
//! and is also the escape hatch for a payload that arrived outside the branch flow. The Python
//! prototype in `.knowledge/research/pixi_sandbox.py` remains a reproducibility reference;
//! `publish --keep` and `tools update` are intentionally the only deferred CLI operations.

mod doctor;
mod pack;
mod plan;
mod publish;
mod restore;
mod support;
mod tools;
mod unpack;

pub use doctor::run as doctor;
pub use pack::run as pack;
pub use plan::run as plan;
pub use publish::run as publish;
pub use restore::run as restore;
pub use tools::run as tools;
pub use unpack::run as unpack;

use tracing_subscriber::{EnvFilter, fmt};

pub fn install_tracing() {
    let filter =
        EnvFilter::try_from_env("PIXI_SANDBOX_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).without_time().try_init();
}

/// Every not-yet-implemented path points at the design section that specifies it.
pub(crate) fn not_yet(verb: &str, section: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "`{verb}` is not implemented yet (schema + behaviour are specified in \
         .knowledge/design.md §{section}; the Python prototype in .knowledge/research/ \
         implements it end-to-end and is the reference for this port)"
    )
}
