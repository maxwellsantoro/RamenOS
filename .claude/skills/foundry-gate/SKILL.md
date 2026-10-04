---
name: foundry-gate
description: Run Foundry gates for a specific slice and report results
disable-model-invocation: true
allowed-tools: Read, Bash, Grep, Glob
---

Run the Foundry gate requested in $ARGUMENTS. Resolve recipes from `just --list`
and `justfile`; use `AGENTS.md` for required integration checks.

| Scope | Command | Boundary |
|-------|---------|----------|
| Boot | `just foundry-s0` | Dual-architecture QEMU boot/IPC/trace |
| Store demo | `just foundry-store-s0` | Host service and launch plan |
| Artifact lifecycle | `just foundry-artifact-s1` | Host CAS/install/rollback |
| Compatibility | `just foundry-compat-s2` | Linux VM; requires S2 inputs |
| Portal | `just foundry-portal-file-ro-s3` | Read-only picker and observed caps |
| Driver / hardware / storage | `just s11`, `just s12`, `just s13` | Default replay, inventory and QEMU |
| Appliance | `just hil-appliance` | Inventory by default; physical mode is opt-in |
| Governance | `just foundry-org-governance-g0` | Packets, negative cases and drift |
| Umbrella | `just foundry-all-s0-s1-s2-s3-s4-s5-s6` | Historical recipe name; broader S0–S8 coverage |

1. Read the gate before running it when inputs, privileges, or physical effects
   are unclear. A requested default gate does not authorize physical actuation.
2. Run the requested recipe; inspect its logs under `out/` and failing assertion.
3. Report status, exit code, and evidence scope. Missing inputs or prerequisites
   are not PASS. Never infer live HIL or metal success from replay/inventory.
4. Cite relevant sources for failures. Use `EVIDENCE_LEVELS.md` and
   `CURRENT_STATUS.md` for claim boundaries; do not launch unrelated suites.
