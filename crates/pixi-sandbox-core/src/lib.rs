//! `pixi-sandbox-core` — everything that has to be right for the transport to be trustworthy.
//!
//! Split from the CLI on purpose: this crate is the part that should be reused (and fuzzed,
//! and property-tested) while the CLI stays a thin shell. The pieces:
//!
//! * [`manifest`] — the wire format (`manifest.json`, schema 3) and its validation rules;
//! * [`payload`] — prebuilt binary payloads as conda packages: validation, budgets, provenance and the
//!   recipe, behind a toolchain seam (TASK-78, ADR-28);
//! * [`payload_config`] — the committed `pixi-sandbox-payload.toml` declaration a `wrap-payload`
//!   run reads (TASK-78, ADR-28);
//! * [`platform`] — the single source of truth for the five supported platform ids and
//!   everything derived from them (target triple, release asset name, GitHub runner);
//! * [`shard`] — the only two file operations in the project: `record_file` (split when
//!   oversized) and `materialise`/`join_parts` (verify, then write, never a half file);
//! * [`files_manifest`] — the per-file oracle for a restored prefix (`envs/<name>/files.json`);
//! * [`build_tool_pins`] — caller-supplied pins for the optional package builder;
//! * [`sandbox_config`] — a reviewed `.pixi-sandbox.toml` publish matrix;
//! * [`transport_budget`] — size measurements and hard refusal thresholds for publishable bundles;
//! * [`verify`] — walk a transport and check every declared byte, and walk a *restored
//!   project* against the per-file oracle; also the ELF linkage check that stops a
//!   dynamically linked tool from being shipped to an airlock.

pub mod build_tool_pins;
pub mod error;
pub mod files_manifest;
pub mod host_requirements;
pub mod manifest;
pub mod payload;
pub mod payload_config;
pub mod platform;
pub mod sandbox_config;
pub mod shard;
pub mod transport_budget;
pub mod vendor_overlap;
pub mod verify;

pub use error::{Error, Result};
