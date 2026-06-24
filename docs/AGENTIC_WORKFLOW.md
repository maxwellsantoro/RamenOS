# Agentic Workflow & Guardrails

**Last Updated:** 2026-02-20
**Status:** Active

RamenOS is an AI-native OS built *by* AI agents. To maintain velocity (achieving in weeks what normally takes years), we do not rely on LLM context windows or "vibes." We use strict, hardware-level guardrails for our coding agents.

## 1. Physical Restraints (Hooks)
We utilize `PreToolUse` and `PostToolUse` hooks (via `.claude/settings.json`) to enforce physical limits on the AI:
- **Formatting & Linting:** Every file written by an agent is automatically run through `rustfmt` and `cargo clippy -D warnings`.
- **Immutable Files:** Agents are physically blocked from manually editing `Cargo.lock` or `*.generated.rs` files via shell hooks. They must use the correct toolchain commands (`cargo` or `just codegen`).

## 2. Specialized Personas (Separation of Concerns)
We do not use monolithic prompts. We use specialized reviewer agents:
- **Boundary Checker:** Enforces `kernel ≠ services ≠ store`.
- **Constitution Reviewer:** Scans for `ioctl` escapes, POSIX leakage, and kernel heap allocations.
- **Foundry Validator:** Maps file changes to the exact CI gate that must be run to prove correctness.

## 3. Procedural Skills (The "IDL" for AI)
Agents follow deterministic state machines for complex tasks (e.g., `new-slice`, `new-idl`). 
1. Define the scope.
2. Write the Foundry gate (Gate-First Testing).
3. Add to the `justfile`.
4. Generate the IDL.
5. Implement the stub.

By removing the "blank canvas," the AI never wanders. It simply executes the factory pipeline.
