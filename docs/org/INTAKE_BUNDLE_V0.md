# IntakeBundleV0

**Last Updated:** 2026-06-23
**Status:** G0.8.1 bounded implementation scaffold

`IntakeBundleV0` is the portable, read-only agent intake surface produced from
a passing and freshness-verified RamenOrg packet set.

## Outputs

```text
out/org/current_board_brief.md
out/org/intake_manifest.json
```

The manifest records repository-relative paths and SHA-256 digests for:

- board brief
- board packet
- work order
- handoff
- vote
- current-task source
- packet-validation report
- context grant

The manifest also carries the exact `path`, `sha256`, and `access` entries for
every granted context file.
It carries `authorized_new_paths` separately for scoped outputs that do not yet
exist and therefore cannot be hash-bound as input context.

## Validation Rules

- The packet-validation report must be `pass` and contain a SHA-256 digest for
  every checked packet plus `docs/org/current_task.yaml`.
- Brief rendering must stop if a checked artifact is missing, changed, or not
  present in the validation report.
- The intake manifest must validate against
  `schemas/org/intake_manifest_v0.schema.json`.
- Manifest digests must match current file bytes.
- Packet/current-task manifest digests must match the validation report ledger.
- Board packet refs, packet repo SHAs, and brief intake citations must agree
  with the manifest.
- Context-grant entries must match current file bytes and the generated grant.
- G0.8.1 is A2-local only and grants no merge, release, self-approval, HIL
  actuation, or public support authority.

## G0.6 Usage Finding

The first intake-only fresh-agent trial passed for a bounded plan without hidden
chat context. It did not provide enough artifact content for a responsible
patch: context refs and scoped source files were named but absent. See
`docs/org/trials/2026-06-23-g0-6-intake-only-agent-trial.md`.
