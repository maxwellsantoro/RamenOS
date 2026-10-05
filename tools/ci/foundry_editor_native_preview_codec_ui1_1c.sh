#!/usr/bin/env bash
# Pure NativePreview shared data only; no Store IO or authority proof.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
EVIDENCE="$PWD/out/desktop/editor-native-preview-codec-ui1-1c"
CONSUMER=tools/ci/editor_native_preview_codec_result.py
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
  cargo clippy -p artifact_store_schema --no-default-features --features std --test editor_preview_codec -- -D warnings
python3 "$CONSUMER" --run-log "$EVIDENCE/build.jsonl" --stderr-log "$EVIDENCE/build.log" -- \
  cargo test -p artifact_store_schema --no-default-features --features std --test editor_preview_codec --no-run --message-format=json
python3 "$CONSUMER" --run-log "$EVIDENCE/cases.log" -- \
  cargo test -p artifact_store_schema --no-default-features --features std --test editor_preview_codec -- --list
python3 "$CONSUMER" --check-list "$EVIDENCE/cases.log"
python3 "$CONSUMER" --run-log "$EVIDENCE/tests.log" -- \
  cargo test -p artifact_store_schema --no-default-features --features std --test editor_preview_codec -- --test-threads=1
python3 "$CONSUMER" --evidence "$EVIDENCE"
echo 'FOUNDRY_EDITOR_NATIVE_PREVIEW_CODEC_UI1_1C: PASS scope=pure-editor-native-preview-data-codec store_io=false authority=false store_runtime=false durability=false target=false device=false containment=false'
