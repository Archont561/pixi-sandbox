//! `pixi-sandbox-core` — everything that has to be right for the transport to be trustworthy.
//!
//! Split from the CLI on purpose: this crate is the part that should be reused (and fuzzed,
//! and property-tested) while the CLI stays a thin shell. The pieces:
//!
//! * [`manifest`] — the wire format (`manifest.json`, schema 1) and its validation rules;
//! * [`shard`] — the only two file operations in the project: `record_file` (split when
//!   oversized) and `materialise`/`join_parts` (verify, then write, never a half file);
//! * [`tools_lock`] — embedded helper-tool pins plus explicit external overrides (decision D4);
//! * [`sandbox_config`] — a reviewed `.pixi-sandbox.toml` publish matrix;
//! * [`verify`] — walk a transport and check every declared byte; also the ELF linkage check
//!   that stops a dynamically linked tool from being shipped to an airlock.

pub mod error;
pub mod manifest;
pub mod sandbox_config;
pub mod shard;
pub mod tools_lock;
pub mod verify;

pub use error::{Error, Result};
