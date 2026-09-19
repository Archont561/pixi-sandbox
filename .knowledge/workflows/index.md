# Operational playbooks

* [One Lockfile, One Digest](/workflows/lockfile-digest-map.md) - The table every other workflow hangs off: which lockfile keys which component and which artifact carries its digest. _(⚠️ reasoned)_
* [Creating and Packing an Environment](/workflows/create-environment.md) - Step-by-step: pixi init, add packages with platforms, tasks, cargo init, sandbox init, kit build - on a networked box. _(✅ ⚠️)_
* [Reconstructing on an Airlocked Machine](/workflows/airlock-clone.md) - What a human types on the sealed box: fetch the dist branch, verify digests, install the drivers, reconstruct, run. _(✅ ⚠️)_
* [Running the Action](/workflows/action-run.md) - How to call the action, what the target types, the nine outcomes measured in this sandbox, and how to publish the action itself. _(✅ ⚠️)_
* [Republishing When a Lockfile Changes](/workflows/ci-republish.md) - The CI loop that keeps the branch honest: lock-guard, digest-keyed matrix, --only-changed, push and tag. _(⚠️ reasoned)_
* [pixi and cargo: What Travels Where](/workflows/pixi-cargo-interop.md) - The interoperability verdict - pixi-pack packs environments, not crate graphs - plus source replacement and the build.rs hole. _(✅)_
* [Copy-paste Scripts](/workflows/scripts.md) - Runnable shell for both sides of the airlock, kept verbatim so a reviewer can diff intent against implementation. _(✅ ⚠️)_
* [What Is Still Unproven](/workflows/unproven.md) - The honest gap register: every claim in this design that is inferred, with the experiment that would settle it. _(🚧 unproven)_
* [Dogfooding on a Box Like This One](/workflows/dogfooding.md) - The fidelity ladder D0-D4 for developing pixi-sandbox inside an airlock, with the measurements that justify each rung. _(✅ ⚠️)_
* [Publishing the Docs](/workflows/publishing-docs.md) - Astro Starlight + GitHub Pages + the in-repo mirror: the config that works, the traps that fail silently, and CI YAML - all measured in this sandbox. _(✅)_

## Group notes

This directory is one area of the `pixi-sandbox` knowledge bundle; the bundle root index is
[here](/index.md) and the conventions (labels, extensions, maintenance rules) are in
[Bundle Conventions](/conventions.md).
