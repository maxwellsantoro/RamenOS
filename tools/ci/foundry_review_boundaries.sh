#!/usr/bin/env bash
# Regression gate for the September project-review boundary fixes.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 tools/ci/test_review_boundaries.py
python3 tools/hil/test_provenance.py
cargo test -p kernel create_region_rejects_unsupported_power_of_two_page_sizes --quiet
cargo test -p store_service --quiet
echo "FOUNDRY_REVIEW_BOUNDARIES: PASS"
