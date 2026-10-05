---
id: doc-10
title: Release-Published Pixi Sandbox Starter Template Specification
type: specification
created_date: '2026-10-05'
updated_date: '2026-10-05'
tags:
  - template
  - onboarding
  - release
  - ci
  - consumer
---

# Release-Published Pixi Sandbox Starter Template

## Goal

Provide a maintained GitHub template repository that lets a developer create a new Pixi project
with Pixi Sandbox already configured for the current released version. The connected-host path
must be ready as soon as the developer selects **Use this template** (or clones the template): no
manual `pixi-sandbox init`, workflow copying, release-version selection, sandbox configuration,
or host-specific CI setup is required before development can begin.

The template is a consumer artifact, not a second implementation of pixi-sandbox. Its owned setup
is generated from, and pinned to, one published pixi-sandbox release.

## User experience contract

For a template revision associated with release `vX.Y.Z`, a user can:

1. select **Use this template** and create a repository under their own GitHub owner;
2. clone it and run the documented Pixi development command; and
3. push to the documented default branch to run the included connected publisher.

The created repository contains a valid `pixi.toml`/lockfile, a reviewed
`pixi-sandbox.toml` policy, the generated publisher workflow, and concise onboarding
instructions. The publisher uses the repository's `GITHUB_TOKEN` with explicitly scoped
permissions; it must not require an Archont561 token, a manually provisioned host, or a copied
workflow. A developer who needs an offline restore launcher may opt into it later with
`pixi-sandbox init`; it is not a prerequisite for connected development.

## Template ownership and contents

- Publish one dedicated repository, initially named `Archont561/pixi-sandbox-starter`, marked as
a GitHub template. The final repository name is an implementation detail, but it must have one
canonical public URL documented from the main project.
- Keep the starter language-neutral: it supplies the Pixi/Sandbox scaffold and a small documented
  development task, not a framework-specific application or a second project generator.
- Treat the following as generated/managed template files: the pinned pixi-sandbox version
  manifest, `pixi.toml`, the lockfile, `pixi-sandbox.toml`, and
  `.github/workflows/publish-sandbox.yml`. The template README may contain maintained prose but
  must state its corresponding release and template revision.
- The committed publisher must be a normal consumer publisher produced by the released CLI. It
  keeps the exact production version pin and checksum verification already required of generated
  consumer workflows; it must never resolve `latest` while packing or publishing.
- The template must include an appropriate `.gitignore` and must not commit `.pixi/`,
  `.pixi-sandbox/`, packages, credentials, release tokens, or a prebuilt transport branch.

## Release-to-template publication contract

A template update begins only after the source release workflow has successfully:

1. published the prefix.dev package variants;
2. created the GitHub Release for tag `vX.Y.Z`; and
3. uploaded and verified the release assets and `SHA256SUMS` for that tag.

The release job explicitly dispatches the template publication; it must not rely solely on a
`release.published` trigger because GitHub suppresses follow-up events created with
`GITHUB_TOKEN`. The dispatched payload includes only the exact tag and immutable release commit.
The receiving job independently verifies that the tag names a published release in
`Archont561/pixi-sandbox`, that it resolves to the supplied commit, and that the required release
assets/checksum manifest exist before it changes the template.

The template publisher derives every managed file from that verified release, refreshes the Pixi
lockfile, regenerates the publisher using the downloaded checksum-verified standalone binary,
and records the tag, source commit, and template commit in durable release evidence. It is
idempotent: rerunning the same verified tag produces no content change; a conflicting manually
edited managed file fails rather than being silently overwritten.

## Credentials and safety

Cross-repository publication uses a narrowly installed, owner-operated GitHub App installation
token (or an equivalent short-lived credential), never a personal access token. It has access only
to the source and starter repositories and only the `contents`, `workflows`, and pull-request
capabilities needed for the update. The ordinary consumer template contains none of these
credentials.

The automation may update only the canonical starter repository; it must never push to template
derivatives, forks, or repositories created by users. It must not force-push or rewrite history.
A failed generation, lock, validation, or push leaves the last known-good template revision
unchanged. Failure is visible in a GitHub step summary and bounded artifact with the release tag,
source commit, intended managed-file diff, and a remediation; no token or secret may be emitted.

## Validation and evidence

Before publishing a new starter revision, CI must prove in a clean temporary directory that:

1. the managed files pin exactly `vX.Y.Z` and no production setup references `latest`;
2. the released standalone binary and `SHA256SUMS` verify successfully and can regenerate the
   template without source-checkout dependencies;
3. a second generation is byte-identical and `init --check` reports no drift;
4. the generated publisher is actionlint-clean and retains checksum verification, native planning,
   doctor-before-publish ordering, and explicit `GITHUB_TOKEN` permissions;
5. a fresh clone installs the locked Pixi environment and runs the documented development task;
   and
6. an isolated local/bare-remote consumer lifecycle can pack, doctor, publish, fetch, and restore
   the template's default sandbox configuration.

After validation, tag the template revision with a namespaced immutable tag such as
`pixi-sandbox-vX.Y.Z`, or create an equivalent starter release. The main project release notes and
the starter README must link to each other so a user can determine exactly which pixi-sandbox
release created their scaffold.

## Non-goals

- Updating repositories that users previously created from the template. Those remain consumers
  of the existing reviewed upgrade lane.
- Replacing the generated consumer publisher, the release asset checksum contract, or the
  airlock transport protocol.
- Providing a framework-specific application starter, managed hosting service, or secrets.
- Floating dependency upgrades: a new release results in a new template revision, not a mutable
  template that silently changes an existing project.

## Rollout

1. Create and validate the starter repository manually once, then mark it as a GitHub template.
2. Add the release-to-template publisher and its fixture/connected proof.
3. Dispatch it for a known release and inspect the immutable starter evidence.
4. Add the starter URL and three-step quickstart to the project documentation only after the
   automated publication is proven.
