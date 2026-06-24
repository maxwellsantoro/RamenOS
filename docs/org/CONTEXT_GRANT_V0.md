# ContextGrantV0

**Last Updated:** 2026-06-23
**Status:** G0.8.1 scaffold

`ContextGrantV0` is a generated, read-only list of the exact repository files an
agent may rely on for a bounded patch plan or bounded implementation patch. It
is not ambient repository access and it does not grant merge, release,
self-approval, HIL actuation, or public support authority.

## Output

```text
out/org/context_grant.json
```

## Rules

- `docs/org/current_task.yaml.context_grant_refs` is the source file list.
- Every granted file carries `path`, `sha256`, and `access`.
- `access: patch` is legal only inside the work-order scope.
- `access: read` is legal only for a declared task context ref outside patch scope.
- Every `required_for_patch_plan` path must be present and hash-bound.
- `authorized_new_paths` may name scoped output files that do not exist yet;
  these are output permissions, not input context.
- Once a previously authorized output exists, it moves into hash-bound
  `granted_context` for the next work order.
- Missing or changed files invalidate the grant.
- Any existing file not listed is unavailable as input context. New output paths
  remain governed by the work-order scope.
- If more context is needed, the agent emits a `context_expansion_request` with
  the requested path and reason; it does not produce a patch.
- G0.8.1 is A2-local only.
