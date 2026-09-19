# The reference sandbox, measured

* [Host Inventory and Verdict](/environment/inventory.md) - CPU, memory, disk, OS, and the exact toolchain present or absent on the reference sandbox - with the headline verdict. _(✅)_
* [Network Model and Egress Matrix](/environment/network-model.md) - Reachable hosts, blocked hosts with the failure mode each shows, port rules, and the one-line reproduction script. _(✅)_
* [What the Constraints Imply](/environment/consequences.md) - Consequences for the project: what can be simulated locally, what must move to CI, and which tools are simply unavailable. _(✅ ⚠️)_
* [Execution and Git Semantics](/environment/execution-model.md) - How commands actually run here (no tty, stdin closed, no persistent background state) and the git/GitHub specifics that follow. _(✅)_
* [Disk and Performance Budget](/environment/budgets.md) - Observed throughput, clone costs and space limits that any artifact-packing design must budget against. _(✅)_
* [Session Safety Rules](/environment/session-safety.md) - What this session must not do: no commits or pushes without being asked, no credentials handling, snapshot exclusions. _(✅)_
* [Reproducing These Measurements](/environment/reproducing.md) - The exact probe commands, so every number in this bundle can be re-derived by a reader with access to the same box. _(✅)_

## Group notes

This directory is one area of the `pixi-sandbox` knowledge bundle; the bundle root index is
[here](/index.md) and the conventions (labels, extensions, maintenance rules) are in
[Bundle Conventions](/conventions.md).
