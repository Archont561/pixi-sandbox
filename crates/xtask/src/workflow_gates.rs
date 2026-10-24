//! Fail-closed scheduling contracts for the repository's supporting workflows (TASK-127).
//! These are structural checks, not a replacement for actual native-runner evidence.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

fn load(root: &Path, name: &str) -> Result<Value> {
    let path = root.join(".github/workflows").join(name);
    serde_yaml::from_str(
        &fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))
}

fn names(value: &Value) -> BTreeSet<&str> {
    if let Some(name) = value.as_str() {
        return [name].into_iter().collect();
    }
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn steps(job: &Value) -> impl Iterator<Item = &Value> {
    job["steps"].as_array().into_iter().flatten()
}

/// Enforce early Linux proof plus complete, success-dependent final publication.
///
/// # Errors
/// Refuses a missing/invalid workflow, weakened dependencies or an unexpected Linux artifact.
pub fn check_release_scheduling(root: &Path) -> Result<()> {
    let release = load(root, "release.yml")?;
    let jobs = &release["jobs"];
    for name in ["linux-binary", "build", "linux-package", "consumer-proof"] {
        ensure!(
            jobs[name]["continue-on-error"].is_null() && jobs[name]["if"].is_null(),
            "{name} must remain mandatory and success-gated"
        );
        ensure!(
            steps(&jobs[name]).all(|s| s["continue-on-error"].is_null()),
            "{name} must not ignore a failed build, artifact or proof step"
        );
    }
    ensure!(
        names(&jobs["consumer-proof"]["needs"]) == ["linux-binary"].into_iter().collect(),
        "consumer proof must depend only on linux-binary"
    );
    ensure!(
        jobs["consumer-proof"]["if"].is_null(),
        "consumer proof must retain default success gating"
    );
    ensure!(
        names(&jobs["release"]["needs"])
            == ["build", "linux-binary", "linux-package", "consumer-proof"]
                .into_iter()
                .collect(),
        "publication must require all binaries, packages and consumer proof"
    );
    ensure!(
        jobs["release"]["if"].as_str()
            == Some("${{ startsWith(github.ref, 'refs/tags/') || inputs.version != '' }}"),
        "publication must retain its tag/dispatch condition and implicit success gate"
    );
    let targets: BTreeSet<_> = jobs["build"]["strategy"]["matrix"]["include"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row["target"].as_str())
        .collect();
    ensure!(
        targets
            == [
                "aarch64-unknown-linux-musl",
                "x86_64-apple-darwin",
                "aarch64-apple-darwin",
                "x86_64-pc-windows-msvc"
            ]
            .into_iter()
            .collect(),
        "the remaining four native release targets must be retained"
    );
    ensure!(
        steps(&jobs["linux-binary"]).any(|s| s["run"]
            == "pixi run -e package build-release-binary -- --target x86_64-unknown-linux-musl"),
        "Linux bootstrap must remain a musl release build"
    );
    ensure!(
        names(&jobs["linux-package"]["needs"]) == ["linux-binary"].into_iter().collect(),
        "Linux packaging must follow its binary build"
    );
    ensure!(
        steps(&jobs["linux-package"])
            .any(|s| s["run"] == "pixi run -e package xtask smoke-conda-package"),
        "Linux package smoke test must remain"
    );
    ensure!(
        steps(&jobs["consumer-proof"]).any(|s| s["uses"]
            .as_str()
            .is_some_and(|u| u.starts_with("actions/download-artifact@"))
            && s["with"]["name"] == "binary-x86_64-unknown-linux-musl"
            && s["with"]["run-id"].is_null()
            && s["with"]["repository"].is_null()),
        "consumer proof must use the matching Linux artifact from this run"
    );
    for command in [
        "pixi run -e package xtask check-conda-platforms dist/conda",
        "pixi run -e package xtask check-release-binaries dist",
    ] {
        ensure!(
            steps(&jobs["release"]).any(|s| s["run"] == command),
            "missing final completeness gate: {command}"
        );
    }
    println!(
        "release scheduling: Linux proof is early; publication remains success-gated on every platform and proof"
    );
    Ok(())
}

/// The exact tag globs that split the two release workflows (doc-18 §5.2): milestone tags
/// start the prepare-and-push half, strict semver tags the build-and-publish half.
const MILESTONE_TAG_GLOB: &str = "milestone-v*";
const RELEASE_TAG_GLOB: &str = "v*.*.*";

/// Guard the milestone auto-release trigger.
///
/// `milestone-v*` starts `auto-release.yml`, whose resolver step turns the tag into the
/// explicit version `prepare-release` cuts; `release.yml` keeps the strict `v*.*.*` glob,
/// whose leading-`v` anchor cannot match a milestone tag, so one tag can never build twice.
///
/// # Errors
/// Refuses a missing/widened/duplicated tag glob, a removed manual trigger lane, a missing
/// or unguarded milestone resolver step, or a prepare step that ignores its resolved output.
pub fn check_release_triggers(root: &Path) -> Result<()> {
    let auto = load(root, "auto-release.yml")?;
    let on = &auto["on"];
    let milestone_tags = names(&on["push"]["tags"]);
    ensure!(
        milestone_tags == [MILESTONE_TAG_GLOB].into_iter().collect(),
        "auto-release.yml must trigger on push tags of exactly {MILESTONE_TAG_GLOB} (one glob, no widening, no overlap with release.yml)"
    );
    ensure!(
        on["workflow_dispatch"].is_object(),
        "auto-release.yml must keep its workflow_dispatch lane for manual cuts"
    );
    ensure!(
        on["workflow_call"].is_object(),
        "auto-release.yml must keep its workflow_call lane for reusable pipelines"
    );
    let cut_steps: Vec<_> = steps(&auto["jobs"]["cut"]).collect();
    let resolver = cut_steps
        .iter()
        .copied()
        .find(|s| s["run"].as_str().is_some_and(|r| r.contains("resolve-milestone-tag")))
        .context(
            "auto-release.yml needs a step running `xtask resolve-milestone-tag` before prepare-release",
        )?;
    ensure!(
        resolver["id"].as_str() == Some("milestone"),
        "the resolve-milestone-tag step must keep id `milestone` so prepare reads steps.milestone.outputs.version"
    );
    ensure!(
        resolver["if"].as_str() == Some("github.event_name == 'push'"),
        "the resolve-milestone-tag step must run only when github.event_name == 'push': a dispatch or call has no milestone ref_name and would fail the resolve"
    );
    ensure!(
        resolver["run"]
            .as_str()
            .is_some_and(|r| r.contains("github.ref_name")),
        "the resolve-milestone-tag step must read the pushed name from github.ref_name"
    );
    let prep = cut_steps
        .iter()
        .copied()
        .find(|s| s["id"].as_str() == Some("prep"))
        .context("auto-release.yml must keep its `prep` prepare-release step")?;
    let prep_run = prep["run"]
        .as_str()
        .context("the prepare step must keep its one-line run")?;
    ensure!(
        prep_run.contains("steps.milestone.outputs.version"),
        "the prepare step must consume steps.milestone.outputs.version on tag pushes"
    );
    ensure!(
        prep_run.contains("inputs.bump"),
        "the prepare step must keep its dispatch/call fallback (inputs.version then inputs.bump)"
    );

    let release = load(root, "release.yml")?;
    let release_tags = names(&release["on"]["push"]["tags"]);
    ensure!(
        release_tags == [RELEASE_TAG_GLOB].into_iter().collect(),
        "release.yml must keep exactly the {RELEASE_TAG_GLOB} tag glob: its leading-v anchor is what stops a milestone tag double-building the artifacts"
    );
    println!(
        "release triggers: {MILESTONE_TAG_GLOB} starts auto-release; {RELEASE_TAG_GLOB} alone starts the build half"
    );
    Ok(())
}

const DOCS_TRUST: &str = "${{ github.event_name == 'workflow_run' && github.event.workflow_run.conclusion == 'success' && github.event.workflow_run.event == 'push' && github.event.workflow_run.head_branch == 'main' && github.event.workflow_run.head_repository.full_name == github.repository && github.event.workflow_run.head_sha == github.sha }}";
const DOCS_DEPLOY: &str = "${{ always() && !cancelled() && ((needs.reuse.result == 'success' && needs.build.result == 'skipped') || (needs.build.result == 'success' && needs.reuse.result == 'skipped')) }}";

/// Enforce trusted same-revision Pages reuse, with an independently checked manual fallback.
///
/// # Errors
/// Refuses missing quality gates, relaxed trust conditions or cross-run artifact ambiguity.
pub fn check_docs_artifacts(root: &Path) -> Result<()> {
    let docs = load(root, "docs.yml")?;
    let ci = load(root, "ci.yml")?;
    let jobs = &docs["jobs"];
    ensure!(
        names(&docs["on"]["workflow_run"]["workflows"]) == ["ci"].into_iter().collect(),
        "docs must observe only ci"
    );
    ensure!(
        names(&docs["on"]["workflow_run"]["branches"]) == ["main"].into_iter().collect(),
        "docs must observe only main"
    );
    ensure!(
        names(&docs["on"]["workflow_run"]["types"]) == ["completed"].into_iter().collect(),
        "docs must observe completed runs"
    );
    ensure!(
        jobs["reuse"]["if"] == DOCS_TRUST,
        "docs reuse must reject failed, fork/PR, wrong-branch and mismatched-SHA producers"
    );
    ensure!(
        jobs["build"]["if"]
            == "github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main'",
        "only manual docs builds may use the fallback"
    );
    ensure!(
        names(&jobs["deploy"]["needs"]) == ["reuse", "build"].into_iter().collect()
            && jobs["deploy"]["if"] == DOCS_DEPLOY,
        "deploy requires exactly one successful producer; missing/failed artifacts must not deploy"
    );
    for step in steps(&jobs["reuse"]) {
        let action = step["uses"]
            .as_str()
            .context("artifact reuse must not run code")?;
        ensure!(
            step["run"].is_null()
                && step["continue-on-error"].is_null()
                && step["if"].is_null()
                && (action.starts_with("actions/download-artifact@")
                    || action.starts_with("actions/upload-pages-artifact@")),
            "artifact reuse may only download and upload static files"
        );
    }
    ensure!(
        steps(&jobs["reuse"]).any(|s| s["uses"]
            .as_str()
            .is_some_and(|u| u.starts_with("actions/download-artifact@"))
            && s["with"]["name"] == "docs-site-${{ github.event.workflow_run.head_sha }}"
            && s["with"]["run-id"] == "${{ github.event.workflow_run.id }}"
            && s["with"]["repository"] == "${{ github.repository }}"
            && s["with"]["path"] == "docs/dist"),
        "docs artifact must bind repository, producer run and source SHA"
    );
    for job in [&ci["jobs"]["ci"], &jobs["build"]] {
        for command in [
            "pixi run docs build",
            "pixi run docs-frontmatter-check",
            "pixi run docs-link-check",
            "pixi run docs-a11y-check",
            "pixi run docs-snippet-check",
        ] {
            ensure!(
                steps(job).any(|s| s["run"] == command
                    && s["continue-on-error"].is_null()
                    && s["if"].is_null()),
                "missing unconditional docs quality gate: {command}"
            );
        }
    }
    let ci_steps: Vec<_> = steps(&ci["jobs"]["ci"]).collect();
    let upload = ci_steps
        .iter()
        .position(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/upload-artifact@"))
                && s["with"]["name"] == "docs-site-${{ github.sha }}"
        })
        .context("missing SHA-bound CI docs artifact")?;
    ensure!(
        ci_steps[upload]["if"] == "github.event_name == 'push' && github.ref == 'refs/heads/main'"
            && ci_steps[upload]["with"]["if-no-files-found"] == "error"
            && ci_steps[upload]["with"]["path"] == "docs/dist",
        "CI artifact upload must be main-push-only and fail for missing output"
    );
    for gate in [
        "pixi run docs-frontmatter-check",
        "pixi run docs-link-check",
        "pixi run docs-a11y-check",
        "pixi run docs-snippet-check",
    ] {
        ensure!(
            ci_steps[..upload].iter().any(|s| s["run"] == gate),
            "artifact upload must follow {gate}"
        );
    }
    println!(
        "docs artifact trust: only successful main-push same-revision CI artifacts or quality-checked manual builds can deploy"
    );
    Ok(())
}

/// Guard the single-suite CI route, shared Cargo build configuration and lint ordering.
///
/// # Errors
/// Refuses a filtered/optional test gate, competing Clippy invocations, release planner,
/// or cache/environment settings that diverge from the effective Pixi build.
pub fn check_ci_builds(root: &Path) -> Result<()> {
    check_ci_test_route(root)?;
    let read_json = |path: &str| -> Result<Value> {
        Ok(serde_json::from_str(&fs::read_to_string(root.join(path))?)?)
    };
    let package = read_json("package.json")?;
    ensure!(
        package["scripts"]["lint:plan"]
            == "cargo run -q -p pixi-sandbox -- plan --config .pixi-sandbox.toml --json",
        "lint planning must use the source-fresh debug binary"
    );
    ensure!(
        package["scripts"]["lint:rust"] == "turbo run lint --filter=@repo/rust",
        "Rust lint must have one workspace owner"
    );
    let rust = read_json("crates/package.json")?;
    ensure!(
        rust["scripts"]["lint:clippy"] == "cargo clippy --workspace --all-targets -- -D warnings"
            && rust["scripts"]["lint"]
                == "bun run lint:fmt && bun run lint:deps && bun run lint:clippy",
        "lint must run workspace/all-target Clippy once with warnings denied"
    );
    ensure!(
        rust["scripts"]["coverage"]
            == "cargo llvm-cov nextest --workspace --lcov --output-path lcov.info --ignore-filename-regex 'pixi-sandbox/src/github\\.rs$'",
        "coverage must execute the unfiltered workspace with only the socket adapter excluded"
    );
    for name in [
        "pixi-sandbox",
        "pixi-sandbox-core",
        "pixi-sandbox-git",
        "xtask",
    ] {
        ensure!(
            read_json(&format!("crates/{name}/package.json"))?["scripts"]["lint"].is_null(),
            "per-crate lint must not compete with workspace Clippy: {name}"
        );
    }
    let manifest: toml::Value = toml::from_str(&fs::read_to_string(root.join("pixi.toml"))?)?;
    ensure!(
        manifest["tasks"]["test-doc"].as_str() == Some("cargo test --doc --workspace"),
        "test-doc must run actual workspace doctests, not the ordinary suite"
    );
    let turbo = read_json("turbo.json")?;
    for (task, predecessor) in [
        ("//#lint:generated", "@repo/rust#lint"),
        ("//#lint:plan", "//#lint:generated"),
        ("//#lint:repo", "//#lint:plan"),
    ] {
        ensure!(
            names(&turbo["tasks"][task]["dependsOn"]) == [predecessor].into_iter().collect(),
            "{task} must follow {predecessor} without competing Cargo processes"
        );
    }
    let hashed = names(&turbo["globalEnv"]);
    for name in [
        "CARGO_INCREMENTAL",
        "CARGO_PROFILE_*",
        "RUSTFLAGS",
        "RUSTDOCFLAGS",
    ] {
        ensure!(hashed.contains(name), "Turbo must forward and hash {name}");
    }
    let repo_inputs = names(&turbo["tasks"]["//#lint:repo"]["inputs"]);
    ensure!(
        repo_inputs.contains("!crates/lcov.info")
            && repo_inputs.contains("!crates/**/.turbo/**")
            && !names(&turbo["tasks"]["//#lint:generated"]["inputs"]).contains("crates/xtask/**"),
        "generated reports and Turbo logs must not invalidate source lint caches"
    );
    println!(
        "CI build graph: one coverage suite, real doctests, payload proof, serialized workspace lint and matching Pixi cache"
    );
    Ok(())
}

fn check_ci_test_route(root: &Path) -> Result<()> {
    let ci = load(root, "ci.yml")?;
    let job = &ci["jobs"]["ci"];
    for command in ["coverage", "test-doc", "test-payload-roundtrip"] {
        let run = format!("pixi run {command}");
        let matching: Vec<_> = steps(job).filter(|step| step["run"] == run).collect();
        ensure!(
            matching.len() == 1,
            "CI requires exactly one {command} gate"
        );
        ensure!(
            matching[0]["if"] == "steps.changes.outputs.code != 'false'"
                && matching[0]["continue-on-error"].is_null(),
            "{command} must remain mandatory for code changes"
        );
    }
    ensure!(
        steps(job).all(|s| s["run"] != "pixi run test"),
        "CI must not duplicate coverage's ordinary workspace suite"
    );
    for name in [
        "CARGO_INCREMENTAL",
        "CARGO_PROFILE_DEV_DEBUG",
        "CARGO_PROFILE_TEST_DEBUG",
    ] {
        ensure!(ci["env"][name] == "0", "CI must pin {name}=0");
    }
    let rust_cache = steps(job)
        .find(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|action| action.starts_with("Swatinem/rust-cache@"))
        })
        .context("CI requires a Rust cache")?;
    ensure!(
        rust_cache["with"]["cmd-format"] == "pixi run --frozen -- {0}"
            && rust_cache["with"]["key"] == "${{ hashFiles('pixi.lock') }}",
        "Rust cache must fingerprint the effective Pixi compiler and lock"
    );
    // TASK-135: a docs-only PR still runs the xtask gates, which compile `xtask` cold without
    // a restored `target/` — measured at 61 s versus 0 s warm (run 40130578805). The cache
    // therefore restores on every run; only the save is gated to code changes, so a docs run
    // is a download when warm and a no-op when cold instead of a fresh compile.
    ensure!(
        rust_cache.get("if").is_none(),
        "Rust cache must restore unconditionally: gate its save with save-if, never the step"
    );
    ensure!(
        rust_cache["with"]["save-if"].as_str()
            == Some("${{ steps.changes.outputs.code != 'false' }}"),
        "Rust cache saves must stay gated to code changes so docs-only runs never rebuild it"
    );

    Ok(())
}

/// The output the airlock's `changes` job publishes: run everywhere except on a pull request
/// whose push range touches none of the watched paths.
const AIRLOCK_RUN_OUTPUT: &str =
    "${{ github.event_name != 'pull_request' || steps.filter.outputs.airlock != 'false' }}";

/// Guard the per-push airlock skip (TASK-135).
///
/// GitHub evaluates trigger-level `on.pull_request.paths` against the **whole PR diff**, so a
/// markdown-only `synchronize` push on a PR that previously touched `crates/**` still paid for
/// the full two-tier proof (measured ~13 wasted runner-minutes on PR #202). The trigger paths
/// stay — the ADR-29 main-push guard reads them — but a `changes` job now answers the per-push
/// question with a push-range [`dorny/paths-filter`] step, and both proof jobs gate on its
/// answer. This check fails closed on any drift between the two glob lists or a missing gate.
///
/// # Errors
/// Refuses a missing/weakened `changes` job, a filter whose globs diverge from the trigger
/// paths, a non-range `base`/`ref`, or proof jobs that stopped declaring the skip condition.
pub fn check_airlock_filter(root: &Path) -> Result<()> {
    let workflow = load(root, "airlock.yml")?;
    let jobs = workflow["jobs"]
        .as_object()
        .context("airlock.yml must declare jobs")?;

    // The trigger paths are the same policy the main-push guard text-parses.
    let trigger = names(&workflow["on"]["pull_request"]["paths"]);
    ensure!(
        !trigger.is_empty(),
        "airlock trigger must keep its pull_request.paths list; the ADR-29 guard reads it"
    );

    let changes = jobs
        .get("changes")
        .context("airlock.yml needs a `changes` job for the per-push filter (TASK-135)")?;
    ensure!(
        changes["outputs"]["run-airlock"].as_str() == Some(AIRLOCK_RUN_OUTPUT),
        "changes job must publish run-airlock defaulting to run outside pull requests"
    );
    let filter_step = steps(changes)
        .find(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|uses| uses.starts_with("dorny/paths-filter@"))
        })
        .context("changes job must use a pinned dorny/paths-filter step")?;
    ensure!(
        filter_step["id"].as_str() == Some("filter")
            && filter_step["if"].as_str() == Some("github.event_name == 'pull_request'"),
        "per-push filter runs only on pull_request events and keeps id `filter`"
    );
    let with = &filter_step["with"];
    ensure!(
        with["token"].as_str() == Some(""),
        "per-push filter must pass an empty token: with any token the action ignores base/ref \
         on pull_request events and diffs the whole PR via the API"
    );
    ensure!(
        with["predicate-quantifier"].as_str() == Some("some"),
        "per-push filter must use `some` semantics: one watched path touched must run the \
         proof, while `every` skips it on any mixed code-and-docs push"
    );
    let base = with["base"]
        .as_str()
        .context("per-push filter must name an explicit base")?;
    let reference = with["ref"]
        .as_str()
        .context("per-push filter must name an explicit ref")?;
    // On synchronize the range must be exactly this push (before..after); every other PR
    // activity falls back to base..head so the first proof still sees the whole diff.
    for (expression, range_end, fallback) in [
        (
            base,
            "github.event.before",
            "github.event.pull_request.base.sha",
        ),
        (
            reference,
            "github.event.after",
            "github.event.pull_request.head.sha",
        ),
    ] {
        ensure!(
            expression.contains("github.event.action == 'synchronize'")
                && expression.contains(range_end)
                && expression.contains(fallback),
            "per-push filter {expression:?} must use {range_end} on synchronize and {fallback} otherwise"
        );
    }
    // The block scalar is itself YAML mapping filter-name → glob list.
    let filters: Value = serde_yaml::from_str(
        with["filters"]
            .as_str()
            .context("per-push filter must declare its filters as a YAML block")?,
    )
    .context("parsing the per-push filters block")?;
    ensure!(
        names(&filters["airlock"]) == trigger,
        "per-push filter globs must match on.pull_request.paths exactly, path for path"
    );

    for job_name in ["plan", "proof"] {
        let job = jobs
            .get(job_name)
            .with_context(|| format!("airlock.yml must keep the {job_name} job"))?;
        ensure!(
            names(&job["needs"]).contains("changes"),
            "the {job_name} job must need `changes` so the skip decision gates it"
        );
        let condition = job["if"]
            .as_str()
            .with_context(|| format!("the {job_name} job must keep an explicit if"))?;
        ensure!(
            condition.contains("needs.changes.outputs.run-airlock != 'false'"),
            "the {job_name} job must skip when the push range touches no watched path"
        );
        ensure!(
            condition.contains("github.event.pull_request.draft == false"),
            "the {job_name} job must keep skipping draft pull requests"
        );
    }

    println!(
        "airlock per-push filter: {n} watched globs gate both proof jobs on the push range",
        n = trigger.len()
    );
    Ok(())
}
