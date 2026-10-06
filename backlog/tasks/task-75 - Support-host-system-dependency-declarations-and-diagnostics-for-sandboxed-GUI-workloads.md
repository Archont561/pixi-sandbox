---
id: TASK-75
title: >-
  Support host-system dependency declarations and diagnostics for sandboxed GUI
  workloads
status: To Do
assignee: []
created_date: '2026-10-06 11:19'
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
- [ ] #1 A reviewed config schema represents host requirements separately from Pixi/conda dependencies, with platform-scoped fields for packages, services, capabilities, libc/runtime floors, and headless GUI providers such as Xvfb; existing configs without the table continue to load unchanged.
- [ ] #2 Pack/manifest output preserves the resolved requirements in the transport so a restored branch remains self-describing without access to the source checkout, while existing transports with no host-requirement section remain restorable.
- [ ] #3 Doctor and/or restore run safe, read-only host probes where possible and report each requirement as satisfied, missing, unknown, or not applicable, with JSON output suitable for CI and human output that distinguishes conda-provided libraries from host packages, host services, display/GPU capabilities, and headless display requirements.
- [ ] #4 Linux diagnostics include actionable installation guidance for common Debian/Ubuntu and Fedora-style systems, including fontconfig/fonts, Xvfb/display, D-Bus, and GUI runtime examples; unsupported platforms or package managers degrade to explicit manual guidance rather than silent success.
- [ ] #5 Tempdir/fixture tests cover absent declarations, satisfied requirements, missing packages/services/capabilities, headless GUI/Xvfb requirements, JSON output, and backward compatibility without network access, host mutation, or this checkout as a fixture.
- [ ] #6 User documentation explains the host boundary, gives a Tauri/headless-GUI example, and warns that Pixi Sandbox does not grant host GPU/display access or install system services.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
