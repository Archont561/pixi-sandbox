//! Integration tests for the standalone-execution probe (`src/standalone.rs`).
//!
//! Issue #81 shipped a `pixi global` trampoline as a transport's self-bin: every declared
//! check was green — the copy was faithful, the trampoline static — and the branch still
//! died at restore. These tests pin the probe's two judgements (structural provenance for
//! the known shapes, behavioural execution for everything else) and the isolation contract
//! of the real runner. The runner is scripted through [`ProbeRunner`] so every verdict is
//! exercised without a process; the real `CommandRunner` is covered with real scripts where
//! *its* contract — an empty environment anchored at scratch — is the thing under test.

use pixi_sandbox::standalone::{
    CommandRunner, ProbeRunner, ProbeVerdict, host_platform, pack_refusal_for_ownership, platform,
    probe,
};
use std::fs;
use std::path::Path;

/// The issue's exact failure text, as a pixi global trampoline produces it.
const TRAMPOLINE_OUTPUT: &str =
    "Couldn't open \"tools/linux-64/trampoline_configuration/pixi-sandbox.json\"";

struct FakeRunner(ProbeVerdict);

impl ProbeRunner for FakeRunner {
    fn run_version(&self, _exe: &Path, _anchor: &Path) -> ProbeVerdict {
        self.0.clone()
    }
}

fn anchor() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("probe anchor");
    fs::create_dir_all(dir.path().join("tmp")).expect("anchor tmp");
    dir
}

#[test]
fn exit_zero_is_the_whole_contract() {
    let anchor = anchor();
    for verdict in [
        ProbeVerdict::Exited {
            success: false,
            output: TRAMPOLINE_OUTPUT.to_string(),
        },
        ProbeVerdict::TimedOut,
        ProbeVerdict::SpawnFailed("could not be started: no such file".to_string()),
    ] {
        assert!(
            probe(Path::new("/candidate"), anchor.path(), &FakeRunner(verdict)).is_err(),
            "a non-clean exit must be refused"
        );
    }
    let ok = ProbeVerdict::Exited {
        success: true,
        output: "pixi-sandbox 0.5.0".to_string(),
    };
    probe(Path::new("/candidate"), anchor.path(), &FakeRunner(ok))
        .expect("a clean --version exit passes");
}

#[test]
fn a_trampoline_verdict_renders_the_evidence_and_the_only_remedy() {
    let anchor = anchor();
    let verdict = ProbeVerdict::Exited {
        success: false,
        output: TRAMPOLINE_OUTPUT.to_string(),
    };
    let refusal = probe(
        Path::new("/transport/.pixi-sandbox/tools/linux-64/pixi-sandbox"),
        anchor.path(),
        &FakeRunner(verdict.clone()),
    )
    .expect_err("the trampoline must be refused");

    assert_eq!(refusal.verdict(), &verdict, "the evidence stays attached");
    let text = refusal.render(Path::new(
        "/transport/.pixi-sandbox/tools/linux-64/pixi-sandbox",
    ));
    assert!(text.contains("does not run standalone"), "{text}");
    assert!(
        text.contains("trampoline_configuration/pixi-sandbox.json"),
        "the captured child output is the evidence an airlock operator needs: {text}"
    );
    assert!(
        text.contains("standalone release asset `pixi-sandbox-<target>`"),
        "the remedy names the only self-bin source that works: {text}"
    );
    assert!(
        text.contains("$(command -v pixi-sandbox)"),
        "the remedy names the trap that produced issue #81: {text}"
    );
    assert!(
        text.contains("not pinned into the manifest"),
        "the message explains why hashes were green: {text}"
    );
}

#[test]
fn spawn_failure_and_timeout_are_distinct_refusals() {
    let anchor = anchor();
    let spawned = probe(
        Path::new("/candidate"),
        anchor.path(),
        &FakeRunner(ProbeVerdict::SpawnFailed(
            "could not be started: exec format error".to_string(),
        )),
    )
    .expect_err("spawn failure refuses");
    assert!(
        spawned
            .render(Path::new("/candidate"))
            .contains("could not be started: exec format error")
    );

    let hung = probe(
        Path::new("/candidate"),
        anchor.path(),
        &FakeRunner(ProbeVerdict::TimedOut),
    )
    .expect_err("a hung candidate refuses");
    assert!(
        hung.render(Path::new("/candidate"))
            .contains("30 s probe timeout")
    );
}

#[test]
fn the_host_mapping_names_exactly_the_release_matrix() {
    assert_eq!(platform("linux", "x86_64"), Some("linux-64"));
    assert_eq!(platform("linux", "aarch64"), Some("linux-aarch64"));
    assert_eq!(platform("macos", "x86_64"), Some("osx-64"));
    assert_eq!(platform("macos", "aarch64"), Some("osx-arm64"));
    assert_eq!(platform("windows", "x86_64"), Some("win-64"));
    assert_eq!(platform("freebsd", "x86_64"), None);
    assert_eq!(platform("linux", "riscv64"), None);
    assert_eq!(
        host_platform(),
        platform(std::env::consts::OS, std::env::consts::ARCH)
    );
}

#[test]
fn pack_refusal_names_a_pixi_global_trampoline_by_its_sibling_config() {
    let dir = tempfile::tempdir().expect("prefix");
    let candidate = dir.path().join("pixi-sandbox");
    fs::write(&candidate, b"\x7fELF trampoline bytes").expect("candidate");
    let configuration = dir.path().join("trampoline_configuration");
    fs::create_dir_all(&configuration).expect("configuration dir");
    fs::write(configuration.join("pixi-sandbox.json"), b"{}").expect("configuration");

    let refusal = pack_refusal_for_ownership(&candidate)
        .expect("a global trampoline is refused before any execution");
    assert!(
        refusal.contains("`pixi global install` trampoline"),
        "{refusal}"
    );
    assert!(
        refusal.contains("standalone release asset `pixi-sandbox-<target>`"),
        "{refusal}"
    );
    assert!(refusal.contains("SHA256SUMS"), "{refusal}");
}

#[test]
fn pack_refusal_names_a_managed_launcher_script() {
    let dir = tempfile::tempdir().expect("prefix");
    let candidate = dir.path().join("pixi-sandbox");
    fs::write(
        &candidate,
        format!(
            "#!/bin/sh\n# {} - regenerated by every restore; safe to delete\n",
            pixi_sandbox::user_tools::MANAGED_MARKER
        ),
    )
    .expect("launcher");

    let refusal = pack_refusal_for_ownership(&candidate)
        .expect("a managed launcher script is refused before any execution");
    assert!(refusal.contains("managed launcher script"), "{refusal}");
    assert!(refusal.contains(".pixi/tools/<platform>/"), "{refusal}");
}

#[test]
fn pack_refusal_leaves_runnable_provenance_to_the_execution_probe() {
    let dir = tempfile::tempdir().expect("prefix");

    // A plain standalone path: the ladder says nothing, execution judges.
    let standalone_bin = dir.path().join("pixi-sandbox");
    fs::write(&standalone_bin, b"\x7fELF real binary").expect("binary");
    assert_eq!(pack_refusal_for_ownership(&standalone_bin), None);

    // A restored transport tool is a *good* self-bin for a repack — classify names it and
    // pack must not refuse it.
    let tool = dir.path().join(".pixi/tools/linux-64/pixi-sandbox");
    fs::create_dir_all(tool.parent().expect("tools dir")).expect("tools tree");
    fs::write(&tool, b"\x7fELF real binary").expect("transport tool");
    assert_eq!(pack_refusal_for_ownership(&tool), None);

    // A conda prefix binary is the real program, not a launcher: execution judges it too.
    let prefix_bin = dir.path().join("env/bin/pixi-sandbox");
    fs::create_dir_all(dir.path().join("env/conda-meta")).expect("conda-meta");
    fs::create_dir_all(prefix_bin.parent().expect("bin dir")).expect("bin tree");
    fs::write(&prefix_bin, b"\x7fELF real binary").expect("prefix binary");
    assert_eq!(pack_refusal_for_ownership(&prefix_bin), None);
}

// The real runner, against real scripts: here the *anchor contract* is the test — an empty
// environment, scratch HOME/TMP/CWD, merged and capped output, and a kill on timeout.
#[cfg(unix)]
mod real_runner {
    use super::*;
    use std::time::Duration;

    fn write_script(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, body).expect("script");
        let mut permissions = fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("chmod");
    }

    #[test]
    fn the_candidate_gets_an_empty_environment_anchored_at_scratch() {
        let anchor = anchor();
        let exe = anchor.path().join("candidate");
        write_script(
            &exe,
            r#"#!/bin/sh
printf 'home=%s path=%s cwd=%s\n' "$HOME" "$PATH" "$(pwd)"
"#,
        );
        let verdict = CommandRunner::new().run_version(&exe, anchor.path());
        let ProbeVerdict::Exited { success, output } = verdict else {
            panic!("a well-formed script must exit: {verdict:?}");
        };
        assert!(success, "{output}");
        let expected_home = format!("home={}", anchor.path().display());
        assert!(
            output.contains(&expected_home),
            "HOME is the anchor, not the ambient profile: {output}"
        );
        assert!(
            output.contains("path=/usr/bin:/bin"),
            "PATH carries no pixi/conda shims: {output}"
        );
        let expected_cwd = format!("cwd={}", anchor.path().display());
        assert!(
            output.contains(&expected_cwd),
            "the candidate works from scratch, not from beside itself: {output}"
        );
    }

    #[test]
    fn a_trampoline_shaped_script_fails_with_its_own_words_captured() {
        let anchor = anchor();
        let exe = anchor.path().join("pixi-sandbox");
        write_script(
            &exe,
            r#"#!/bin/sh
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
config="$dir/trampoline_configuration/pixi-sandbox.json"
if [ ! -f "$config" ]; then
  printf "Couldn't open \"%s\"\n" "$config" >&2
  exit 1
fi
echo 'pixi-sandbox 0.5.0'
"#,
        );
        let verdict = CommandRunner::new().run_version(&exe, anchor.path());
        let ProbeVerdict::Exited { success, output } = verdict else {
            panic!("the trampoline script ran and failed: {verdict:?}");
        };
        assert!(!success, "no config beside the probe anchor: {output}");
        assert!(
            output.contains("Couldn't open")
                && output.contains("trampoline_configuration/pixi-sandbox.json"),
            "stderr is captured into the verdict: {output}"
        );
    }

    #[test]
    fn a_hung_candidate_is_killed_at_the_timeout() {
        let anchor = anchor();
        let exe = anchor.path().join("sleeper");
        write_script(&exe, "#!/bin/sh\nsleep 30\n");
        let runner = CommandRunner::with_timeout(Duration::from_millis(150));
        let started = std::time::Instant::now();
        let verdict = runner.run_version(&exe, anchor.path());
        assert_eq!(verdict, ProbeVerdict::TimedOut);
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the kill is prompt; the suite must not nap"
        );
    }

    #[test]
    fn the_runner_resolves_relative_paths_against_the_calling_cwd() {
        // Regression (found by smoke-testing the built binary, not the suite): a relative
        // branch location reached the runner, the child chdir'd into the anchor, and the
        // spawn failed with an ENOENT that blamed the wrong thing. One test per process
        // under nextest is what makes the cwd dance here safe.
        let dir = tempfile::tempdir().expect("workdir");
        let exe = dir.path().join("candidate");
        write_script(&exe, "#!/bin/sh\necho relative-ok\n");
        let anchor = dir.path().join("anchor");
        fs::create_dir_all(anchor.join("tmp")).expect("anchor tmp");
        std::env::set_current_dir(dir.path()).expect("chdir");

        let verdict = CommandRunner::new().run_version(Path::new("candidate"), Path::new("anchor"));
        assert_eq!(
            verdict,
            ProbeVerdict::Exited {
                success: true,
                output: "relative-ok\n".to_string()
            }
        );
    }

    #[test]
    fn a_missing_candidate_is_a_spawn_failure_not_a_panic() {
        let anchor = anchor();
        let verdict =
            CommandRunner::new().run_version(&anchor.path().join("not-here"), anchor.path());
        let ProbeVerdict::SpawnFailed(detail) = verdict else {
            panic!("a missing file cannot start: {verdict:?}");
        };
        assert!(detail.contains("could not be started"), "{detail}");
    }
}
