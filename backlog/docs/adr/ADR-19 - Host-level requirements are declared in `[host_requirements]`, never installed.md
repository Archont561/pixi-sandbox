---
id: ADR-19
title: "Host-level requirements are declared in `[host_requirements]`, never installed"
status: accepted
tags:
  - decision
---

# ADR-19 — Host-level requirements are declared in `[host_requirements]`, never installed

**Decision.** A project declares what its workloads need from the *machine* in a new
`[host_requirements]` table (issue #109, TASK-75). Five requirement kinds — `libc` (a
`major.minor[.patch]` minimum, optionally written `>=`), `packages`, `services`, `capabilities`
and `headless` providers — are shared by default and refined by optional
`[host_requirements.linux]`, `[host_requirements.osx]` and `[host_requirements.windows]`
sections. Resolution is total and one-way: shared entries always apply, a family section adds to
them (declaration order kept, duplicates dropped) and replaces the shared `libc`, and no section
can remove a shared entry — scoping to one family means declaring the requirement in that
section alone. The resolved set for a *bundle's* platform is what a transport will carry (the
manifest slice of TASK-75). Validation is fail-closed at load: capabilities are a closed set
(`display`, `gpu`), families are exactly the three named sections, `libc` is refused outside
Linux, entries must be single whitespace-free names listed once, and an empty table or empty
family section is an error rather than a declaration of nothing. The probes that classify a
host's state are `doctor`'s, they are read-only, and they report `satisfied` / `missing` /
`unknown` / `not applicable` while leaving the exit code alone until a caller asks for
enforcement with `--require-host-requirements` (which fails on `missing` only, never on
`unknown`).

**Why.** A transport restores a Pixi environment, not a machine: fonts, an X server, D-Bus, GPU
access and the C runtime live on the host, so a GUI workload can die *after* a restore that
verified perfectly, and the operator reads a stack trace inside a sandbox that reported success.
The declaration makes that boundary explicit and testable. Two naming and scoping choices carry
the weight. First, `host_requirements` rather than pixi's `system-requirements`: pixi's table
constrains the *solver* (virtual packages such as `__glibc` are selected against), while every
entry here is host state that pixi-sandbox never installs — sharing the name would blur exactly
the boundary the feature exists to draw. The snake-case spelling is not a preference: every
multi-word key in `.pixi-sandbox.toml` (`branch_prefix`, `max_blob_mib`, `push_paths`,
`cancel_in_progress`) is snake-case, so the table, the Rust field, the manifest key and the JSON
key are one token rather than three spellings. Second, family scoping rather than per-Pixi-
platform or per-bundle scoping: the probe backends and the facts themselves (a glibc floor, a
package manager) are OS-level, so `linux-64` and `linux-aarch64` must not be able to disagree
about Xvfb; per-bundle refinement stays additive if a project ever needs it. The report-only
default is the other load-bearing choice: the generated publisher runs `doctor` on a CI runner
that has no fonts, no display and no D-Bus, so a missing *host* requirement must never be able to
red-line a *publish*; enforcement stays a flag the operator of a given host can pass.

**Config schema stays 1.** The table is additive and optional, so nothing a consumer already
wrote changes and `init` keeps writing schema 1. A bump could not buy a better message for an
older binary either: `SandboxConfig::load` deserializes before it validates, so a 0.6.0 binary
meets `[host_requirements]` as an unknown field and refuses the file there, never reaching the
schema check. The bump stays reserved for a change that makes an *older* config mean something
different, per the constant's own doctrine.

**How it travels.** `pack --config <path>` resolves the table for `--platform`'s host family
before anything is created — an unreadable or invalid config refuses the run while no output
directory exists, with the other pre-flight guards — and records the result as
`manifest.host_requirements`, additive within schema 2 and omitted entirely when the project
declares nothing, so a transport without the section is byte-identical to what earlier releases
packed. The generated publisher passes the same config path to `pack` that it already passed to
`plan` and `doctor`, so a consumer's branch carries what its reviewed config declares with no
extra step. `doctor` reports the declared set (human line plus a `host_requirements` JSON
object) from the branch alone, and the branch's own README/AGENTS.md name it for a human reader.

**The probes, and who owns them.** `doctor` classifies each declared requirement against the
machine it runs on through a `HostProbe` seam (`host_requirements::HostProbe`): the CLI
implements it with read-only queries (`ldd --version` with a `getconf` fallback, `dpkg-query` /
`rpm` / `pacman`, `systemctl is-active`, `DISPLAY`/`WAYLAND_DISPLAY`, `/dev/dri`, a `PATH`
lookup that never executes what it finds), and everything that turns observations into a verdict
— the four-state classification, the per-distribution remedies, the conda-versus-host boundary —
is pure data in core, driven in tests by a scripted probe. The seam is what makes AC#5 possible
at all: a suite cannot assert "xvfb is missing" against a real host without lying about the
machine it runs on. Verdicts are reported and never enforced unless the caller passes
`--require-host-requirements`, which fails on `missing` alone — `unknown` must never punish a
machine that cannot be inspected (a container with no `systemctl`, a distribution with no
supported package manager), and a transport whose platform family differs from the host's is
`not applicable` and probes nothing at all. A requirement for another family is answered without
a single query, which is the one case where the honest answer is also the cheap one.

**Honest limits.** `display` and `gpu` are reportable, never grantable — a sandbox cannot hand a
process a GPU or a user's session, and `doctor` says so in the remedy rather than pretending to
fix it. `headless` providers are checked for existence on `PATH`, never launched, because
starting a display server to test whether one works would be a sandbox escaping its own
boundary. The service probe special-cases `dbus` (session address, system socket) because a
container usually has no systemd to ask; other services degrade to `unknown` there. Package
guidance is curated for the names in the issue (fontconfig, fonts, Xvfb, D-Bus, GTK, WebKit) on
apt/dnf/pacman and falls back to the declared name with an explicit "names differ between
distributions" caveat elsewhere. And the family list is closed at three: a project needing a
fourth family needs a decision, not a config key.

**What would change it.** Evidence that real projects need per-bundle declarations (a GUI bundle
and a docs bundle in one repository is the plausible case) would add an optional per-bundle
override; a non-glibc runtime floor worth enforcing (musl, a macOS deployment target) would
extend `libc` into a per-family floor rather than adding a second key; and a consumer hit by the
unknown-field refusal on an older binary would argue for the schema bump this decision declines.
