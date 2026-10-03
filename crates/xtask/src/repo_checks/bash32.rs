//! Check 8: the surviving shell script stays on the Bash 3.2 surface of the macOS runners.

use super::Failure;
use super::support::rel;
use crate::util::lines_without_opt_out;
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};

/// The script that remains shell (`restore.sh`, which bootstraps hosts that have nothing but
/// sh and git) is invoked with whatever `bash` the host has, and on the GitHub macOS runners
/// that is Apple's /bin/bash 3.2 — Bash
/// 4 never shipped (GPLv3). v0.3.6's release run 36865921206 exited 127 on both darwin legs
/// over one Bash-4 builtin; Linux (bash 5) and Windows (Git bash 5) passed, so macOS is the
/// only leg that can ever see this failure — the blind spot this check closes. actionlint
/// cannot help: the construct sits one invocation layer down, inside the script a step calls.
pub(super) fn bash32_surface(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    // The forbidden set is the Bash-4 surface people actually reach for: the array-read
    // builtins and coprocesses, associative arrays, case conversion, and `&>>`.
    let bash4 = Regex::new(
        r"(^|[^[:alnum:]_])(mapfile|readarray|coproc)([^[:alnum:]_]|$)|(declare|typeset)[\t ]+-[A-Za-z]*A([\t ]|$)|\$\{[A-Za-z_][A-Za-z0-9_]*(,,|\^\^)|&>>",
    )
    .expect("static regex");
    let mut hits = Vec::new();
    let scripts_dir = root.join("scripts");
    let mut scripts: Vec<PathBuf> = std::fs::read_dir(&scripts_dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "sh"))
                .collect()
        })
        .unwrap_or_default();
    scripts.sort();
    for file in scripts {
        let text = crate::util::read(&file)?;
        for (number, line) in lines_without_opt_out(&text) {
            if bash4.is_match(line) {
                hits.push(format!("{}:{number}:{line}", rel(root, &file)));
            }
        }
    }
    if !hits.is_empty() {
        failures.push(Failure::with(
            "a script uses a Bash-4-only construct (array-read builtins, declare -A, case conversion, &>>), which macOS's /bin/bash 3.2 cannot run (v0.3.6 run 36865921206):", // stale-ref-allowed
            hits,
            "rewrite it with 3.2-compatible builtins (e.g. a while-read process-substitution loop instead of the array builtin) — or better, move the logic into this xtask",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{headlines, valid_fixture};
    use proptest::prelude::*;
    use std::fs;

    #[test]
    fn a_bash4_builtin_in_a_surviving_script_fires_check_8() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join("scripts/gate.sh"),
            "#!/usr/bin/env bash\nmapfile -t lines < <(ls)\n",
        )
        .expect("script");
        let found = headlines(dir.path());
        assert!(
            found.iter().any(|h| h.contains("Bash-4-only construct")),
            "{found:?}"
        );
    }

    /// Noise around an injected token: letters, digits and underscores only, so it can never
    /// contribute the `&`, `$`, `{`, `}`, `-`, `,` or `^` characters the regex's own boundaries
    /// and alternatives key off — the property is about the token, not about accidentally
    /// forging or defeating a match out of random punctuation.
    fn noise() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_]{0,12}"
    }

    /// One instance of each forbidden alternative the regex's doc comment names. Each fires
    /// regardless of what whitespace-delimited text surrounds it.
    fn forbidden_token() -> impl Strategy<Value = &'static str> {
        prop::sample::select(
            [
                "mapfile",
                "readarray",
                "coproc",
                "declare -A",
                "typeset -A",
                "${x,,}",
                "${x^^}",
                "&>>",
            ]
            .as_slice(),
        )
    }

    /// Near-misses that must never fire: flags without the `A` the associative-array check
    /// looks for, a single comma/caret instead of the doubled case-conversion form, the
    /// builtin names as a substring of a longer identifier (both sides), and the common `2>&1`
    /// idiom, which is not `&>>`.
    fn allowed_token() -> impl Strategy<Value = &'static str> {
        prop::sample::select(
            [
                "declare -i",
                "declare -r",
                "typeset -x",
                "mapfile2",
                "a_mapfile_b",
                "${x,}",
                "2>&1",
                "echo hello",
            ]
            .as_slice(),
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn any_injected_forbidden_token_always_fires_check_8(
            before in noise(),
            after in noise(),
            token in forbidden_token(),
        ) {
            let dir = valid_fixture();
            fs::write(
                dir.path().join("scripts/gate.sh"),
                format!("#!/usr/bin/env bash\n{before} {token} {after}\n"),
            )
            .unwrap();
            let found = headlines(dir.path());
            prop_assert!(
                found.iter().any(|h| h.contains("Bash-4-only construct")),
                "token={} found={:?}",
                token,
                found
            );
        }

        #[test]
        fn allow_listed_vocabulary_never_fires_check_8(
            before in noise(),
            after in noise(),
            token in allowed_token(),
        ) {
            let dir = valid_fixture();
            fs::write(
                dir.path().join("scripts/gate.sh"),
                format!("#!/usr/bin/env bash\n{before} {token} {after}\n"),
            )
            .unwrap();
            let found = headlines(dir.path());
            prop_assert!(
                !found.iter().any(|h| h.contains("Bash-4-only construct")),
                "token={} found={:?}",
                token,
                found
            );
        }
    }
}
