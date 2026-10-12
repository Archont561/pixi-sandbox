# doc-18 — Documentation refactor plan: roles, lifecycle phases, and lifecycle-split references

**Source:** pixi-sandbox documentation audit, reviewed 2026-10-12 (captured public pages, structured
sitemap metadata, desktop screenshots). Benchmarks: Pixi docs (210 unique current-version pages) and
Playwright docs (148 current English stable pages, 74 API-reference pages).

**Fair-comparison rule:** Pixi and Playwright have much broader product scopes. Their page counts are
not a target. The relevant comparison is whether pixi-sandbox gives each of its distinct audiences a
clear, safe path from installation to verified offline restore. The right end state is roughly 18–25
focused pages, not Pixi's 210-page hierarchy.

**Task mapping** — each work item in the audit's prioritized plan is a backlog task:

| Audit item | Task |
|---|---|
| P0 · Create "First verified offline bundle" | task-97 |
| P0 · Role-based entry points on the homepage | task-98 |
| P0 · Contextual machine/network chips | task-101 |
| P0 · Glossary and security boundaries | task-99 |
| P0 · Troubleshooting / incident response | task-100 |
| P1 · Split CLI reference by lifecycle | task-102 |
| P1 · Split configuration reference | task-103 |
| P1 · Releases, compatibility, upgrading | task-104 |
| P1 · Upgrade migration guide | task-105 |
| P1 · Printable operator checklist | task-106 |
| P2 · Replace homepage "Icons & Stack" | task-107 |
| P2 · Wider reference layout | task-108 |
| P2 · Standardize guide templates | task-109 |
| P2 · Snippet/link/accessibility checks | task-110 |
| Umbrella · New IA (sidebar, "How it works", "Choose your role") | task-96 |

The audit body follows verbatim — it is the evidence and the plan.

---

# pixi-sandbox documentation audit

## Benchmarks: Pixi and Playwright documentation

**Reviewed:** 12 October 2026
**Evidence:** captured public pages, structured sitemap metadata, and desktop screenshots

| Documentation set | Captured scope | What it represents in this audit |
|---|---:|---|
| **pixi-sandbox** | 13 pages | The product being evaluated: an offline transport tool for Pixi environments and Cargo crates. |
| **Pixi** | 210 unique current-version pages from 212 sitemap entries | The domain benchmark: a mature documentation system for the underlying package/environment manager. |
| **Playwright** | 148 current English stable pages, including 74 API-reference pages | The documentation-product benchmark: task guidance, reference organization, operational coverage, and lifecycle trust. |

> **Fair-comparison rule:** Pixi and Playwright have much broader product scopes. Their page counts are not a target. The relevant comparison is whether pixi-sandbox gives each of its distinct audiences a clear, safe path from installation to verified offline restore.

---

## Executive assessment

**pixi-sandbox has the strongest visual identity of the three home experiences and unusually good operational specificity.** The documentation understands the actual problem: a connected side, an air-gapped/airlock side, Git transport, verification, Cargo vendoring, host limitations, and CI publishing. It is materially more concrete about offline failure boundaries than many mature developer-tool sites.

The gap is not subject-matter knowledge. It is **documentation orchestration**:

- the docs serve several roles—project owner, CI maintainer, airlock operator, and platform/security reviewer—but do not make those role-based paths primary;
- the compact 13-page set concentrates too much operational detail into the CLI and configuration references;
- the first journey is not yet framed as a deterministic two-sided deployment exercise with clear success evidence;
- version, release, migration, security-boundary, and troubleshooting surfaces are weaker than the mature baselines; and
- a few homepage elements compete with the primary operational story.

### Recommended strategy

Preserve the branded, diagram-led visual foundation. Refactor the content system around **roles and lifecycle phases**:

1. install and verify on the connected host;
2. configure and publish a transport in CI;
3. transfer and restore at the airlock;
4. verify offline behavior and investigate failures; and
5. understand the exact security and host boundaries.

The right end state is roughly **18–25 focused pages**, not Pixi's 210-page hierarchy.

---

# What pixi-sandbox already does exceptionally well

## 1. The product story is specific and differentiated

The homepage immediately says what the product does: it packs Pixi environments and vendored Cargo crates, publishes to an orphan branch, and restores on a no-network machine. It explains the mechanism rather than using generic "reproducible environments" language.

The visual flow from **Pixi project → orphan branch → airlock** is effective. It makes a difficult deployment topology understandable in one screen.

**Keep:**

- the connected-side/orphan-branch/airlock diagram;
- explicit zero-network language;
- feature claims tied to mechanics: Git-native transport, sharding, SHA-256 verification, Cargo vendoring, static Rust CLI;
- clear non-goals such as the host-requirements boundary.

## 2. Operator-facing instructions are unusually concrete

The **Restore on the airlock** guide is strong. It tells the operator to stop if anything reaches the network, shows how to transfer the branch, verifies before writing, explains durable diagnostic logs, and spells out what the restore command does in order.

The **Using in your project** guide is also operationally credible: it distinguishes the connected host from the airlock, names generated files, explains the branch requirement, gives platform-specific launchers, and clarifies that pushing/workflow completion/air-gap transfer are deployment operations rather than hidden tool behavior.

This is a major strength. It aligns with Playwright's best operational guides—such as configuration, running/debugging, CI, traces, and release notes—where users receive a safe workflow rather than only API prose.

## 3. Safety and limits are documented instead of hand-waved

The docs cover verification, immutable diagnostic logs, credential redaction, content-addressed transport, bundle budgets, GitHub blob limits, machine capability limits, and the distinction between a Pixi environment and a complete host machine.

The **GUI and host requirements** guide is especially valuable because it states what the tool will not do. That is a professional trust signal.

## 4. Installation paths reflect real deployment constraints

The installation guide offers Cargo, canonical Pixi Global, release binary/transport bootstrap, and source paths. It calls out the canonical channel and documents the old package-specific channel for upgrades from earlier versions.

This is materially better than a generic "install this package" page because it understands that a tool for air-gapped use must support verified bootstrap paths.

## 5. The UI is already polished and purposeful

The visual system is stronger and more product-specific than a generic documentation theme:

- a branded hero and icon;
- a clear transport-flow illustration;
- feature cards;
- status/badge strip;
- callouts and diagram cards;
- consistent Starlight navigation, search, ToC, edit-page link, and next/previous controls.

The documentation feels like a serious product rather than a repository README rendered as a site.

---

# Benchmark comparison

## Comparison matrix

| Dimension | pixi-sandbox | Pixi docs | Playwright docs | Assessment for pixi-sandbox |
|---|---|---|---|---|
| **Value proposition** | Very strong: the airlock/offline transport problem is explicit and visually demonstrated. | Strong: package-management value is broad but clear. | Strong: browser automation/testing purpose is immediately clear. | Keep current specificity; do not dilute it with generic packaging language. |
| **First-success path** | Good commands, but it spans connected host, CI, transfer, and airlock roles. | Strong: Installation → First workspace → Basic usage presents a linear, runnable progression. | Strong: installation and first test have clear progressive steps. | Reframe as a two-sided, verified success path with artifacts and expected results. |
| **Audience/role guidance** | Present in prose, but secondary. | Broad navigation separates tutorials, concepts, build, deployment, integration, reference, and misc. | Strong task/product segmentation: getting started, configuration, fixtures, operational guides, reference. | Make Project Owner, CI Maintainer, Airlock Operator, and Security Reviewer entry paths visible. |
| **Concept model** | Strong high-level transport diagram and concrete vocabulary. | Extensive concepts/tutorials explain workspace, lock file, environments, channels, and tasks. | Strong conceptual guides such as fixtures, configuration, parallelism, and assertions. | Add a first-class glossary and lifecycle/architecture page so terms are defined once. |
| **Reference findability** | Broad but dense: 22k-character CLI reference and 14k-character configuration reference. | Granular command reference pages for CLI subcommands and focused config/reference pages. | 74 deep-linkable API pages plus focused configuration/reference sections. | Split large reference pages by lifecycle command/category, not into 100+ micro-pages. |
| **Operational safety** | Excellent: offline verification, logs, branch transfer, budgets, host limits. | Strong technical reference and deployment guidance, but not primarily an air-gap product. | Strong testing operational docs: CI, debugging, retries, reporters, traces. | Preserve this advantage; elevate threat boundaries and incident recovery into primary docs. |
| **Migration / upgrades** | One retired Actions migration page and an installation note for older channels. | Current-version documentation and changelog. | Versioned docs, release notes, migration guides. | Add a release/upgrading page and a visible compatibility/migration policy. |
| **Visual identity** | Strongest product branding and system diagram of the three. | Functional, conservative, versioned manual style. | Functional, consistent, mature documentation style. | Retain the identity; remove distractions and use visual emphasis for role decisions. |
| **Feedback / trust signals** | Has an Edit page link; no clear release/version surface in the reviewed sitemap. | Visible version selector, changelog, GitHub link, and rich current docs structure. | Version context, release notes, community and issue pathways. | Add documentation version, compatibility matrix, changelog/release link, and issue-reporting link. |

---

# Content critique

## 1. The core journey needs to become a role-aware deployment journey

pixi-sandbox is not a single-machine CLI. Its real workflow involves at least two environments and often a third CI actor:

- **Connected project owner:** configures the bundle and commits generated assets.
- **CI maintainer:** publishes/reviews the transport branch and failures.
- **Airlock operator:** verifies local Git objects and restores without network access.
- **Security/platform reviewer:** evaluates verification, host requirements, budgets, and policy.

The current documentation contains these details, but users must infer which pages apply to them. The homepage's single **Get Started** call to action and the Quickstart do not sufficiently distinguish roles.

### Refactor

Immediately after the homepage hero, add four entry cards:

| I am… | Primary outcome | First page |
|---|---|---|
| **A project owner** | Configure and create my first transport bundle. | First offline bundle |
| **A CI maintainer** | Publish safely and keep workflow policy current. | CI publishing |
| **An airlock operator** | Verify and restore locally with zero network. | Restore on the airlock |
| **A security/platform reviewer** | Understand guarantees, constraints, and host requirements. | Security and host boundaries |

This is the most important IA change because it converts an impressive technical description into a directed user journey.

## 2. Quickstart should prove the complete happy path, not merely initialize

The homepage Quick Start currently shows:

```bash
pixi global install … pixi-sandbox
pixi-sandbox init
```

That gets the CLI installed and creates configuration, but it does not demonstrate the defining promise: **a verified offline restore**.

The separate Quickstart and Using in your project pages contain more of the story, but a new reader has to assemble connected-side actions, CI publishing, Git transfer, and airlock verification themselves.

### Refactor: "First verified offline bundle"

Use a concise four-stage path:

1. **Connected host:** install CLI and run `pixi-sandbox init`.
2. **Configure:** choose one minimal environment/platform bundle.
3. **Publish:** run or trigger the generated workflow; show the expected sandbox branch name.
4. **Airlock:** clone/import a bundle, run `./restore.sh`, then prove the expected command works with `--offline` / `--frozen`.

For each stage, show:

- who performs it;
- machine/network condition;
- exact command;
- expected artifact or output;
- a failure link.

A good visual treatment is a horizontal sequence matching the existing hero diagram, with labels such as **Connected**, **CI**, **Transfer**, and **Airlock**.

### Acceptance criterion

A new user can complete a minimal transport with one documented Pixi project, identify the expected `sandbox/<bundle>-<platform>` branch, restore on a simulated offline machine, and verify a documented command succeeds without network access.

## 3. The product's vocabulary needs one canonical home

The documentation relies on important terms:

- connected host;
- airlock;
- transport;
- bundle;
- orphan branch;
- sandbox branch;
- pack, doctor, publish, restore, unpack;
- host requirements;
- tools lock / embedded binary;
- shard and blob budget.

The docs explain these terms contextually, but not in one shared model. The risk is that a user reads `restore` before `Using in your project`, or a security reviewer lands in the CLI Reference, and has to infer terms from implementation detail.

### Refactor

Create two short foundational pages:

1. **How pixi-sandbox works**
   - one lifecycle diagram;
   - what exists on each machine;
   - what crosses the air gap;
   - what is verified;
   - which files are generated or committed.

2. **Glossary and security boundaries**
   - concise definitions;
   - what each guarantee means;
   - what the tool does **not** provide;
   - links to host requirements, budgets, and restore guidance.

Pixi provides a relevant model: it separates onboarding from concepts such as environments, lock files, channels, platforms, and package specifications. Playwright does the same with fixtures, configuration, and execution concepts.

## 4. Operational/security information deserves first-class navigation

The current documentation already has unusually valuable security-adjacent material:

- SHA-256 verification;
- zero-network claims;
- content-addressed Git transport;
- blob-size limits/sharding;
- credential redaction;
- immutable diagnostic logs;
- host machine limitations;
- Cargo vendoring.

Some of this is buried in CLI/configuration details. For a tool used to carry environments across an air gap, this is not secondary reference information; it is a key adoption and approval decision.

### Refactor

Create a visible **Security and operations** group:

```text
Security and operations
  Verification guarantees and threat boundaries
  Host and GUI requirements
  Transport sizing, sharding, and budgets
  Incident diagnostics and troubleshooting
  Restore checklist for operators
```

The current **Restore on the airlock** page can become the center of that group. It should also have a concise printable/operator checklist at the top, followed by the detailed explanation.

## 5. Reference content is accurate but too concentrated

The CLI Reference covers global usage, diagnostics, `pack`, `doctor`, `publish`, `restore`, `unpack`, `init`, `plan`, `tools`, and `self-update` in one long page. The Configuration Reference contains schema, budgets, runners, bundle shape, workflow policy, host requirements, validation, and examples in one other long page.

This is efficient for maintainers but harder for users to scan, link, review, and update. The right-side table of contents helps but cannot replace strong page boundaries.

### Do not copy Pixi's command-page count

Pixi has a large CLI and therefore maintains many command-reference pages. pixi-sandbox should avoid that maintenance burden. A practical split is:

| New page | Contents |
|---|---|
| **CLI overview and shared diagnostics** | invocation forms, `-v`/`-vv`, logs, redaction, exit/failure behavior |
| **Create and verify a transport** | `init`, `plan`, `pack`, `doctor`, sizes/budgets |
| **Publish and rotate transport branches** | `publish`, branch naming, retention, CI workflow behavior |
| **Restore and inspect a transport** | `restore`, `unpack`, offline checks, operator failure recovery |
| **Tools and self-update** | embedded tools, lock/pins, `tools`, `self-update` |
| **Configuration overview** | file selection, schema/version, minimal config |
| **Bundle, CI, and host policy** | `[[bundle]]`, budgets/runners/workflow, host requirements |

Each reference page should start with **Use this when…**, provide the minimum command/configuration, then table details, safety implications, failure modes, and related guides.

## 6. Migration and release trust need to be more visible

The site includes **Migrating from composite Actions** and documents a channel change for versions up to `0.4.3`. Those are valuable, but readers cannot easily discover a broader policy for:

- supported pixi-sandbox versions;
- compatible Pixi versions;
- Rust/Cargo/operating-system support;
- generated workflow compatibility;
- retired interfaces and upgrade paths;
- security fixes and notable behavior changes.

Both benchmark sites give users visible lifecycle signals: Pixi has a versioned docs experience and changelog; Playwright has release notes, versioned docs, and migration guidance.

### Refactor

Add a **Releases, compatibility, and upgrading** page with:

- current release and documentation version;
- support matrix: OS, platform, Pixi requirement, Git requirement, Rust requirement where relevant;
- upgrade checklist;
- generated-workflow upgrade policy;
- migration table for retired composite Actions/reusable publisher paths;
- release notes or a clearly linked GitHub releases feed.

Add a compact version/release badge in the documentation header or footer. A full version switcher is only needed when multiple documentation versions are actively maintained.

## 7. Troubleshooting should be organized by symptoms, not only commands

The CLI Reference is strong on diagnostics, but users under pressure will search for a symptom:

- "restore attempted network access";
- "doctor found a digest mismatch";
- "sandbox branch is not in the transfer bundle";
- "restore cannot execute embedded binary";
- "Git object is larger than GitHub's limit";
- "host lacks GUI/display/font capability";
- "CI publish is over budget";
- "vendored Cargo build still fetches."

### New page: "Troubleshooting and incident response"

For each scenario include:

1. symptom/error text;
2. likely cause;
3. safe immediate action;
4. evidence to collect (`--log-file`, `doctor --verify`, branch/ref checks);
5. recovery action;
6. escalation/reproducibility details.

This adapts Playwright's running/debugging model to pixi-sandbox's safety-sensitive domain.

---

# UI and visual-design critique

## What to keep

### The home page has stronger visual storytelling than the benchmarks

The homepage does more than state features. The project/orphan-branch/airlock flow makes the core architecture visual, and the feature cards translate technical properties into benefits. Pixi and Playwright are more conservative and functional; pixi-sandbox has an opportunity to remain the more memorable experience.

### The existing documentation shell is effective

The common technical-documentation shell—search, left navigation, article, right-side ToC, edit-page link, and previous/next buttons—is a good choice. There is no need to replace Starlight or reproduce Pixi's or Playwright's styling.

### Status and safety callouts are a good fit

The visual system already uses callouts and cards. Use these for operational distinctions, not decoration:

- **Connected host**
- **Airlock only**
- **Writes files**
- **Verification-only / writes nothing**
- **Needs network**
- **Must be offline**
- **Experimental / retired / compatibility path**

Use text and icons together; do not make color the only signal.

## What should change

| Issue | Impact | UI refactor |
|---|---|---|
| **The homepage's badge strip, six feature cards, Quick Start, and "Icons & Stack" section compete for attention.** | Users can miss the next operational step after understanding the hero. | Keep the transport diagram; replace the generic icon-stack section with role cards or "Choose your path." Move design-system/icon details to contributor docs. |
| **The Quick Start is visually short but conceptually incomplete.** | The most important promise—verified offline restore—is not demonstrated above the fold. | Show a four-stage connected → CI → transfer → airlock sequence with one verified outcome. |
| **CLI/configuration pages are visually dense.** | Long code/table pages become hard to orient, especially in a three-column desktop layout. | Split by lifecycle boundary; give reference pages a wider content mode and preserve the local ToC. |
| **Machine/network context is mostly prose.** | Running a connected-host command on an airlock is a high-cost mistake. | Add compact context chips above command blocks: `Connected host`, `Airlock`, `CI`, `Network required`, `Offline only`, `Read-only verification`. |
| **The product's security posture is not visible as a navigational category.** | Reviewers and operators may not recognize where to evaluate guarantees and limits. | Add a first-class "Security and operations" nav group and matching homepage path. |
| **Version/freshness is not visually obvious.** | Users cannot quickly connect a docs page to a release or compatibility boundary. | Add current version/release link and a short support badge in the header/footer. |

## Accessibility and responsive validation

The current UI has positive signals: search, theme control, skip link, ToC, semantic headings, and an edit-page link. However, this review is based on desktop captures and static markup, not a WCAG audit. Validate before refactoring:

- keyboard operation of navigation groups, tabs, search, theme control, copy buttons, and page anchors;
- visible focus indicators in both themes;
- contrast in yellow/green/cyan badges, diagram labels, muted code tokens, and callout surfaces;
- responsive behavior of the transport diagram, large TOML/code examples, and wide tables;
- semantic labels for machine/network state chips;
- print/no-script usability of the airlock checklist.

---

# Recommended information architecture

```text
Overview
  What is pixi-sandbox?
  How it works
  Glossary and security boundaries
  Compatibility and releases

Start here
  Install and verify the CLI
  First verified offline bundle
  Choose your role

For project owners
  Configure bundles
  Cargo vendoring
  Transport sizes, sharding, and budgets

For CI maintainers
  CI publishing
  Generated workflow policy
  Migration from composite Actions

For airlock operators
  Restore on the airlock
  Operator checklist
  Troubleshooting and incident response

Security and operations
  Verification guarantees
  Host and GUI requirements
  Diagnostics and evidence logs

Reference
  CLI overview and shared diagnostics
  Create and verify transport
  Publish transport branches
  Restore and inspect transport
  Tools and self-update
  Configuration overview
  Bundle, CI, and host policy

Project
  Architecture and repository layout
  Contributing / edit this documentation
```

## Existing-page migration map

| Current page | Proposed destination | Main change |
|---|---|---|
| Homepage | Overview | Retain hero/flow; remove public "Icons & Stack"; add role entry points and complete lifecycle CTA. |
| Installation | Install and verify the CLI | Preserve installation variants; add prerequisites/compatibility table and first command check. |
| Quickstart | First verified offline bundle | Make the end-to-end connected-to-airlock result the center. |
| Using in your project | Configure bundles | Keep detailed content; begin with owner role, artifacts, and outcome. |
| CI Publishing | For CI maintainers | Keep; add generated workflow lifecycle and upgrade/migration links. |
| Restore on the airlock | For airlock operators | Keep; add printable checklist and symptom links. |
| GUI and host requirements | Security and operations | Keep non-goals; link from configuration and restore. |
| Design / Repository layout | Project | Consolidate architecture/contributor material; remove from primary adoption flow. |
| Retired Action interfaces | Migration from composite Actions | Expand into migration matrix with before/after commands and workflow changes. |
| CLI Reference | Five CLI references | Split along lifecycle boundaries. |
| Configuration Reference | Configuration overview + policy | Split schema/basic config from bundles/CI/host policy. |

---

# Prioritized refactor plan

## P0 — adoption safety and role clarity

| Work | Effort | Acceptance criterion |
|---|---:|---|
| Create **First verified offline bundle**. | Medium | A reader can create, publish, transfer, restore, and prove one minimal bundle offline using exact commands and expected artifacts. |
| Add role-based entry points on the homepage. | Small | Every primary audience can select a role and reach a tailored first page in one click. |
| Add contextual machine/network chips to commands. | Small–Medium | Commands clearly state where they run and whether network/filesystem writes are expected. |
| Publish **Glossary and security boundaries**. | Medium | Connected host, airlock, transport, bundle, sandbox branch, doctor, and host requirements are defined once and linked from all relevant pages. |
| Publish troubleshooting/incident response. | Medium | Top restore/verification/branch/budget/vendoring failure modes have safe next actions and evidence collection steps. |

## P1 — reference and lifecycle maturity

| Work | Effort | Acceptance criterion |
|---|---:|---|
| Split CLI Reference by lifecycle concern. | Medium | Readers can reach create/verify, publish, restore, or tools/reference information without scanning an all-command page. |
| Split Configuration Reference. | Medium | Minimal setup, bundle policy, CI policy, budgets, and host requirements are independently deep-linkable. |
| Add releases, compatibility, and upgrading. | Small–Medium | Current release, Pixi/OS/Git requirements, generated-workflow policy, and retired interfaces are visible from every page. |
| Upgrade migration guide. | Small | Previous composite Action/reusable publisher users have a clear before/after mapping and migration checklist. |
| Add a printable operator checklist. | Small | An airlock operator can use a compact no-network checklist without reading the complete reference. |

## P2 — visual and documentation-system refinement

| Work | Effort | Acceptance criterion |
|---|---:|---|
| Replace homepage "Icons & Stack" section. | Small | The homepage ends with useful paths, comparison, support, or security information instead of implementation/design-system content. |
| Add wider reference layout. | Small–Medium | TOML examples, CLI flags, and tables are readable at 1280–1440px without excessive wrapping. |
| Standardize guide templates. | Small | Every guide starts with outcome, audience, prerequisites, machine context, and maturity; ends with verify/failures/next steps. |
| Add screenshot/diagram annotations. | Medium | Architecture visual labels remain readable at mobile and low-vision zoom sizes. |
| Add snippet/link/accessibility tests. | Medium | Command snippets are tested where practical; internal links, keyboard navigation, mobile layout, and contrast are checked in CI. |

---

# Patterns to adopt from each benchmark

## From Pixi

1. **A linear beginner path** — Installation → First workspace → Basic usage.
   **Adaptation:** Install → First verified bundle → Airlock restore.

2. **A broad but understandable top-level taxonomy** — getting started, concepts, deployment, integration, advanced, reference.
   **Adaptation:** use role/lifecycle groups rather than duplicate Pixi's product taxonomy.

3. **Deep-linkable command reference.**
   **Adaptation:** split pixi-sandbox reference into a small set of lifecycle pages.

4. **Version/changelog visibility.**
   **Adaptation:** current release/support status plus release notes before a full multi-version system.

5. **Contextual learning prompts and next steps.**
   **Adaptation:** link each stage of the transport lifecycle to the next safe action and prerequisite.

## From Playwright

1. **Concept page before advanced reference.**
   **Adaptation:** one canonical transport/lifecycle model before CLI/configuration details.

2. **Task-oriented operational documentation.**
   **Adaptation:** make publish, restore, offline verification, and incident response first-class workflows.

3. **Reference organization by user intent.**
   **Adaptation:** group commands by create/verify, publish, restore, and tools—not alphabetically or as one scroll.

4. **Explicit stability and change communication.**
   **Adaptation:** expose compatibility, deprecation/migration, and release behavior in UI and docs.

5. **Consistent page templates and local navigation.**
   **Adaptation:** use the current strong Starlight shell while standardizing audience, prerequisites, outcome, verification, failure modes, and next links.

## What not to copy

- Do not grow to Pixi's 210-page size or Playwright's 148-page size just to appear mature.
- Do not create one reference page per small option or flag unless independent discovery justifies it.
- Do not replace pixi-sandbox's branded transport diagram with a generic documentation landing page.
- Do not hide safety boundaries behind "advanced" pages: they are core to the product promise.
- Do not introduce a version switcher before several documentation versions are actively supported.

---

# Suggested first two-week documentation sprint

1. Ship the **First verified offline bundle** guide.
2. Put **Project owner / CI maintainer / Airlock operator / Security reviewer** entry cards on the homepage.
3. Add command context chips for machine, network, and write behavior.
4. Extract **How it works** and **Glossary/security boundaries** from repeated prose.
5. Add **Troubleshooting and incident response**.
6. Add a compact **Compatibility/release** footer block and link to edit/report feedback.

This gives pixi-sandbox the clearest adoption path, strengthens its safety promise, and uses its existing strong visual system rather than replacing it.

---

## Evidence pages reviewed

### pixi-sandbox

- `/` — product positioning, diagram, feature cards, Quick Start, navigation/visual system
- `/installation/` — Cargo/Pixi Global/release bootstrap and channel-upgrade guidance
- `/quickstart/`
- `/guides/using-in-your-project/` — connected host vs. airlock workflow
- `/guides/ci-publishing/`
- `/guides/host-requirements/`
- `/guides/actions/` — retired Actions migration
- `/restore/` — offline operator instructions and verification
- `/reference/cli/` — command/diagnostics density
- `/reference/configuration/` — schema/policy density
- `/repository/` and `/design/`

### Pixi

- `/latest/`
- `/latest/installation/`
- `/latest/first_workspace/`
- `/latest/getting_started/`
- `/latest/deployment/*`
- `/latest/integration/*`
- `/latest/reference/cli/*`
- `/latest/CHANGELOG/`

### Playwright

- `/docs/intro`
- `/docs/test-fixtures`
- `/docs/test-configuration`
- `/docs/running-tests`
- `/docs/test-cli`
- `/docs/test-parallel`
- `/docs/release-notes`
- `/docs/api/*` current stable pages
