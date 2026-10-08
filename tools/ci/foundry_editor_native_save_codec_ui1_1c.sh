#!/usr/bin/env bash
# Pure Save data codec only; actual fresh compiler/test evidence, no live authority.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
exec python3 tools/ci/editor_native_save_codec_result.py
