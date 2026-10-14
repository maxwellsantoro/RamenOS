#!/usr/bin/env bash
# Foundry gate for S10.3 Projection Storage (schema + semantic store IDL contract).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S10.3 Projection Storage Foundry Gate ==="
mkdir -p "$ROOT_DIR/out/foundry/selected-tests"
SELECTED_RUN_ROOT="$(mktemp -d "$ROOT_DIR/out/foundry/selected-tests/projection-storage.XXXXXX")"
RUNNER="$ROOT_DIR/tools/ci/selected_test_runner.py"

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=idl_contract"
test -f idl/harness/semantic_store_v1.toml
test -f kernel_api/src/generated/semantic_store_v1.generated.rs

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=schema_tests"
python3 "$RUNNER" --package artifact_store_schema --filter projection_storage --include-doc-tests \
  --output "$SELECTED_RUN_ROOT/schema"

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=kernel_api_wire"
python3 "$RUNNER" --package kernel_api --filter semantic_store_v1 --include-doc-tests \
  --output "$SELECTED_RUN_ROOT/kernel-api"

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=store_cli_validation"
TMP_INDEX="$(mktemp)"
cat >"$TMP_INDEX" <<'EOF'
{
  "schema_version": 1,
  "entries": [
    {
      "schema_version": 1,
      "content_id": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      "tags": ["receipt"],
      "path_alias": "/home/user/docs/receipt.pdf",
      "domain_id": 0
    }
  ],
  "path_projections": [
    {
      "schema_version": 1,
      "virtual_path": "/home/user/docs/receipt.pdf",
      "content_id": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      "domain_id": 0
    }
  ]
}
EOF
cargo run -p store_cli -- validate-projection-index --src "$TMP_INDEX" >/dev/null
rm -f "$TMP_INDEX"

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=store_service_projection_query"
# One Cargo-selected lib/bin build and one union execution per matching harness.
python3 "$RUNNER" --package store_service --include-doc-tests \
  --filter projection_index --filter durable_index_roundtrip \
  --filter corrupt_index_fail_closed --filter read_only_override_rejects_mutation \
  --filter ingest_artifact_updates_projection_index --filter read_only_vfs_projection \
  --filter materialize_rejects_parent_dir_escape \
  --filter projection_cow_commit_repoints_path_preserves_prior_blob \
  --filter review_projection_cow --output "$SELECTED_RUN_ROOT/store-service"

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=durable_index_roundtrip"
# Covered by the nonempty intended union above.
# Covered by the nonempty intended union above.
# Covered by the nonempty intended union above.

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=ingest_updates_projection_index"
# Covered by the nonempty intended union above.

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=read_only_vfs_projection"
# Covered by the nonempty intended union above.
# Covered by the nonempty intended union above.

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=projection_cow_commit"
# Covered by the nonempty intended union above.
# Covered by the nonempty intended union above.

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: PASS"
echo "FOUNDRY_PROJECTION_STORAGE_S10_3: ok"
