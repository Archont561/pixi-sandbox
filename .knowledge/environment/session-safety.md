---
type: Policy
title: "Session Safety Rules"
description: What this session must not do: no commits or pushes without being asked, no credentials handling, snapshot exclusions.
resource: https://github.com/Archont561/pixi-sandbox
tags: [environment, policy, safety]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`SANDBOX_CONSTRAINTS.md`], sections: ["11"] }
---

# Session Safety Rules

## 11. Safety constraints of this session

* Work only on `arena/01a0b663-pixi-sandbox`; never switch/create/push other branches.
* Never delete/move the repo root or `.git`.
* Never request or store GitHub credentials/tokens/2FA; `gh` is already authenticated — if auth fails, report it rather than working around it.
* Do not commit large datasets/artifacts unless required by repo convention.
* Files outside `/home/user` are not persisted — do not write "important" things there.

---
