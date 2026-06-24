# Contributing

## Toolchain and formatting
- Toolchain is pinned in `rust-toolchain.toml` (`nightly-2026-02-08`).
- Rust formatting style is pinned in `rustfmt.toml`:
  - `style_edition = "2024"`
- Run formatting check before pushing:
  - `cargo fmt --all --check`

## Lint policy
- Host-workspace clippy is fail-closed on warnings.
- Strict tranches are enforced in CI with `-D warnings`.
- Temporary warning-tolerant baseline runs are local-only:
  - `just clippy-baseline-soft`

## Required local preflight
- Run the same end-to-end preflight flow before pushing:
  - `just preflight`
- `just preflight` runs:
  1. format check
  2. IDL codegen
  3. strict lint baseline + strict tranches
  4. host workspace tests
  5. Foundry umbrella gate (`tools/ci/foundry_all_s0_s1_s2_s3_s4_s5_s6.sh`)

## Lint debt discipline
- If a new `allow(...)` is necessary, record it in `docs/LINT_DEBT.md` with:
  - reason,
  - owner,
  - explicit exit criteria.
