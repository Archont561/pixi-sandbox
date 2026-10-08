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
- [ ] #2 Pack/manifest output preserves the resolved requirements in the transport so a restored branch remains self-describing without access to the source checkout, while existing transports with no host-requirement section remain restorable.
- [ ] #3 Doctor and/or restore run safe, read-only host probes where possible and report each requirement as satisfied, missing, unknown, or not applicable, with JSON output suitable for CI and human output that distinguishes conda-provided libraries from host packages, host services, display/GPU capabilities, and headless display requirements.
- [ ] #4 Linux diagnostics include actionable installation guidance for common Debian/Ubuntu and Fedora-style systems, including fontconfig/fonts, Xvfb/display, D-Bus, and GUI runtime examples; unsupported platforms or package managers degrade to explicit manual guidance rather than silent success.
- [ ] #5 Tempdir/fixture tests cover absent declarations, satisfied requirements, missing packages/services/capabilities, headless GUI/Xvfb requirements, JSON output, and backward compatibility without network access, host mutation, or this checkout as a fixture.
- [ ] #6 User documentation explains the host boundary, gives a Tauri/headless-GUI example, and warns that Pixi Sandbox does not grant host GPU/display access or install system services.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08 — schema slice landed (AC#1). `[host_requirements]` is a new top-level table in `SandboxConfig` (`crates/pixi-sandbox-core/src/host_requirements.rs`, wired in `sandbox_config.rs`, decision D19): shared `libc`/`packages`/`services`/`capabilities`/`headless` declarations plus optional `[host_requirements.linux|osx|windows]` sections, resolved for a bundle's platform family by `HostRequirements::resolved_for` and surfaced through `SandboxConfig::host_requirements_for` (the entry point the manifest slice will call). Capabilities are a closed set (`display`, `gpu`); `libc` accepts `major.minor[.patch]` with an optional `>=` and is refused outside Linux; empty tables and empty family sections are errors. The config schema stays 1 — the table is additive, so existing configs load unchanged, and a bump could not improve the refusal an older binary emits (`load` deserializes before it validates, so 0.6.0 meets the table as an unknown field). The family fact lives on `Platform` (`Platform::host_family`, `HostFamily`) so a sixth platform cannot skip the question.

Spelling note: the table is `[host_requirements]` (snake_case), not the kebab spelling the issue's comment sketched, because every multi-word key in `.pixi-sandbox.toml` is snake_case and the same token then serves the TOML table, the Rust field, the manifest key and the JSON key (D19 records this).

Evidence: `cargo test -p pixi-sandbox-core --offline --test host_requirements` 27 passed (schema, resolution, refusals, libc grammar, two properties); full suite green at 710 passing / 1 skipped (was 681 / 1 skipped). Remaining slices: manifest section (AC#2), `doctor` probes + JSON (AC#3/#4), fixture coverage for probe outcomes (AC#5), user guide (AC#6). Nothing in the schema slice installs or probes host state; the table is validated and resolvable but not yet consumed.
<!-- SECTION:NOTES:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
