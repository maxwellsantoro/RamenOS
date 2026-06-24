#!/usr/bin/env bash
# Foundry gate for S10.3 Projection Storage (schema + semantic store IDL contract).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

echo "=== S10.3 Projection Storage Foundry Gate ==="

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=idl_contract"
test -f idl/harness/semantic_store_v1.toml
test -f kernel_api/src/generated/semantic_store_v1.generated.rs

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=schema_tests"
cargo test -p artifact_store_schema projection_storage --quiet

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=kernel_api_wire"
cargo test -p kernel_api semantic_store_v1 --quiet

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
cargo test -p store_service projection_index --quiet

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=durable_index_roundtrip"
cargo test -p store_service durable_index_roundtrip --quiet
cargo test -p store_service corrupt_index_fail_closed --quiet
cargo test -p store_service read_only_override_rejects_mutation --quiet

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=ingest_updates_projection_index"
cargo test -p store_service ingest_artifact_updates_projection_index --quiet

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=read_only_vfs_projection"
cargo test -p store_service read_only_vfs_projection --quiet
cargo test -p store_service materialize_rejects_parent_dir_escape --quiet

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: INFO step=projection_cow_commit"
cargo test -p store_service projection_cow_commit_repoints_path_preserves_prior_blob --quiet

echo "FOUNDRY_PROJECTION_STORAGE_S10_3: PASS"
echo "FOUNDRY_PROJECTION_STORAGE_S10_3: ok"
