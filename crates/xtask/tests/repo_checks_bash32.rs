//! `repo_checks::bash32` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use proptest::prelude::*;
use std::fs;
use support::{headlines, valid_fixture};

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
