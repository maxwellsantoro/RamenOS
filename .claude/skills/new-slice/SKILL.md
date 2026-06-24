---
name: new-slice
description: Scaffold a new vertical slice with Foundry gate, IDL spec, and implementation stub
disable-model-invocation: true
allowed-tools: Read, Write, Edit, Bash, Glob, Grep
---

Scaffold a new vertical slice for: $ARGUMENTS

## Pre-flight

Read these files for context:
- `SLICES.md` — existing slice definitions
- `CURRENT_STATUS.md` — what exists now
- `ROADMAP.md` — where this fits

## Steps

### 1. Define the slice scope

From $ARGUMENTS, determine:
- **Slice ID** (e.g., S3.1, S4)
- **OS capability** being added
- **Store feature** that consumes it
- **Foundry gate** that validates it

If any of these are unclear, ask the user. Every slice MUST have all three components (OS + Store + Foundry). This is non-negotiable per the Constitution.

### 2. Create the Foundry gate script (gate-first)

Create `tools/ci/foundry_<slice_name>.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

PASS=0
FAIL=0

assert_eq() {
  local label="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    echo "PASS: $label"
    ((PASS++))
  else
    echo "FAIL: $label (expected '$expected', got '$actual')"
    ((FAIL++))
  fi
}

# --- Gate assertions go here ---
# TODO: Add assertions for the slice

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
```

Make it executable: `chmod +x tools/ci/foundry_<slice_name>.sh`

### 3. Add justfile recipe

Add a new recipe to `justfile`:
```
foundry-<slice-name>:
	./tools/ci/foundry_<slice_name>.sh
```

### 4. Create IDL spec (if the slice introduces a new interface)

If the slice needs a new harness or portal, use the `/new-idl` skill or create the spec manually following existing patterns in `idl/`.

### 5. Create implementation stubs

Based on the slice scope, create minimal stubs in the appropriate crates. Do NOT write full implementations — stubs should compile and be wired into the gate.

### 6. Verify the gate runs

```sh
just foundry-<slice-name>
```

The gate should run (it may fail assertions — that's expected at this stage).

### 7. Update tracking documents

Add the slice to `SLICES.md` with its OS/Store/Foundry components listed.

### 8. Report

Print a summary:
- Slice ID and scope
- Gate script location
- IDL spec (if created)
- Implementation stubs created
- Next steps for filling in the implementation
