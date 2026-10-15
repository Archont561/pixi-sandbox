//! The `pixi.lock` scanner `pack` uses to refuse a `PyPI` dependency `pixi-pack` cannot carry
//! (issue #157), through the promoted library boundary.

use pixi_sandbox::pixi_lock::{NonWheelPypi, non_wheel_pypi, refuse_non_wheel_pypi};
use tempfile::TempDir;

/// A v7 lock with one environment whose single `PyPI` dependency is a path/editable source, the
/// shape that made `pixi-pack` bail with "package X is not a wheel file" (polybridge PR #1).
const PATH_DEPENDENCY: &str = "\
version: 7
platforms:
- name: linux-64
  virtual-packages:
  - __unix=0=0
environments:
  orchestration:
    channels:
    - url: https://conda.anaconda.org/conda-forge/
    indexes:
    - https://pypi.org/simple
    packages:
      linux-64:
      - conda: https://conda.anaconda.org/conda-forge/linux-64/zlib-1.3.2-h25fd6f3_3.conda
      - pypi: ./runtimes/engine
packages:
- conda: https://conda.anaconda.org/conda-forge/linux-64/zlib-1.3.2-h25fd6f3_3.conda
  sha256: 00
- pypi: ./runtimes/engine
  name: polyglot-runtime
  version: 0.1.0
";

/// A lock that mixes every location shape a real environment sees: wheels (plain, fragment,
/// and a `direct+` URL to a wheel), a git source, an sdist, and a path/editable source across
/// two platforms and three environments (one with no top-level `name` entry).
const MIXED: &str = "\
version: 7
environments:
  orchestration:
    channels:
    - url: https://conda.anaconda.org/conda-forge/
    packages:
      linux-64:
      - conda: https://conda.anaconda.org/conda-forge/linux-64/zlib-1.3.2-h25fd6f3_3.conda
      - pypi: ./runtimes/engine
      - pypi: https://files.example.org/packages/b7/ce/six-1.17.0-py2.py3-none-any.whl#sha256=4721f391ed90541fddacab5acf947aa0d3dc7d27b2e1e8eda2be8970586c3274
  web:
    channels:
    - url: https://conda.anaconda.org/conda-forge/
    packages:
      linux-64:
      - pypi: git+https://github.com/pallets/flask#f61172b8dd3f962d33f25c50b2f5405e90ceffa5
      - pypi: https://files.example.org/packages/a2/6c/typer-0.27.3-py3-none-any.whl
      - pypi: direct+https://github.com/pallets/click/releases/download/8.1.7/click-8.1.7-py3-none-any.whl
      - pypi: https://files.example.org/packages/de/ad/sdist-only-1.0.0.tar.gz
      osx-64:
      - pypi: ./runtimes/engine
  unlisted:
    channels:
    - url: https://conda.anaconda.org/conda-forge/
    packages:
      linux-64:
      - pypi: ./anonymous
packages:
- pypi: ./runtimes/engine
  name: polyglot-runtime
  version: 0.1.0
- pypi: git+https://github.com/pallets/flask#f61172b8dd3f962d33f25c50b2f5405e90ceffa5
  name: flask
  version: 3.2.0.dev0
- pypi: https://files.example.org/packages/de/ad/sdist-only-1.0.0.tar.gz
  name: sdist-only
  version: 1.0.0
";

fn envs(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn finding(environment: &str, package: &str, location: &str) -> NonWheelPypi {
    NonWheelPypi {
        environment: environment.to_string(),
        package: package.to_string(),
        location: location.to_string(),
    }
}

#[test]
fn a_path_pypi_dependency_is_reported_with_its_environment_and_name() {
    let findings = non_wheel_pypi(PATH_DEPENDENCY, &envs(&["orchestration"]), "linux-64");
    assert_eq!(
        findings,
        vec![NonWheelPypi {
            environment: "orchestration".to_string(),
            package: "polyglot-runtime".to_string(),
            location: "./runtimes/engine".to_string(),
        }]
    );
}

#[test]
fn wheels_fragments_and_direct_wheel_urls_are_accepted() {
    // `orchestration` on linux-64 is a path source plus a wheel: only the path source is a
    // finding, so the `#sha256=` wheel must not appear.
    let findings = non_wheel_pypi(MIXED, &envs(&["orchestration"]), "linux-64");
    assert_eq!(
        findings,
        vec![finding(
            "orchestration",
            "polyglot-runtime",
            "./runtimes/engine"
        )],
        "a wheel, with or without a sha256 fragment, is transportable"
    );

    // `web` on linux-64 carries a `direct+…whl` URL (a wheel) alongside a git source and an
    // sdist; the first is accepted, the other two are findings.
    let findings = non_wheel_pypi(MIXED, &envs(&["web"]), "linux-64");
    assert_eq!(
        findings,
        vec![
            finding(
                "web",
                "flask",
                "git+https://github.com/pallets/flask#f61172b8dd3f962d33f25c50b2f5405e90ceffa5"
            ),
            finding(
                "web",
                "sdist-only",
                "https://files.example.org/packages/de/ad/sdist-only-1.0.0.tar.gz"
            ),
        ]
    );
}

#[test]
fn every_offending_dependency_is_reported_across_environments_in_order() {
    let findings = non_wheel_pypi(MIXED, &envs(&["orchestration", "web"]), "linux-64");
    assert_eq!(
        findings,
        vec![
            finding("orchestration", "polyglot-runtime", "./runtimes/engine"),
            finding(
                "web",
                "flask",
                "git+https://github.com/pallets/flask#f61172b8dd3f962d33f25c50b2f5405e90ceffa5"
            ),
            finding(
                "web",
                "sdist-only",
                "https://files.example.org/packages/de/ad/sdist-only-1.0.0.tar.gz"
            ),
        ]
    );
}

#[test]
fn only_the_requested_environment_and_platform_are_checked() {
    // The path source is listed for `web` on osx-64, not linux-64, and `orchestration` is not
    // selected — so a linux-64 check of `web` sees only the linux-64 findings.
    assert_eq!(
        non_wheel_pypi(MIXED, &envs(&["web"]), "osx-64"),
        vec![finding("web", "polyglot-runtime", "./runtimes/engine")]
    );
    assert!(
        non_wheel_pypi(MIXED, &envs(&["orchestration"]), "osx-64").is_empty(),
        "an unselected platform must not produce a finding"
    );
}

#[test]
fn a_location_without_a_name_entry_falls_back_to_the_location() {
    assert_eq!(
        non_wheel_pypi(MIXED, &envs(&["unlisted"]), "linux-64"),
        vec![finding("unlisted", "./anonymous", "./anonymous")]
    );
}

#[test]
fn an_unknown_environment_reports_nothing() {
    assert!(non_wheel_pypi(MIXED, &envs(&["absent"]), "linux-64").is_empty());
}

fn write_lock(lock: &str) -> (TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pixi.lock");
    std::fs::write(&path, lock).unwrap();
    (dir, path)
}

#[test]
fn the_refusal_names_every_environment_package_and_location() {
    let (_dir, path) = write_lock(MIXED);
    let error = refuse_non_wheel_pypi(&path, &envs(&["orchestration", "web"]), "linux-64")
        .expect_err("a non-wheel dependency must be refused");
    let message = format!("{error:#}");
    for expected in [
        "environment `orchestration`",
        "`polyglot-runtime`",
        "`./runtimes/engine`",
        "environment `web`",
        "`flask`",
        "`sdist-only`",
        "build it into a wheel",
    ] {
        assert!(
            message.contains(expected),
            "missing {expected:?} in:\n{message}"
        );
    }
}

#[test]
fn a_transportable_lock_is_not_refused() {
    let wheels = "\
version: 7
environments:
  default:
    channels:
    - url: https://conda.anaconda.org/conda-forge/
    packages:
      linux-64:
      - pypi: https://files.example.org/packages/b7/ce/six-1.17.0-py2.py3-none-any.whl#sha256=00
packages:
- pypi: https://files.example.org/packages/b7/ce/six-1.17.0-py2.py3-none-any.whl#sha256=00
  name: six
  version: 1.17.0
";
    let (_dir, path) = write_lock(wheels);
    refuse_non_wheel_pypi(&path, &envs(&["default"]), "linux-64")
        .expect("a wheel-only environment is transportable");
}

#[test]
fn an_unreadable_lock_is_reported_with_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.lock");
    let error = refuse_non_wheel_pypi(&path, &envs(&["default"]), "linux-64")
        .expect_err("a missing lock is an error");
    assert!(format!("{error:#}").contains("absent.lock"), "{error:#}");
}

#[test]
fn a_lock_without_pypi_dependencies_or_an_empty_lock_reports_nothing() {
    let conda_only = "\
version: 7
environments:
  default:
    channels:
    - url: https://conda.anaconda.org/conda-forge/
    packages:
      linux-64:
      - conda: https://conda.anaconda.org/conda-forge/linux-64/zlib-1.3.2-h25fd6f3_3.conda
packages:
- conda: https://conda.anaconda.org/conda-forge/linux-64/zlib-1.3.2-h25fd6f3_3.conda
  sha256: 00
";
    assert!(non_wheel_pypi(conda_only, &envs(&["default"]), "linux-64").is_empty());
    assert!(non_wheel_pypi("", &envs(&["default"]), "linux-64").is_empty());
}
