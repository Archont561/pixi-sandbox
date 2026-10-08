---
id: TASK-75
title: >-
  Support host-system dependency declarations and diagnostics for sandboxed GUI
  workloads
status: To Do
assignee: []
created_date: '2026-10-06 11:19'
updated_date: '2026-10-08'
labels:
  - host-requirements
  - gui
  - diagnostics
  - airlock
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/109'
priority: high
type: feature
ordinal: 75000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Pixi Sandbox restores Pixi/conda environments and vendored dependencies, but GUI workloads can still fail after a successful restore because host-level requirements are outside the transport boundary: fonts/fontconfig configuration, X11 or Xvfb, D-Bus, GPU/display access, libc floors, and other system services.

Add a first-class, reviewed way for a sandbox bundle to declare host-system requirements separately from packages restored into the Pixi environment. The declaration must be carried into the self-contained transport so `doctor` and/or `restore` can report missing host packages, services, and capabilities before users launch the application. The feature must keep the boundary explicit: Pixi Sandbox may diagnose and guide host setup, but it must not pretend to bundle display servers, GPU access, or OS services inside the Pixi environment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A reviewed config schema represents host requirements separately from Pixi/conda dependencies, with platform-scoped fields for packages, services, capabilities, libc/runtime floors, and headless GUI providers such as Xvfb; existing configs without the table continue to load unchanged.
- [x] #2 Pack/manifest output preserves the resolved requirements in the transport so a restored branch remains self-describing without access to the source checkout, while existing transports with no host-requirement section remain restorable.
- [ ] #3 Doctor and/or restore run safe, read-only host probes where possible and report each requirement as satisfied, missing, unknown, or not applicable, with JSON output suitable for CI and human output that distinguishes conda-provided libraries from host packages, host services, display/GPU capabilities, and headless display requirements.
- [ ] #4 Linux diagnostics include actionable installation guidance for common Debian/Ubuntu and Fedora-style systems, including fontconfig/fonts, Xvfb/display, D-Bus, and GUI runtime examples; unsupported platforms or package managers degrade to explicit manual guidance rather than silent success.
- [ ] #5 Tempdir/fixture tests cover absent declarations, satisfied requirements, missing packages/services/capabilities, headless GUI/Xvfb requirements, JSON output, and backward compatibility without network access, host mutation, or this checkout as a fixture.
- [ ] #6 User documentation explains the host boundary, gives a Tauri/headless-GUI example, and warns that Pixi Sandbox does not grant host GPU/display access or install system services.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08 — schema slice landed (AC#1). `[host_requirements]` is a new top-level table in `SandboxConfig` (`crates/pixi-sandbox-core/src/host_requirements.rs`, wired in `sandbox_config.rs`, decision D19): shared `libc`/`packages`/`services`/`capabilities`/`headless` declarations plus optional `[host_requirements.linux|osx|windows]` sections, resolved for a bundle's platform family by `HostRequirements::resolved_for` and surfaced through `SandboxConfig::host_requirements_for` (the entry point the manifest slice will call). Capabilities are a closed set (`display`, `gpu`); `libc` accepts `major.minor[.patch]` with an optional `>=` and is refused outside Linux; empty tables and empty family sections are errors. The config schema stays 1 — the table is additive, so existing configs load unchanged, and a bump could not improve the refusal an older binary emits (`load` deserializes before it validates, so 0.6.0 meets the table as an unknown field). The family fact lives on `Platform` (`Platform::host_family`, `HostFamily`) so a sixth platform cannot skip the question.

Spelling note: the table is `[host_requirements]` (snake_case), not the kebab spelling the issue's comment sketched, because every multi-word key in `.pixi-sandbox.toml` is snake_case and the same token then serves the TOML table, the Rust field, the manifest key and the JSON key (D19 records this).

2026-10-08 — manifest slice landed (AC#2, same day, second commit). `Manifest` gains `host_requirements: Option<HostRequirementSet>` (`crates/pixi-sandbox-core/src/manifest.rs`), additive within schema 2 and `skip_serializing_if = "Option::is_none"`, so a project that declares nothing packs the same bytes as before and a reader that predates the field ignores it (the manifest is deliberately not `deny_unknown_fields`; a test pins both directions). `pack --config <path>` resolves the set for `--platform`'s host family before anything is created — an unreadable or invalid config refuses the run while no output directory exists, matching the other pre-flight guards — and writes it into the manifest; the generated publisher now passes the same config path to `pack` that it already passed to `plan` and `doctor` (both runner dialects, golden render regenerated). The branch README and AGENTS.md gain a paragraph/bullet naming the requirements, and `doctor` prints the declared set (human + a `host_requirements` JSON object) with an explicit hint that this build does not probe the host yet.

Evidence: `cargo test -p pixi-sandbox-core --test manifest` 28 passed (wire round-trip, absent-key default, unknown-key tolerance, summary); `cargo test -p pixi-sandbox --bin pixi-sandbox host_requirements` 5 passed (resolution: no config, no table, per-family, family with nothing of its own, invalid config refused with the chain); `--test generated_workflow` 44 passed (both legs pass the config; seven config references pinned); `--test e2e doctor_reports_the_host_requirements_the_transport_declares` passed (fixture transport, section injected: JSON read-back, still verifies green, human line; pristine fixture reports no section). Suite 710 -> 720 passing / 1 skipped (core 176 -> 179, sandbox 388 -> 395, xtask 125, git 21). Remaining: probes + remedies (AC#3/#4), probe fixtures (AC#5), user guide (AC#6).

Evidence: `cargo test -p pixi-sandbox-core --offline --test host_requirements` 27 passed (schema, resolution, refusals, libc grammar, two properties); full suite green at 710 passing / 1 skipped (was 681 / 1 skipped). Remaining slices: manifest section (AC#2), `doctor` probes + JSON (AC#3/#4), fixture coverage for probe outcomes (AC#5), user guide (AC#6). Nothing in the schema slice installs or probes host state; the table is validated and resolvable but not yet consumed.
<!-- SECTION:NOTES:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
