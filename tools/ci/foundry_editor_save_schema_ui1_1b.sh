#!/usr/bin/env bash
# Pure payload/schema prerequisite only; no Store handler, IO or authority proof.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
EVIDENCE="$PWD/out/desktop/editor-save-schema-ui1-1b"
CONSUMER=tools/ci/editor_save_schema_result.py
mkdir -p "$EVIDENCE"
# This serialized root runner owns only these evidence files.
for file in result.json consumer-self-test.log no-std.log std-clippy.log build.jsonl build.log cases.log tests.log; do
  rm -f "$EVIDENCE/$file" "$EVIDENCE/$file.execution.json"
done
python3 "$CONSUMER" --check-sources
python3 "$CONSUMER" --run-log "$EVIDENCE/consumer-self-test.log" -- python3 "$CONSUMER" --self-test
python3 "$CONSUMER" --run-log "$EVIDENCE/no-std.log" -- \
  cargo check -p artifact_store_schema --no-default-features
python3 "$CONSUMER" --run-log "$EVIDENCE/std-clippy.log" -- \
  cargo clippy -p artifact_store_schema --no-default-features --features std --test editor_save -- -D warnings
python3 "$CONSUMER" --run-log "$EVIDENCE/build.jsonl" --stderr-log "$EVIDENCE/build.log" -- \
  cargo test -p artifact_store_schema --no-default-features --features std --test editor_save --no-run --message-format=json
python3 "$CONSUMER" --run-log "$EVIDENCE/cases.log" -- \
  cargo test -p artifact_store_schema --no-default-features --features std --test editor_save -- --list
python3 "$CONSUMER" --check-list "$EVIDENCE/cases.log"
python3 "$CONSUMER" --run-log "$EVIDENCE/tests.log" -- \
  cargo test -p artifact_store_schema --no-default-features --features std --test editor_save -- --test-threads=1
python3 "$CONSUMER" --evidence "$EVIDENCE"
echo 'FOUNDRY_EDITOR_SAVE_SCHEMA_UI1_1B: PASS scope=pure-editor-save-payload-schema store_io=false authority=false store_runtime=false durability=false target=false device=false containment=false'
