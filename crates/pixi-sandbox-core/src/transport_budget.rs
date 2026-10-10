//! Transport size budgets for publishable sandbox branches.
//!
//! A transport is still self-contained — budgets refuse an over-large branch before publish;
//! they do not introduce external payloads or partial-environment restores. The partition
//! remedy is another reviewed `[[bundle]]` environment set, producing another complete branch.

use crate::manifest::{Blob, Manifest};
use serde::Serialize;

/// Bytes in one mebibyte.
pub const MIB: u64 = 1024 * 1024;
/// `MIB` in f64 — 2^20 is exact, so the budget conversions below lose nothing to the
/// float domain itself.
const MIB_F64: f64 = 1024.0 * 1024.0;

/// GitHub's hard blob limit is 100 MiB; the default leaves the same margin the packer already
/// uses for sharding.
pub const DEFAULT_MAX_BLOB_MIB: f64 = 95.0;
/// A self-contained transport above 2 GiB is where the current measurements say a consumer
/// should split environment sets before GitHub push/repository limits become the incident.
pub const DEFAULT_MAX_TRANSPORT_MIB: f64 = 2048.0;
/// The publisher stages exactly this many file bytes into the orphan snapshot before Git
/// compression; keep the same default as `max_transport_mib` so root docs/manifest overhead does
/// not silently turn a green transport into a giant push.
pub const DEFAULT_MAX_REPOSITORY_PUSH_MIB: f64 = 2048.0;
/// Restore preflight asks for packed + unpacked env bytes plus two vendor copies. Eight GiB is a
/// conservative default for a developer/CI airlock while still catching runaway bundles.
pub const DEFAULT_MAX_RESTORE_REQUIRED_MIB: f64 = 8192.0;

/// Hard thresholds, already converted to bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportBudgets {
    pub max_blob_bytes: u64,
    pub max_transport_bytes: u64,
    pub max_repository_push_bytes: u64,
    pub max_restore_required_bytes: u64,
}

impl Default for TransportBudgets {
    fn default() -> Self {
        Self {
            max_blob_bytes: mib_to_bytes(DEFAULT_MAX_BLOB_MIB),
            max_transport_bytes: mib_to_bytes(DEFAULT_MAX_TRANSPORT_MIB),
            max_repository_push_bytes: mib_to_bytes(DEFAULT_MAX_REPOSITORY_PUSH_MIB),
            max_restore_required_bytes: mib_to_bytes(DEFAULT_MAX_RESTORE_REQUIRED_MIB),
        }
    }
}

/// Transport sizes that are cheap to know from the manifest and checkout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TransportMeasurements {
    /// Largest stored Git object declared by the manifest. Split blobs count by part; unsplit
    /// blobs and embedded tools count by their own file size.
    pub largest_blob_bytes: u64,
    /// Declared bytes verified by `doctor --verify`: env packs, per-env file oracles, tools,
    /// and vendored Cargo blobs.
    pub transport_bytes: u64,
    /// Bytes the publisher will stage into the orphan snapshot, when the caller has a checkout
    /// to measure (doctor does; a manifest alone does not).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_push_bytes: Option<u64>,
    /// Same estimate `restore` preflights: packed + unpacked envs, plus two copies of vendor.
    pub restore_required_bytes: u64,
}

/// One refused budget, with the config field name and an operator-facing remedy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BudgetViolation {
    pub field: &'static str,
    pub measured_bytes: u64,
    pub limit_bytes: u64,
    pub remedy: &'static str,
}

/// Full budget verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BudgetReport {
    pub measurements: TransportMeasurements,
    pub budgets: SerializableBudgets,
    pub violations: Vec<BudgetViolation>,
}

/// JSON-friendly copy of [`TransportBudgets`] with field names matching `.pixi-sandbox.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SerializableBudgets {
    pub max_blob_bytes: u64,
    pub max_transport_bytes: u64,
    pub max_repository_push_bytes: u64,
    pub max_restore_required_bytes: u64,
}

impl From<TransportBudgets> for SerializableBudgets {
    fn from(value: TransportBudgets) -> Self {
        Self {
            max_blob_bytes: value.max_blob_bytes,
            max_transport_bytes: value.max_transport_bytes,
            max_repository_push_bytes: value.max_repository_push_bytes,
            max_restore_required_bytes: value.max_restore_required_bytes,
        }
    }
}

impl BudgetReport {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Measure one manifest, with optional publisher snapshot bytes supplied by the caller.
#[must_use]
pub fn measure(manifest: &Manifest, repository_push_bytes: Option<u64>) -> TransportMeasurements {
    let mut largest_blob_bytes = 0u64;
    let mut transport_bytes = 0u64;

    for (_, blob) in manifest.blobs(None) {
        transport_bytes = transport_bytes.saturating_add(blob.size);
        largest_blob_bytes = largest_blob_bytes.max(largest_stored_object(blob));
    }

    for env in manifest.envs.values() {
        if let Some(files) = &env.files {
            transport_bytes = transport_bytes.saturating_add(files.blob.size);
            largest_blob_bytes = largest_blob_bytes.max(largest_stored_object(&files.blob));
        }
    }

    for tool in manifest.tools.values() {
        transport_bytes = transport_bytes.saturating_add(tool.size_bytes);
        largest_blob_bytes = largest_blob_bytes.max(tool.size_bytes);
    }

    let restore_required_bytes = restore_required_bytes(manifest);

    TransportMeasurements {
        largest_blob_bytes,
        transport_bytes,
        repository_push_bytes,
        restore_required_bytes,
    }
}

/// Measure and compare against hard thresholds.
#[must_use]
pub fn check(
    manifest: &Manifest,
    repository_push_bytes: Option<u64>,
    budgets: TransportBudgets,
) -> BudgetReport {
    let measurements = measure(manifest, repository_push_bytes);
    let mut violations = Vec::new();

    push_violation(
        &mut violations,
        "budgets.max_blob_mib",
        measurements.largest_blob_bytes,
        budgets.max_blob_bytes,
        "reduce --shard-limit-mib below the budget or remove the single oversized payload; environment-set partitioning cannot split one file",
    );
    push_violation(
        &mut violations,
        "budgets.max_transport_mib",
        measurements.transport_bytes,
        budgets.max_transport_bytes,
        PARTITION_REMEDY,
    );
    if let Some(bytes) = measurements.repository_push_bytes {
        push_violation(
            &mut violations,
            "budgets.max_repository_push_mib",
            bytes,
            budgets.max_repository_push_bytes,
            PARTITION_REMEDY,
        );
    }
    push_violation(
        &mut violations,
        "budgets.max_restore_required_mib",
        measurements.restore_required_bytes,
        budgets.max_restore_required_bytes,
        PARTITION_REMEDY,
    );

    BudgetReport {
        measurements,
        budgets: budgets.into(),
        violations,
    }
}

const PARTITION_REMEDY: &str = "partition the reviewed environment set into separate [[bundle]] entries so each self-contained branch stays under budget; do not split an individual environment or move payloads outside the transport";

fn push_violation(
    violations: &mut Vec<BudgetViolation>,
    field: &'static str,
    measured_bytes: u64,
    limit_bytes: u64,
    remedy: &'static str,
) {
    if measured_bytes > limit_bytes {
        violations.push(BudgetViolation {
            field,
            measured_bytes,
            limit_bytes,
            remedy,
        });
    }
}

fn largest_stored_object(blob: &Blob) -> u64 {
    blob.parts
        .iter()
        .map(|part| part.size)
        .max()
        .unwrap_or(blob.size)
}

fn restore_required_bytes(manifest: &Manifest) -> u64 {
    let envs = manifest.envs.values().fold(0u64, |total, env| {
        total
            .saturating_add(env.packed_size_bytes)
            .saturating_add(env.unpacked_size_bytes)
    });
    let vendor = manifest
        .vendor
        .as_ref()
        .map_or(0, |vendor| vendor.size_bytes.saturating_mul(2));
    envs.saturating_add(vendor)
}

/// MiB (possibly fractional) to bytes. Budget inputs are human-edited, so the
/// conversion saturates instead of wrapping: Rust's f64->u64 `as` cast saturates
/// (NaN and negatives become 0, anything above `u64::MAX` becomes `u64::MAX`) — a
/// silent wrap would under-report a limit. The bounds are pinned by
/// `mib_to_bytes_converts_exactly_and_saturates_at_the_bounds`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[must_use]
pub fn mib_to_bytes(value: f64) -> u64 {
    (value * MIB_F64).round() as u64
}

/// Bytes to MiB for human-readable reports. The u64->f64 cast can lose the low bits of
/// huge byte counts; that is inherent to a display conversion and harmless here.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn bytes_to_mib(value: u64) -> f64 {
    value as f64 / MIB_F64
}
