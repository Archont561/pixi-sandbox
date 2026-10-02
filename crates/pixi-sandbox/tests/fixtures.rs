//! The fixture policy, as tests.
//!
//! The rule (tests/fixtures/README.md, design.md §12): project-level tests use the fixture
//! project, never this repository. The repository *is* this tool's development environment —
//! its `dev` environment contains `pixi-pack`, its lockfile resolves hundreds of MiB — so a
//! test pointed at it silently passes on a developer's machine and fails (or never runs) on a
//! clean one. These tests make the rule executable instead of aspirational.

use pixi_sandbox_core::manifest::Manifest;
use pixi_sandbox_core::verify;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The complete pixi project definition that tests sandbox (never the repository root).
fn demo_project() -> PathBuf {
    crate_dir().join("tests/fixtures/demo-project")
}

/// A synthetic, already-packed payload: no packer, no pixi, no network needed.
fn transport() -> PathBuf {
    crate_dir().join("tests/fixtures/transport")
}

#[test]
fn the_demo_project_is_a_complete_pixi_project() {
    let project = demo_project();
    for required in [
        "pixi.toml",
        "pixi.lock",
        "Cargo.toml",
        "Cargo.lock",
        "src/main.rs",
    ] {
        assert!(
            project.join(required).is_file(),
            "{required} is missing from the fixture project"
        );
    }

    let manifest = std::fs::read_to_string(project.join("pixi.toml")).unwrap();
    assert!(
        manifest.contains("[workspace]"),
        "a pixi manifest needs a workspace table"
    );
    assert!(manifest.contains("name = \"demo-project\""));
    assert!(
        manifest.contains("[dependencies]"),
        "the fixture must have a real environment"
    );

    // A lockfile with packages in it, so `pack`/`vendor`/hash steps have something to chew on.
    let lock = std::fs::read_to_string(project.join("pixi.lock")).unwrap();
    assert!(
        lock.contains("conda:"),
        "the fixture lockfile must resolve packages"
    );
    assert_eq!(
        lock.matches("conda:").count(),
        lock.matches("conda:").count(),
        "sanity"
    );

    // The crate is its own cargo workspace: it must never be a member of ours.
    let cargo = std::fs::read_to_string(project.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("[workspace]"),
        "the fixture crate must declare its own [workspace] root"
    );
}

#[test]
fn the_demo_project_does_not_depend_on_the_packers() {
    // This is the whole point: if the fixture pulled in pixi-pack/pixi-unpack, every test
    // using it would depend on the developer's environment again.
    for file in ["pixi.toml", "pixi.lock"] {
        let text = std::fs::read_to_string(demo_project().join(file)).unwrap();
        // Comments are documentation, dependencies are the contract: only the latter count.
        // (The fixture's pixi.toml explains *why* the packers are absent, at length.)
        let dependencies: String = text
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in ["pixi-pack", "pixi-unpack"] {
            let text = &dependencies;
            assert!(
                !text.contains(forbidden),
                "{file} depends on {forbidden}: the fixture must be a plain project, and a test \
                 that needs a packer must ask for one explicitly (tests/fixtures/README.md)"
            );
        }
    }
}

#[test]
fn the_duplicate_source_project_is_a_plain_project_with_the_collision_it_exists_for() {
    // A second project fixture, for the pack-time duplicate-crate-source check (design.md §11,
    // backlog task-8). It carries the same policy as `demo-project`: a plain pixi project with
    // no packer, so using it cannot depend on the developer's environment.
    let project = crate_dir().join("tests/fixtures/duplicate-source-project");
    for required in [
        "pixi.toml",
        "pixi.lock",
        "Cargo.toml",
        "Cargo.lock",
        "src/main.rs",
    ] {
        assert!(
            project.join(required).is_file(),
            "{required} is missing from the duplicate-source fixture"
        );
    }
    for file in ["pixi.toml", "pixi.lock"] {
        let text = std::fs::read_to_string(project.join(file)).unwrap();
        for forbidden in ["pixi-pack", "pixi-unpack"] {
            assert!(
                !text.contains(forbidden),
                "{file} depends on {forbidden}: every project fixture must be a plain project \
                 (tests/fixtures/README.md)"
            );
        }
    }

    // The lockfile must still contain the collision, or the test that uses it would be
    // asserting nothing. `toml` is a dependency of the CLI crate, not this test binary, so
    // the check is deliberately textual rather than a parse.
    let lock = std::fs::read_to_string(project.join("Cargo.lock")).unwrap();
    let itoa = lock
        .match_indices("name = \"itoa\"")
        .map(|(at, _)| &lock[at..at + 200.min(lock.len() - at)])
        .collect::<Vec<_>>();
    assert_eq!(
        itoa.len(),
        2,
        "the fixture must declare itoa twice, once per source: {lock}"
    );
    assert!(
        itoa.iter().any(|entry| entry.contains("crates.io-index")),
        "one itoa must come from the registry: {lock}"
    );
    assert!(
        itoa.iter().any(|entry| entry.contains("git+")),
        "one itoa must come from a git source: {lock}"
    );
    // A `source`-less entry (a path member) must be present too, so the check is exercised
    // against a lockfile that is not uniformly shaped.
    assert!(
        lock.contains("name = \"local-helper\""),
        "the fixture should include a path member with no source: {lock}"
    );
}

#[test]
fn the_fixture_transport_verifies_and_covers_the_split_case() {
    let dir = transport();
    let manifest = Manifest::load(&Manifest::path_in(&dir)).expect("fixture manifest must parse");
    manifest.validate().expect("fixture manifest must validate");

    // The fixture is only useful if it is honest: the payload the tests measure is the payload
    // the manifest describes, down to the split parts.
    let report = verify::verify(&manifest, &dir, None);
    assert!(report.ok(), "fixture must verify: {:?}", report.failures);
    assert_eq!(
        report.files, 12,
        "9 env+vendor blobs, 3 tools, and the files.json oracle"
    );
    let demo = &manifest.envs["demo"];
    assert_eq!(
        demo.files.as_ref().unwrap().entries,
        14,
        "the fixture's per-file oracle: 13 regular files (2 conda-meta records presence-only) + the libz.so symlink"
    );
    assert!(
        manifest
            .envs
            .values()
            .flat_map(|env| env.blobs.iter())
            .any(|blob| !blob.parts.is_empty()),
        "the fixture must contain a split blob, or the `.partNNN` path is never covered"
    );
    assert!(
        manifest
            .envs
            .values()
            .flat_map(|env| env.blobs.iter())
            .any(|blob| blob.path.ends_with("prefix/prefix.tar.gz")),
        "the fixture must carry a real conda prefix, or prefix relocation is never covered"
    );
    assert!(
        manifest
            .vendor
            .as_ref()
            .is_some_and(|v| !v.blobs.is_empty()),
        "the fixture must carry vendored crates, so vendor verification is covered"
    );
}

#[test]
fn the_fixture_transport_uses_the_documentation_only_root_layout() {
    let mut root_files = std::fs::read_dir(transport())
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_type().unwrap().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    root_files.sort();
    assert_eq!(root_files, ["AGENTS.md", "README.md"]);
}

#[test]
fn the_fixture_transport_is_small_enough_to_commit() {
    // A fixture that grows into a real payload would defeat the point (it is meant to be a
    // ~95 KB stand-in for a 262 MB transport, real conda prefix and all). Keep it under a
    // quarter of a MiB.
    let bytes: u64 = walk(&transport()).iter().map(|(_, size)| size).sum();
    assert!(
        bytes < 256 * 1024,
        "fixture transport is {bytes} bytes — regenerate it smaller, or it stops being a fixture"
    );
}

/// The oracle the fixture carries must describe the tarball the fixture carries. This test
/// re-derives every digest from `prefix.tar.gz` with the same canonicalisation pack uses —
/// with `@PREFIX@` standing in for the staging path the fixture's unpacker substitutes — so a
/// tarball edited without regenerating `files.json` fails here, not in a downstream airlock.
#[test]
#[cfg(unix)]
fn the_fixtures_file_list_is_derivable_from_its_tarball() {
    use pixi_sandbox_core::files_manifest::{self, FilesDoc};
    use std::process::Command as StdCommand;

    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("prefix");
    std::fs::create_dir_all(&prefix).unwrap();
    let status = StdCommand::new("tar")
        .arg("-xzf")
        .arg(transport().join(".pixi-sandbox/envs/demo/pack/prefix/prefix.tar.gz"))
        .arg("-C")
        .arg(&prefix)
        .arg("--strip-components=1")
        .status()
        .unwrap();
    assert!(
        status.success(),
        "system tar must unpack the fixture tarball"
    );

    // The fixture's placeholder plays the role the staging path plays in a real pack, which
    // is exactly how files.json was generated (D13): both sides canonicalise their own path
    // spelling to the sentinel, so the digests must agree.
    let (derived, _) = files_manifest::scan_prefix(&prefix, &[b"@PREFIX@".to_vec()]).unwrap();
    let listed = FilesDoc::parse(
        &std::fs::read(transport().join(".pixi-sandbox/envs/demo/files.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(derived.entries(), listed.entries());
    let by_path: std::collections::BTreeMap<&str, _> =
        listed.files.iter().map(|e| (e.p.as_str(), e)).collect();
    for entry in &derived.files {
        let recorded = by_path.get(entry.p.as_str()).unwrap_or_else(|| {
            panic!(
                "{}: listed by the scan but missing from files.json",
                entry.p
            )
        });
        assert_eq!(
            entry.h, recorded.h,
            "{}: digest re-derived from the tarball must equal the recorded one",
            entry.p
        );
        assert_eq!(entry.x, recorded.x, "{}: executable bit", entry.p);
        assert_eq!(entry.l, recorded.l, "{}: symlink target", entry.p);
    }
}

/// The rule that matters is narrower than "always mention fixtures", and sharper: locating a
/// path from `CARGO_MANIFEST_DIR` and then walking **up** (`".."`, `.parent()`) is how a test
/// reaches this repository instead of the fixture — that is forbidden, and this is the test
/// that keeps it forbidden as the suite grows.
#[test]
fn no_test_targets_the_repository_root() {
    let tests_dir = crate_dir().join("tests");
    let mut checked = 0;
    let mut sources = Vec::new();
    collect_rust(&tests_dir, &mut sources);
    assert!(
        !sources.is_empty(),
        "no test sources found under {tests_dir:?}"
    );

    for source in sources {
        let text = std::fs::read_to_string(&source).unwrap();
        for statement in text.split(';') {
            if !statement.contains("CARGO_MANIFEST_DIR") {
                continue;
            }
            if statement.contains("fixtures") {
                checked += 1;
                continue;
            }
            // The one sanctioned exception: scripts/restore.sh is the repository's own airlock
            // bootstrap, which no test executed until `tests/restore_script.rs`. A copy would
            // prove nothing about the script a developer runs verbatim.
            //
            // This exception still never lets a test touch the repository as a *project*: it
            // executes the script against tempdirs and the transport fixture, never this tree.
            if statement.contains("restore.sh") {
                continue;
            }
            let escapes = statement.contains("\"..\"") || statement.contains(".parent()");
            assert!(
                !escapes,
                "{}: a test walks out of its own crate from CARGO_MANIFEST_DIR — tests target \
                 the fixture project, never this repository's root \
                 (tests/fixtures/README.md):\n{statement}",
                source.display()
            );
        }
    }
    assert!(
        checked >= 4,
        "expected the suite to locate fixtures in several places"
    );
}

fn collect_rust(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("tests dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn walk(dir: &Path) -> Vec<(PathBuf, u64)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("fixture dir").flatten() {
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path).unwrap();
        if meta.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push((path, meta.len()));
        }
    }
    out
}
