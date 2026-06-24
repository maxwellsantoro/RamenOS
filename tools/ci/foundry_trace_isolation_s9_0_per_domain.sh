#!/bin/bash
# V-012 Phase 1: Per-domain trace isolation gate
#
# This gate tests that per-domain trace buffers are properly isolated.
# Tests:
# 1. Domain registry initializes kernel domain
# 2. Multiple user domains can be registered
# 3. Events from domain 1 don't leak to domain 2
# 4. Each domain has its own writer claim
# 5. Legacy API still works for backward compatibility

set -euo pipefail

echo "=== V-012 Phase 1: Trace Isolation Gate ==="

# Test 1: Kernel domain is pre-registered
echo "Test 1: Kernel domain pre-registration"
cargo test --lib -p kernel kernel_domain_pre_registered --quiet
echo "✓ Kernel domain pre-registered"

# Test 2: Multiple user domains can be registered
echo "Test 2: Register multiple user domains"
cargo test --lib -p kernel register_multiple_domains --quiet
echo "✓ Multiple domains registered"

# Test 3: Per-domain buffers are isolated
echo "Test 3: Per-domain buffer isolation"
cargo test --lib -p kernel per_domain_buffers_are_isolated --quiet
echo "✓ Per-domain buffers are isolated"

# Test 4: Events don't leak between domains
echo "Test 4: Cross-domain event leakage check"
cargo test --lib -p kernel domain_events_dont_leak_to_other_domains --quiet
echo "✓ Events don't leak between domains"

# Test 5: Writer claims are per-domain
echo "Test 5: Per-domain writer claims"
cargo test --lib -p kernel claim_writer_is_per_domain --quiet
echo "✓ Writer claims are per-domain"

# Test 6: Invalid domain IDs are rejected
echo "Test 6: Invalid domain ID rejection"
cargo test --lib -p kernel emit_to_invalid_domain_panics --quiet
cargo test --lib -p kernel read_from_invalid_domain_panics --quiet
echo "✓ Invalid domain IDs are rejected"

# Test 7: Overflow handling works per-domain
echo "Test 7: Per-domain overflow handling"
cargo test --lib -p kernel per_domain_ring_overflow_handling --quiet
echo "✓ Overflow handling works per-domain"

# Test 8: Legacy API backward compatibility
echo "Test 8: Legacy API backward compatibility"
cargo test --lib -p kernel legacy_api_still_works --quiet
cargo test --lib -p kernel legacy_read_skips_overwritten_events --quiet
echo "✓ Legacy API works for backward compatibility"

# Test 9: Atomic ordering guarantees
echo "Test 9: Atomic ordering guarantees"
cargo test --lib -p kernel writer_release_semantics_visible_to_reader --quiet
echo "✓ Atomic ordering guarantees work"

echo ""
echo "=== All V-012 Phase 1 Tests Passed ==="
echo "FOUNDRY_TRACE_ISOLATION_S9_0_PER_DOMAIN: ok"
