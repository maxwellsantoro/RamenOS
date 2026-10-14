#!/usr/bin/env bash
# Additive default-off host regressions; original task/recording gates stay required.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
exec python3 tools/ci/editor_reliability_gate.py
