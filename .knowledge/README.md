# `.knowledge/` — Open Knowledge Format

Everything needed to understand *why* this project is the way it is. Written for humans and
agents who were not in the room.

| file | read it when |
| --- | --- |
| [`decisions.md`](decisions.md) | you want the eight load-bearing decisions and the measurement behind each — **start here** |
| [`design.md`](design.md) | you are changing the transport, the CLI surface, the CI flow, or you disagree with a decision |
| [`rust-bootstrap.md`](rust-bootstrap.md) | you are replacing the embedded Python bootstrap with a standalone Rust release artifact |
| [`publish-automation.md`](publish-automation.md) | you are changing `.pixi-sandbox.toml`, the setup action, or native multi-platform branch publication |
| [`research-results.md`](research-results.md) | you want the study's conclusions without the full design doc |
| [`research/EVIDENCE.md`](research/EVIDENCE.md) | you need the raw lab output (sizes, timings, git economics, pixi internals) |
| [`research/REPRODUCE-TRANSCRIPT.md`](research/REPRODUCE-TRANSCRIPT.md) | you want to see the whole flow run cold, verbatim |
| [`research/pixi_sandbox.py`](research/pixi_sandbox.py) | you are checking Rust behaviour or the portable bootstrap — this is the reference implementation |
| [`research/reproduce.sh`](research/reproduce.sh) | you want to re-run the end-to-end proof (`pixi run -e dev sandbox-proof`) |

## Conventions

- Findings carry their provenance: `✅` means measured in the lab, `⚠️` means reasoned but not
  verified. Numbers live next to the claim, not in a separate appendix.
- Sections are referenced by number from code comments (`… §11.3`, `… D4`). Keep those anchors
  stable when editing.
- The lab was: Debian 13, 2 vCPU, pixi 0.81.0, pixi-pack/pixi-unpack 0.7.11, cargo 1.98.1,
  linux-64. Anything version-sensitive says so inline.
