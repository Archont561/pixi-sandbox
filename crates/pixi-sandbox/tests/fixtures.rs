//! The fixture policy, as tests.
//!
//! The rule (tests/fixtures/README.md, design.md §12): project-level tests use the fixture
//! project, never this repository. The repository *is* this tool's development environment —
//! its `dev` environment contains `pixi-pack`, its lockfile resolves hundreds of MiB — so a
//! test pointed at it silently passes on a developer's machine and fails (or never runs) on a
//! clean one. These tests make the rule executable instead of aspirational.

mod support;

use pixi_sandbox_core::manifest::Manifest;
use pixi_sandbox_core::verify;
use rstest::rstest;
use std::path::{Path, PathBuf};
use support::{crate_dir, demo_project, duplicate_source_project, fixture_transport as transport};

#[rstest]
#[case("pixi.toml")]
#[case("pixi.lock")]
#[case("Cargo.toml")]
#[case("Cargo.lock")]
#[case("src/main.rs")]
fn the_demo_project_carries_every_required_file(demo_project: PathBuf, #[case] required: &str) {
    assert!(
        demo_project.join(required).is_file(),
        "{required} is missing from the fixture project"
    );
}

#[test]
fn the_demo_project_manifest_declares_a_real_environment() {
    let manifest = std::fs::read_to_string(demo_project().join("pixi.toml")).unwrap();
    assert!(
        manifest.contains("[workspace]"),
        "a pixi manifest needs a workspace table"
    );
    assert!(manifest.contains("name = \"demo-project\""));
    assert!(
        manifest.contains("[dependencies]"),
        "the fixture must have a real environment"
    );
}

#[test]
fn the_demo_project_lockfile_resolves_packages() {
    // A lockfile with packages in it, so `pack`/`vendor`/hash steps have something to chew on.
    let lock = std::fs::read_to_string(demo_project().join("pixi.lock")).unwrap();
    assert!(
        lock.contains("conda:"),
        "the fixture lockfile must resolve packages"
    );
}

#[test]
fn the_demo_project_crate_is_its_own_workspace() {
    // The crate is its own cargo workspace: it must never be a member of ours.
    let cargo = std::fs::read_to_string(demo_project().join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("[workspace]"),
        "the fixture crate must declare its own [workspace] root"
    );
}

// This is the whole point: if the fixture pulled in pixi-pack/pixi-unpack, every test
// using it would depend on the developer's environment again.
#[rstest]
#[case("pixi.toml", "pixi-pack")]
#[case("pixi.toml", "pixi-unpack")]
#[case("pixi.lock", "pixi-pack")]
#[case("pixi.lock", "pixi-unpack")]
fn the_demo_project_does_not_depend_on_the_packers(
    demo_project: PathBuf,
    #[case] file: &str,
    #[case] forbidden: &str,
) {
    let text = std::fs::read_to_string(demo_project.join(file)).unwrap();
    // Comments are documentation, dependencies are the contract: only the latter count.
    // (The fixture's pixi.toml explains *why* the packers are absent, at length.)
    let dependencies: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !dependencies.contains(forbidden),
        "{file} depends on {forbidden}: the fixture must be a plain project, and a test \
         that needs a packer must ask for one explicitly (tests/fixtures/README.md)"
    );
}

#[rstest]
#[case("pixi.toml")]
#[case("pixi.lock")]
#[case("Cargo.toml")]
#[case("Cargo.lock")]
#[case("src/main.rs")]
fn the_duplicate_source_project_carries_every_required_file(
    duplicate_source_project: PathBuf,
    #[case] required: &str,
) {
    assert!(
        duplicate_source_project.join(required).is_file(),
        "{required} is missing from the duplicate-source fixture"
    );
}

// A second project fixture, for the pack-time duplicate-crate-source check (design.md §11,
// backlog task-8). It carries the same policy as `demo-project`: a plain pixi project with
// no packer, so using it cannot depend on the developer's environment.
#[rstest]
#[case("pixi.toml", "pixi-pack")]
#[case("pixi.toml", "pixi-unpack")]
#[case("pixi.lock", "pixi-pack")]
#[case("pixi.lock", "pixi-unpack")]
fn the_duplicate_source_project_does_not_depend_on_the_packers(
    duplicate_source_project: PathBuf,
    #[case] file: &str,
    #[case] forbidden: &str,
) {
    let text = std::fs::read_to_string(duplicate_source_project.join(file)).unwrap();
    assert!(
        !text.contains(forbidden),
        "{file} depends on {forbidden}: every project fixture must be a plain project \
         (tests/fixtures/README.md)"
    );
}

#[rstest]
fn the_duplicate_source_project_carries_the_collision_it_exists_for(
    duplicate_source_project: PathBuf,
) {
    // The lockfile must still contain the collision, or the test that uses it would be
    // asserting nothing. `toml` is a dependency of the CLI crate, not this test binary, so
    // the check is deliberately textual rather than a parse.
    let lock = std::fs::read_to_string(duplicate_source_project.join("Cargo.lock")).unwrap();
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
/// Task-38's support module is a policy boundary, not a convenience import: a local copy of
/// any listed helper is a regression because it lets setup semantics drift without a reviewer
/// noticing. Keep the definitions in support and keep their rstest fixture entrypoints visible.
#[test]
fn shared_test_helpers_are_defined_once_and_exposed_as_fixtures() {
    let tests_dir = crate_dir().join("tests");
    let support = tests_dir.join("support/mod.rs");
    let support_text = std::fs::read_to_string(&support).unwrap();
    let helpers = [
        ("bin", "bin"),
        ("isolated_home", "isolated_home"),
        ("fixture_transport", "fixture_transport"),
        ("demo_project", "demo_project"),
        ("isolated_bin", "isolated_bin_fixture"),
        ("copy_tree", "copy_tree_fixture"),
        ("make_executable", "make_executable_fixture"),
        ("host_platform", "host_platform"),
        ("git", "git_fixture"),
        ("run_git", "run_git_fixture"),
        ("commit", "commit_fixture"),
        ("transport_repo", "transport_repo_fixture"),
        ("write_executable", "write_executable_fixture"),
        ("fake_tools", "fake_tools_fixture"),
        ("path_with_fake_tools", "path_with_fake_tools_fixture"),
        ("file_tree", "file_tree_fixture"),
        ("pack_reference", "pack_reference"),
        ("synthetic_pack_fixture", "synthetic_pack_fixture"),
    ];
    let mut sources = Vec::new();
    collect_rust(&tests_dir, &mut sources);

    for (helper, fixture) in helpers {
        let definition = format!("fn {helper}(");
        let defined_in = sources
            .iter()
            .filter(|path| {
                std::fs::read_to_string(path)
                    .unwrap()
                    .lines()
                    .map(str::trim_start)
                    .any(|line| {
                        line.starts_with(&definition)
                            || line.starts_with(&format!("pub {definition}"))
                    })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            defined_in,
            [&support],
            "{helper} must have exactly one test-support definition"
        );
        assert!(
            support_text.contains(&format!("#[fixture]\npub fn {fixture}(")),
            "{helper} must stay available through the {fixture} rstest fixture"
        );
    }
}

/// Tests live under `tests/`, never in the file they judge.
///
/// A `#[cfg(test)] mod tests` beside production code puts the oracle and the thing it judges
/// in one file, where a refactor can quietly adjust both at once; it also hides the module's
/// real testable surface, because a unit test reaches privates an outside caller never can.
/// So a module that deserves tests is promoted to `lib.rs` and tested through its public API
/// from `tests/`, and anything genuinely private to the binary target (`cli.rs`, `commands/`)
/// is covered black-box by driving the binary.
///
/// The list below is the remaining debt from before the rule, not an escape hatch: it may
/// shrink, never grow. Adding a file to it in a pull request is the signal to split that
/// module instead.
#[test]
fn production_sources_carry_no_inline_test_modules() {
    // Pre-rule modules awaiting the same split. Shrink this list; do not extend it.
    const LEGACY: [&str; 5] = [
        "commands/init.rs",
        "commands/restore.rs",
        "commands/tools/update.rs",
        "generated/relock_workflow.rs",
        "user_tools.rs",
    ];

    let src = crate_dir().join("src");
    let mut sources = Vec::new();
    collect_rust(&src, &mut sources);
    assert!(!sources.is_empty(), "no production sources found");

    let mut offenders = Vec::new();
    let mut legacy_seen = Vec::new();
    for source in sources {
        let relative = source
            .strip_prefix(&src)
            .expect("source sits under src")
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&source).expect("production source is UTF-8");
        if !text.contains("#[cfg(test)]") {
            continue;
        }
        if LEGACY.contains(&relative.as_str()) {
            legacy_seen.push(relative);
        } else {
            offenders.push(relative);
        }
    }

    assert!(
        offenders.is_empty(),
        "these production sources carry inline `#[cfg(test)]` tests:\n  {}\n\n\
         Move them to `crates/pixi-sandbox/tests/`: promote the module to `lib.rs` as `pub mod` \
         and test it through its public API, or — if it is private to the binary target — \
         drive the binary black-box from `tests/cli.rs`.",
        offenders.join("\n  ")
    );

    let stale: Vec<&str> = LEGACY
        .iter()
        .copied()
        .filter(|path| !legacy_seen.iter().any(|seen| seen == path))
        .collect();
    assert!(
        stale.is_empty(),
        "these entries no longer carry inline tests — delete them from LEGACY so the list \
         keeps shrinking:\n  {}",
        stale.join("\n  ")
    );
}

/// Keep coverage from silently drifting toward only the CLI's happy path. This is a lightweight
/// structural guard rather than a replacement for llvm-cov: every production module must have its
/// derived module symbol mentioned by a test under `tests/`. The inline-`#[cfg(test)]` branch
/// below survives only for the `LEGACY` modules that have not been split yet; new modules take
/// the `tests/` route (see `production_sources_carry_no_inline_test_modules`). `main.rs`
/// is wiring only and deliberately excluded.
#[test]
fn coverage_guard_requires_a_test_route_for_every_production_module() {
    let crate_root = crate_dir();
    let src = crate_root.join("src");
    let mut production = Vec::new();
    collect_rust(&src, &mut production);
    let mut test_sources = Vec::new();
    collect_rust(&crate_root.join("tests"), &mut test_sources);
    let test_text = test_sources
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("test source is UTF-8"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut uncovered = Vec::new();
    for source in production {
        let relative = source.strip_prefix(&src).expect("source sits under src");
        if relative == Path::new("main.rs") {
            continue;
        }
        let stem = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("Rust source has a UTF-8 file stem");
        let symbol = match stem {
            // `mod.rs` is named after its containing module; `lib.rs` is the crate symbol used
            // by integration tests. Both cases make failure output point at a useful remedy.
            "mod" => relative
                .parent()
                .and_then(Path::file_name)
                .and_then(|part| part.to_str())
                .unwrap_or("module"),
            "lib" => "pixi_sandbox",
            _ => stem,
        };
        let source_text = std::fs::read_to_string(&source).expect("production source is UTF-8");
        if source_text.contains("#[cfg(test)]") || test_text.contains(symbol) {
            continue;
        }
        uncovered.push(format!("{} (symbol `{symbol}`)", relative.display()));
    }

    assert!(
        uncovered.is_empty(),
        "coverage guard found production modules with no unit-test block or integration-test \
         symbol reference:\n  {}\n\nAdd a focused test under tests/ that names the module symbol, \
         or add a #[cfg(test)] unit-test module beside the production code. `src/main.rs` is \
         intentionally excluded because it is CLI wiring.",
        uncovered.join("\n  ")
    );
}

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
