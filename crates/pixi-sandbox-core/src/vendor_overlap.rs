//! Shared-crate accounting across per-workspace vendor trees (TASK-131 AC#4).
//!
//! Two lockfiles depending on the same crate+version produce two identical
//! `<name>-<version>` directories under two transport paths because each tree is produced by
//! its own `cargo vendor`. The manifest therefore records two blob lists and two sizes: branch
//! *payload* bytes are counted once per tree. Git's content-addressed object store keeps only
//! one underlying object, so actual branch *storage* is smaller — but checkout and
//! verification still read and hash both copies. This module makes that overlap explicit
//! instead of letting a reviewer assume the summed payload is all unique bytes.

use crate::manifest::{Manifest, Vendor};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// A crate's content fingerprint: every relative path with its sha256, plus the crate's total
/// recorded bytes.
type Fingerprint = (BTreeSet<(String, String)>, u64);

/// The overlap between the vendor trees one transport carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VendorOverlap {
    /// Number of vendor trees compared (the root tree plus every workspace tree).
    pub trees: usize,
    /// Distinct crate directories present, byte-identical, in at least two trees.
    pub shared_crates: usize,
    /// Single-copy bytes of the shared crates — what content-addressed Git storage keeps
    /// once, while the payload sums one copy per tree.
    pub shared_bytes: u64,
}

/// Analyse the manifest's root and per-workspace vendor trees for identical crates.
///
/// Returns `None` when fewer than two trees exist or no crate is shared, so callers can skip
/// the paragraph rather than report an uninteresting zero. A crate is shared only when its
/// directory name *and* every file's relative path with its sha256 match, which keeps a
/// same-named directory with different contents from being counted as overlap.
#[must_use]
pub fn vendor_overlap(manifest: &Manifest) -> Option<VendorOverlap> {
    let mut trees: Vec<&Vendor> = Vec::new();
    if let Some(vendor) = &manifest.vendor {
        trees.push(vendor);
    }
    trees.extend(manifest.workspace_vendors.iter().map(|tree| &tree.vendor));
    if trees.len() < 2 {
        return None;
    }

    let tree_count = trees.len();
    // crate name -> fingerprints seen in earlier trees -> single-copy byte size.
    let mut seen: BTreeMap<String, BTreeMap<BTreeSet<(String, String)>, u64>> = BTreeMap::new();
    // crate name -> single-copy bytes, recorded once even when three trees match.
    let mut shared: BTreeMap<String, u64> = BTreeMap::new();
    for vendor in trees {
        for (crate_name, (fingerprint, bytes)) in tree_fingerprints(vendor) {
            let fingerprints = seen.entry(crate_name.clone()).or_default();
            match fingerprints.entry(fingerprint) {
                std::collections::btree_map::Entry::Occupied(_) => {
                    shared.entry(crate_name).or_insert(bytes);
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(bytes);
                }
            }
        }
    }

    if shared.is_empty() {
        return None;
    }
    Some(VendorOverlap {
        trees: tree_count,
        shared_crates: shared.len(),
        shared_bytes: shared.into_values().fold(0u64, u64::saturating_add),
    })
}

/// One tree's crates as `(crate directory, fingerprint with relative paths and digests, bytes)`.
fn tree_fingerprints(vendor: &Vendor) -> Vec<(String, Fingerprint)> {
    let mut crates: BTreeMap<String, Fingerprint> = BTreeMap::new();
    for blob in &vendor.blobs {
        let Some((crate_name, relative)) = crate_parts(&blob.path) else {
            continue;
        };
        let entry = crates.entry(crate_name).or_default();
        entry.0.insert((relative, blob.sha256.clone()));
        entry.1 = entry.1.saturating_add(blob.size);
    }
    crates.into_iter().collect()
}

/// Split a transport blob path into its crate directory and the path inside that crate.
///
/// Root blobs look like `vendor/<crate>/...` (or a `vendor/<crate>.tar` archive); workspace
/// blobs add the `workspaces/<slug>/` qualifier. The split is path-based rather than reading
/// the optional `Vendor.directory`, so a manifest serialised without it still analyses
/// correctly; [`Manifest::validate`](crate::manifest::Manifest::validate) already confines
/// every blob to the tree it belongs to.
fn crate_parts(blob_path: &str) -> Option<(String, String)> {
    let relative = blob_path.strip_prefix("vendor/")?;
    let components = relative.split('/').collect::<Vec<_>>();
    let (crate_name, inside) = match components.as_slice() {
        // vendor/workspaces/<slug>/<crate>/... — four or more components.
        ["workspaces", _slug, name, rest @ ..] => (name.to_string(), rest.join("/")),
        // vendor/<crate>/... or a vendor/<crate>.tar archive at the tree root.
        [name, rest @ ..] => (name.to_string(), rest.join("/")),
        [] => return None,
    };
    if crate_name.is_empty() {
        return None;
    }
    Some((crate_name, inside))
}
