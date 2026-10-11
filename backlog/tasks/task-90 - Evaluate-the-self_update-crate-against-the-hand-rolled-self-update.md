---
id: TASK-90
title: Evaluate the self_update crate against the hand-rolled self-update
status: To Do
assignee: []
created_date: '2026-10-11 11:51'
labels:
  - spike
  - self-update
  - dependencies
dependencies: []
references:
  - 'https://docs.rs/self_update/latest/self_update/'
  - Cargo.lock
  - crates/pixi-sandbox/src/self_update/ownership.rs
  - crates/pixi-sandbox/src/self_update/replace.rs
  - crates/pixi-sandbox/tests/self_update.rs
  - 'backlog/docs/spikes/self-update/doc-14 - Self-Update-Crate-Evaluation.md'
priority: medium
type: spike
ordinal: 90000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decide whether `pixi-sandbox` keeps its hand-rolled updater or adopts the `self_update` crate for
the download → verify → replace engine. Ours is `crates/pixi-sandbox/src/self_update/*` (889 lines
of production code: `resolver`, `assets`, `checksums`, `ownership`, `replace`, plus the `mod.rs`
orchestration) judged by 94 tests in `crates/pixi-sandbox/tests/self_update*.rs` (1,710 lines across
seven files). This is an **evaluation spike, then a decision** — the deliverable is a document, not
a migration. Write it as the next `backlog/docs/spikes/` document:
`backlog/docs/spikes/self-update/doc-14 - Self-Update-Crate-Evaluation.md` (doc-13 is the highest
document today). Do **not** add a `.knowledge/*-spike.md` tombstone: those files are pointers *into*
`backlog/docs/spikes/` (`.knowledge/rattler-spike.md` → doc-5), not a second home for a live spike.

**Scope boundary, measured:** the airlock cannot reach crates.io (`crates.io`, `index.crates.io`,
`static.crates.io` all refuse; the crate is not in the Cargo cache). The dependency can therefore be
added on paper and reasoned about, but never built or tested here. Reading the crate's docs over HTTP
**does** work — the fetch path is not the bash egress allowlist — and that asymmetry is exactly what
produced a wrong first assessment last session: a blocked `curl` for the crate is not evidence about
the crate's contents. Say what was read and from where.

**Already measured this session. Carry these into doc-14; do not re-derive them:**

- **It builds on the `ureq` this repo already has.** `ureq` is a first-class optional client here
  (`default-features = false, features = ["ureq", "rustls", "github"]`); the crate requires
  `ureq ^3.0.6` and `Cargo.lock` carries `3.4.2`. Against today's `Cargo.lock` (272 crates) the only
  missing non-optional dependencies are `self-replace` and `urlencoding`; `sha2 0.11.0` is already
  present at exactly the version the `checksums` feature wants. No reqwest, no hyper, no tokio — if
  `progress-bar` and `async` stay off.
- **Bare binaries are supported.** `ArchiveKind::Plain(Option<Compression>)` is documented as "a bare
  file, optionally compressed (e.g. a plain binary…)" and is *not* feature-gated, so the five
  `pixi-sandbox-<triple>` release assets need no `archive-*` feature.
- **Its checksum path matches this repo's contract.** `checksum_from_asset("SHA256SUMS")` fetches the
  sums asset *before* the artifact, accepts coreutils text/binary modes, leading paths, the BSD tag
  form and `#` comments, and treats a missing asset *or* a missing entry as
  `Error::ChecksumSourceInvalid` rather than a skipped check.
- **It has a test seam.** `ReleaseSource` is a public trait driven through `backends::custom`, and
  `.http_client(Arc<dyn HttpClient>)` exists for test doubles — so the 94 tests above can be
  re-pointed rather than deleted.
- **It is ahead of us on transport:** `Error::RateLimited` distinct from `Unauthorized` (correctly
  classifying GitHub's secondary 403 + `Retry-After`, clamped to 24 h), `rate_limit_delay()`,
  `check_install_path_writable` preflight, `Error::InstallPathNotWritable`, proxies,
  timeouts/retries, `UpdateCheckGuard`, `api_base_url`, `restart()`. We have no rate-limit handling
  at all, and that gap is a legitimate argument for adoption.
- **Footgun:** the defaults are interactive (`show_output: true`, `no_confirm: false`) and `update()`
  blocks on a `yes/no` prompt — every call site would need `.no_confirm(true).show_output(false)` or
  the self-update silently hangs a CI lane waiting on a TTY that is not there.

**Two properties must survive any adoption, and both are ours, not the crate's:**

1. **Refusal before download.** `self_update::ownership::classify` (145 lines) rejects a managed
   launcher, a `pixi global` trampoline, a `conda-meta/` prefix and a manifest-verified transport
   tool *before a byte moves*. The crate's `verify_archive`/`verify_binary` hooks run **after** the
   download, so `ownership.rs` stays and is called before `update()` — the crate can never be the
   thing that decides whether we were allowed to touch the file.
2. **`ReplaceStrategy` as a parameter, not a `cfg!` read**, so both the Unix and the Windows dances
   are exercised on any machine — 24 tests plus the `replace-running-image` job on `windows-latest`.
   The crate delegates to `self-replace`; whether it sweeps orphaned corpses the way `windows_swap`
   does (stale `.old-*` siblings reaped at the start of every run) is **unverified**.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 `backlog/docs/spikes/self-update/doc-14 - Self-Update-Crate-Evaluation.md` exists, states one outcome — adopt, adopt-partially (which halves), or reject — and records that no dependency was added to any `Cargo.toml` or `Cargo.lock` in this task.
- [ ] #2 The `windows_swap` corpse question is answered from the crate's published sources or docs, naming the file and line (or doc URL); the two behaviours are addressed separately — replacing a running image, and reaping stale `.old-*` siblings — and anything not resolvable offline is labelled unverified with the missing proof named.
- [ ] #3 Both must-survive properties are settled in writing: the refusal ladder (`ownership::classify`) runs before any download byte moves, and `ReplaceStrategy` stays an explicit parameter with its 24 tests and the Windows `replace-running-image` job intact — or the spike says exactly what is lost and why that is acceptable.
- [ ] #4 The test-surface mapping is recorded per file: which of the 94 tests in `tests/self_update*.rs` (1,710 lines) get re-pointed at the crate's `ReleaseSource`/`.http_client(Arc<dyn HttpClient>)` seams, which are replaced by crate behaviour, and which are dropped — each dropped one with its reason, so the count moving is explained rather than discovered.
- [ ] #5 The dependency-surface delta is recorded exactly (feature set, crates gained, crates that stay out) together with the airlock limitation: adoption cannot be built or tested here, so the decision must carry the connected-host step (a lockfile refresh and one real build) before any migration task starts.
- [ ] #6 The outcome is recorded where a decision belongs — a `backlog/decisions/decision-<next>` file when the answer is to adopt or reject outright, or an explicit deferral in doc-14 with the trigger that reopens it — and no production code or dependency is migrated in this task.
<!-- AC:END -->

## Definition of Done

<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-11 11:51
<!-- COMMENTS:END -->
