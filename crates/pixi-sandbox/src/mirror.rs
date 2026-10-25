//! TASK-137 spike: one Cargo local registry served from disk, built from the union of
//! several `Cargo.lock` files.
//!
//! # Spike status — do not wire this into product paths yet
//!
//! ADR-6/ADR-33 vendoring gives each lockfile its own source tree, so a crate two projects
//! share is checked out, hashed and verified twice. This module experiments with the
//! alternative proposed for m-2: a single [`local-registry`] in Cargo's native on-disk
//! layout, where every `(name, version)` pair exists once and any number of lockfiles resolve
//! against it. It is deliberately **not** reachable from the CLI, `pack`, `restore` or the
//! manifest schema, and no transport layout changes. The ignored `mirror_offline` test is the
//! only consumer, and ADR-34 records the questions a promoted design must answer first.
//!
//! [`local-registry`]: https://doc.rust-lang.org/cargo/reference/source-replacement.html#local-registry-sources

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use toml::Value;

/// Source name used in the generated `config.toml`; stable so two projects point at one mirror.
const SOURCE_REPLACEMENT_NAME: &str = "pixi-sandbox-mirror";

/// Only registry crates are mirrored; git and path sources have no `.crate` in the cache.
const REGISTRY_SOURCE_PREFIX: &str = "registry+";

/// One `Cargo.lock` that contributes its registry crates to the mirror.
#[derive(Debug, Clone, Copy)]
pub struct MirrorProject<'a> {
    /// Label used in the per-lock report, e.g. `"root"` or `"workspaces/subproject"`.
    pub label: &'a str,
    /// Path to the project's `Cargo.lock`.
    pub cargo_lock: &'a Path,
}

/// What one mirror build produced, for the measurement ledger and test assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorReport {
    /// Number of input lockfiles.
    pub projects: usize,
    /// Registry packages seen across every lockfile, counting repeats per lock.
    pub lock_packages: usize,
    /// Distinct `(name, version)` `.crate` files written into the mirror.
    pub unique_crates: usize,
    /// Lock entries served by a crate an earlier lock already contributed.
    pub duplicated_pairs: usize,
    /// Total bytes of the `.crate` files, each shared copy counted once.
    pub mirror_bytes: u64,
    /// Number of JSON lines written across the registry index.
    pub index_entries: usize,
    /// `(label, registry package count)` per input lockfile, in input order.
    pub per_lock: Vec<(String, usize)>,
}

/// A registry package as a `Cargo.lock` records it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RegistryPackage {
    name: String,
    version: String,
    checksum: String,
}

/// Build a local registry under `out_dir` from the union of every project's lockfile.
///
/// `cargo_home` is the connected-side `CARGO_HOME` in which `cargo fetch` has already
/// downloaded the `.crate` files; `out_dir` must be a fresh directory. The layout is Cargo's
/// documented local-registry shape: `.crate` files at the root and one JSON-line index file
/// per crate name under `index/`. A ready-to-use cargo home carrying only the source
/// replacement `config.toml` is written to [`cargo_home_in`].
///
/// # Errors
///
/// Returns an error if a lockfile cannot be parsed, a locked crate is absent from the cache,
/// a downloaded `.crate` does not match the lock's checksum, an index manifest cannot be read,
/// or the output directory already holds an index.
pub fn build_local_registry(
    projects: &[MirrorProject<'_>],
    cargo_home: &Path,
    out_dir: &Path,
) -> Result<MirrorReport> {
    if projects.is_empty() {
        bail!("a mirror needs at least one Cargo.lock");
    }
    let index_root = out_dir.join("index");
    if index_root.exists() {
        bail!(
            "refusing to append into an existing mirror at {}: build into a fresh directory so \
             index entries are never written twice",
            out_dir.display()
        );
    }
    fs::create_dir_all(&index_root).with_context(|| {
        format!("create mirror index directory {}", index_root.display())
    })?;

    let mut per_lock: Vec<(String, usize)> = Vec::new();
    let mut all_packages: Vec<RegistryPackage> = Vec::new();
    for project in projects {
        let packages = parse_registry_packages(project.cargo_lock)?;
        all_packages.extend(packages.iter().cloned());
        per_lock.push((project.label.to_string(), packages.len()));
    }
    all_packages.sort();

    // One entry per (name, version): a shared crate is copied once no matter how many locks
    // name it. BTreeMap keeps the on-disk order deterministic for fixture assertions.
    let mut unique: BTreeMap<(String, String), RegistryPackage> = BTreeMap::new();
    for package in all_packages {
        let key = (package.name.clone(), package.version.clone());
        let _ = unique.insert(key, package);
    }

    let lock_packages: usize = per_lock.iter().map(|(_, count)| *count).sum();
    // Every unique key was inserted from a lock entry, so the union can never outnumber its
    // sources; a subtraction failure here would mean the dedup logic above is wrong.
    let duplicated_pairs = lock_packages - unique.len();

    let mut mirror_bytes = 0u64;
    let mut index_entries = 0usize;
    for package in unique.values() {
        let cached = find_cached_crate(cargo_home, &package.name, &package.version)?;
        let file_name = format!("{}-{}.crate", package.name, package.version);
        let destination = out_dir.join(&file_name);
        let bytes = copy_and_verify(&cached, &destination, &package.checksum)?;
        mirror_bytes += bytes;

        let manifest = crate_manifest(&destination)?;
        let entry = index_entry(&package, &manifest)?;
        append_index_entry(&index_root, &package.name, &entry)?;
        index_entries += 1;
    }

    let config_home = cargo_home_in(out_dir);
    fs::create_dir_all(&config_home)
        .with_context(|| format!("create mirror cargo home {}", config_home.display()))?;
    fs::write(config_home.join("config.toml"), render_cargo_config(out_dir)?)
        .context("write mirror source-replacement config")?;

    Ok(MirrorReport {
        projects: projects.len(),
        lock_packages,
        unique_crates: unique.len(),
        duplicated_pairs,
        mirror_bytes,
        index_entries,
        per_lock,
    })
}

/// The cargo home the builder writes, ready to be passed as `CARGO_HOME` on the offline host.
#[must_use]
pub fn cargo_home_in(mirror_dir: &Path) -> PathBuf {
    mirror_dir.join("cargo-home")
}

/// Render the source-replacement `config.toml` that redirects crates.io to the mirror.
///
/// # Errors
///
/// Returns an error if `mirror_dir` cannot be canonicalized to an absolute path.
pub fn render_cargo_config(mirror_dir: &Path) -> Result<String> {
    let absolute = mirror_dir
        .canonicalize()
        .with_context(|| format!("mirror directory {} is not on disk", mirror_dir.display()))?;
    Ok(format!(
        "# Generated by the pixi-sandbox local-registry mirror spike (TASK-137, ADR-34).\n\
         [source.crates-io]\n\
         replace-with = \"{SOURCE_REPLACEMENT_NAME}\"\n\n\
         [source.{SOURCE_REPLACEMENT_NAME}]\n\
         local-registry = \"{}\"\n",
        absolute.display()
    ))
}

/// Cargo's registry index path rules: length-based sharding so index lookups stay local.
#[must_use]
pub fn index_relative_path(name: &str) -> String {
    let normalized = name.to_ascii_lowercase();
    let chars: Vec<char> = normalized.chars().collect();
    match chars.len() {
        1 => format!("1/{normalized}"),
        2 => format!("2/{normalized}"),
        3 => format!("3/{}/{normalized}", chars[0]),
        // Four or more characters: two two-character shards, then the full crate name.
        _ => format!(
            "{}{}/{}{}/{normalized}",
            chars[0], chars[1], chars[2], chars[3]
        ),
    }
}

/// Parse the registry packages out of one `Cargo.lock`, skipping path/workspace members.
fn parse_registry_packages(lock_path: &Path) -> Result<Vec<RegistryPackage>> {
    let text = fs::read_to_string(lock_path)
        .with_context(|| format!("read Cargo.lock {}", lock_path.display()))?;
    let document: Value = toml::from_str(&text)
        .with_context(|| format!("parse Cargo.lock {}", lock_path.display()))?;
    let Some(packages) = document.get("package").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    let mut parsed = Vec::new();
    for package in packages {
        // Entries without a source are path/workspace members: nothing to mirror.
        let Some(source) = package.get("source").and_then(Value::as_str) else {
            continue;
        };
        if !source.starts_with(REGISTRY_SOURCE_PREFIX) {
            bail!(
                "{} records a non-registry source {source:?}; the spike mirrors registry crates \
                 only",
                lock_path.display()
            );
        }
        let name = package
            .get("name")
            .and_then(Value::as_str)
            .context("lock package is missing its name")?;
        let version = package
            .get("version")
            .and_then(Value::as_str)
            .context("lock package is missing its version")?;
        let checksum = package
            .get("checksum")
            .and_then(Value::as_str)
            .context("registry package is missing its checksum")?;
        parsed.push(RegistryPackage {
            name: name.to_string(),
            version: version.to_string(),
            checksum: checksum.to_string(),
        });
    }
    Ok(parsed)
}

/// Locate `name-version.crate` in `CARGO_HOME/registry/cache/<index-dir>/`.
fn find_cached_crate(cargo_home: &Path, name: &str, version: &str) -> Result<PathBuf> {
    let cache_root = cargo_home.join("registry").join("cache");
    let file_name = format!("{name}-{version}.crate");
    let index_dirs = fs::read_dir(&cache_root).with_context(|| {
        format!(
            "read cargo registry cache {} — run `cargo fetch` with this CARGO_HOME first",
            cache_root.display()
        )
    })?;
    for entry in index_dirs {
        let candidate = entry?.path().join(&file_name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!(
        "{file_name} is not under {}: the connected-side fetch did not download this locked crate",
        cache_root.display()
    );
}

/// Copy a `.crate` into the mirror while hashing it, refusing anything the lock does not attest.
fn copy_and_verify(source: &Path, destination: &Path, expected_checksum: &str) -> Result<u64> {
    let mut input = File::open(source)
        .with_context(|| format!("open cached crate {}", source.display()))?;
    let mut output = File::create(destination)
        .with_context(|| format!("create mirror crate {}", destination.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = input
            .read(&mut buffer)
            .with_context(|| format!("read cached crate {}", source.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .with_context(|| format!("write mirror crate {}", destination.display()))?;
        total += read as u64;
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected_checksum {
        let _ = fs::remove_file(destination);
        bail!(
            "checksum mismatch for {}: the Cargo.lock attests {expected_checksum} but the cached \
             .crate hashes to {actual}",
            destination.display()
        );
    }
    Ok(total)
}

/// Read `name-version/Cargo.toml` from a gzip-compressed crate tarball.
fn crate_manifest(crate_path: &Path) -> Result<Value> {
    let file = File::open(crate_path)
        .with_context(|| format!("open crate tarball {}", crate_path.display()))?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path.file_name().is_some_and(|name| name == "Cargo.toml") {
            let mut text = String::new();
            entry.read_to_string(&mut text).with_context(|| {
                format!("read Cargo.toml inside {}", crate_path.display())
            })?;
            return toml::from_str(&text).with_context(|| {
                format!("parse Cargo.toml inside {}", crate_path.display())
            });
        }
    }
    bail!("{} contains no Cargo.toml", crate_path.display())
}

/// Build one registry index JSON line for a crate version.
fn index_entry(package: &RegistryPackage, manifest: &Value) -> Result<Json> {
    Ok(json!({
        "name": package.name,
        "vers": package.version,
        "deps": collect_deps(manifest),
        "cksum": package.checksum,
        "features": manifest
            .get("features")
            .and_then(Value::as_table)
            .and_then(|table| serde_json::to_value(table).ok())
            .unwrap_or_else(|| json!({})),
        "yanked": false,
    }))
}

/// Collect normal, dev and build dependencies, including target-specific tables.
fn collect_deps(manifest: &Value) -> Vec<Json> {
    let mut deps = Vec::new();
    for (table, kind) in [
        ("dependencies", None),
        ("dev-dependencies", Some("dev")),
        ("build-dependencies", Some("build")),
    ] {
        if let Some(table_value) = manifest.get(table).and_then(Value::as_table) {
            for (alias, value) in table_value {
                if let Some(object) = dep_object(alias, value, kind, None) {
                    deps.push(object);
                }
            }
        }
    }
    if let Some(targets) = manifest.get("target").and_then(Value::as_table) {
        for (target, target_value) in targets {
            for (table, kind) in [
                ("dependencies", None),
                ("dev-dependencies", Some("dev")),
                ("build-dependencies", Some("build")),
            ] {
                if let Some(table_value) = target_value.get(table).and_then(Value::as_table) {
                    for (alias, value) in table_value {
                        if let Some(object) = dep_object(alias, value, kind, Some(target.as_str()))
                        {
                            deps.push(object);
                        }
                    }
                }
            }
        }
    }
    deps
}

/// Convert one manifest dependency entry into the registry index dep shape.
///
/// Returns `None` for path and git dependencies: the spike mirrors registry crates only, and
/// those sources have no presence in a local registry.
fn dep_object(
    alias: &str,
    value: &Value,
    kind: Option<&str>,
    target: Option<&str>,
) -> Option<Json> {
    match value {
        Value::String(requirement) => Some(json!({
            "name": alias,
            "req": requirement,
            "features": [],
            "optional": false,
            "default_features": true,
            "target": target,
            "kind": kind,
        })),
        Value::Table(table) => {
            if table.contains_key("path") || table.contains_key("git") {
                return None;
            }
            let name = table
                .get("package")
                .and_then(Value::as_str)
                .unwrap_or(alias);
            let requirement = table
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("*");
            let features = table
                .get("features")
                .and_then(Value::as_array)
                .and_then(|array| serde_json::to_value(array).ok())
                .unwrap_or_else(|| json!([]));
            let optional = table
                .get("optional")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let default_features = table
                .get("default-features")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            Some(json!({
                "name": name,
                "req": requirement,
                "features": features,
                "optional": optional,
                "default_features": default_features,
                "target": target,
                "kind": kind,
            }))
        }
        // Unknown dependency shapes carry no index representation.
        _ => None,
    }
}

/// Append one JSON line to the index file that owns `crate_name`.
fn append_index_entry(index_root: &Path, crate_name: &str, entry: &Json) -> Result<()> {
    let index_path = index_root.join(index_relative_path(crate_name));
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create index shard {}", parent.display()))?;
    }
    let mut line = serde_json::to_string(entry).context("serialize index entry")?;
    line.push('\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&index_path)
        .with_context(|| format!("open index file {}", index_path.display()))?;
    file.write_all(line.as_bytes())
        .with_context(|| format!("append index entry {}", index_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use sha2::{Digest, Sha256};
    use tempfile::tempdir;

    #[test]
    fn index_paths_follow_cargos_length_sharding() {
        assert_eq!(index_relative_path("a"), "1/a");
        assert_eq!(index_relative_path("ab"), "2/ab");
        assert_eq!(index_relative_path("abc"), "3/a/abc");
        assert_eq!(index_relative_path("abcd"), "ab/cd/abcd");
        assert_eq!(index_relative_path("walkdir"), "wa/lk/walkdir");
        // Registry names are normalized to lowercase for the shard path.
        assert_eq!(index_relative_path("MyCrate"), "my/cr/mycrate");
    }

    #[test]
    fn two_lockfiles_share_one_copy_of_each_crate() {
        let scratch = tempdir().expect("test tempdir");
        let cache_dir = scratch
            .path()
            .join("home/registry/cache/index.crates.io-deadbeef");
        fs::create_dir_all(&cache_dir).expect("cache dir");

        let alpha_manifest = "[package]\nname = \"alpha\"\nversion = \"1.0.0\"\n\n\
                              [dependencies]\nbeta = \"1.0\"\n";
        let beta_manifest = "[package]\nname = \"beta\"\nversion = \"1.2.0\"\n";
        write_crate(&cache_dir, "alpha", "1.0.0", alpha_manifest);
        write_crate(&cache_dir, "beta", "1.2.0", beta_manifest);
        let beta_checksum = checksum_of(&cache_dir.join("beta-1.2.0.crate"));
        let alpha_checksum = checksum_of(&cache_dir.join("alpha-1.0.0.crate"));

        let root_lock = scratch.path().join("root-Cargo.lock");
        fs::write(
            &root_lock,
            format!(
                "version = 3\n\n\
                 [[package]]\nname = \"alpha\"\nversion = \"1.0.0\"\n\
                 source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
                 checksum = \"{alpha_checksum}\"\n\
                 dependencies = [\"beta 1.2.0 (registry+https://github.com/rust-lang/crates.io-index)\"]\n\n\
                 [[package]]\nname = \"beta\"\nversion = \"1.2.0\"\n\
                 source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
                 checksum = \"{beta_checksum}\"\n\n\
                 [[package]]\nname = \"local-member\"\nversion = \"0.1.0\"\n"
            ),
        )
        .expect("write root lock");
        let sub_lock = scratch.path().join("sub-Cargo.lock");
        fs::write(
            &sub_lock,
            format!(
                "version = 3\n\n[[package]]\nname = \"beta\"\nversion = \"1.2.0\"\n\
                 source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
                 checksum = \"{beta_checksum}\"\n"
            ),
        )
        .expect("write sub lock");

        let mirror_dir = scratch.path().join("mirror");
        let report = build_local_registry(
            &[
                MirrorProject {
                    label: "root",
                    cargo_lock: &root_lock,
                },
                MirrorProject {
                    label: "sub",
                    cargo_lock: &sub_lock,
                },
            ],
            &scratch.path().join("home"),
            &mirror_dir,
        )
        .expect("build the mirror");

        assert_eq!(report.projects, 2);
        assert_eq!(report.lock_packages, 3);
        assert_eq!(report.unique_crates, 2);
        assert_eq!(report.duplicated_pairs, 1);
        assert_eq!(report.index_entries, 2);
        assert_eq!(
            report.per_lock,
            vec![("root".to_string(), 2), ("sub".to_string(), 1)]
        );
        assert!(mirror_dir.join("alpha-1.0.0.crate").is_file());
        assert!(mirror_dir.join("beta-1.2.0.crate").is_file());

        let alpha_index =
            fs::read_to_string(mirror_dir.join("index/al/ph/alpha")).expect("alpha index");
        let line: Json = serde_json::from_str(alpha_index.trim()).expect("index line is JSON");
        assert_eq!(line["cksum"].as_str(), Some(alpha_checksum.as_str()));
        assert_eq!(line["deps"][0]["name"], json!("beta"));
        assert_eq!(line["deps"][0]["req"], json!("1.0"));
        assert_eq!(line["deps"][0]["kind"], Json::Null);
        assert!(mirror_dir.join("index/be/ta/beta").is_file());

        let config = fs::read_to_string(cargo_home_in(&mirror_dir).join("config.toml"))
            .expect("mirror config");
        assert!(config.contains("replace-with = \"pixi-sandbox-mirror\""));
        assert!(config.contains("local-registry ="));
    }

    #[test]
    fn a_crate_that_disagrees_with_its_lock_checksum_is_refused() {
        let scratch = tempdir().expect("test tempdir");
        let cache_dir = scratch
            .path()
            .join("home/registry/cache/index.crates.io-deadbeef");
        fs::create_dir_all(&cache_dir).expect("cache dir");
        write_crate(&cache_dir, "beta", "1.2.0", "[package]\nname = \"beta\"\n");
        let lock = scratch.path().join("Cargo.lock");
        fs::write(
            &lock,
            "version = 3\n\n[[package]]\nname = \"beta\"\nversion = \"1.2.0\"\n\
             source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
             checksum = \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
        )
        .expect("write lock");

        let mirror_dir = scratch.path().join("mirror");
        let error = build_local_registry(
            &[MirrorProject {
                label: "root",
                cargo_lock: &lock,
            }],
            &scratch.path().join("home"),
            &mirror_dir,
        )
        .expect_err("a wrong checksum must fail the build");
        assert!(format!("{error:#}").contains("checksum mismatch"));
        assert!(
            !mirror_dir.join("beta-1.2.0.crate").exists(),
            "the unverified crate is removed, not left behind"
        );
    }

    /// Write a minimal gzip `.crate` tarball holding one `Cargo.toml`.
    fn write_crate(cache_dir: &Path, name: &str, version: &str, manifest: &str) {
        let encoder = GzEncoder::new(Vec::new(), Compression::fast());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        let path = format!("{name}-{version}/Cargo.toml");
        header.set_path(&path).expect("tar path");
        header.set_size(manifest.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, &path, manifest.as_bytes())
            .expect("append manifest");
        let bytes = builder
            .into_inner()
            .expect("finish tar")
            .finish()
            .expect("finish gzip");
        fs::write(cache_dir.join(format!("{name}-{version}.crate")), bytes).expect("write crate");
    }

    fn checksum_of(path: &Path) -> String {
        let mut hasher = Sha256::new();
        hasher.update(fs::read(path).expect("read crate"));
        format!("{:x}", hasher.finalize())
    }
}
