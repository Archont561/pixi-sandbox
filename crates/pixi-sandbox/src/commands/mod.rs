//! Command implementations.
//!
//! `pack`, `publish`, `restore`, `unpack`, `doctor`, `plan`, `tools list`, and `tools update`
//! share one manifest/config contract. `unpack` is the single-environment primitive that
//! `restore` drives once per env, and is also the escape hatch for a payload that arrived
//! outside the branch flow. `.knowledge/design.md` specifies every verb; there are no deferred
//! operations left, so nothing here can send an operator to a section instead of doing the work.

mod doctor;
mod init;
mod pack;
mod plan;
mod publish;
mod restore;
mod self_update;
mod support;
mod tools;
mod unpack;

pub use doctor::run as doctor;
pub use init::run as init;
pub use pack::run as pack;
pub use plan::run as plan;
pub use publish::run as publish;
pub use restore::run as restore;
pub use self_update::run as self_update;
pub use tools::run as tools;
pub use unpack::run as unpack;

use tracing_subscriber::{EnvFilter, fmt};

pub fn install_tracing() {
    let filter =
        EnvFilter::try_from_env("PIXI_SANDBOX_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).without_time().try_init();
}
