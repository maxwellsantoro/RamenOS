set shell := ["bash", "-euxo", "pipefail", "-c"]

# --- Basics ---
fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets --exclude kernel_uefi --exclude kernel_aarch64 -- -D warnings

clippy-baseline:
	bash ./tools/ci/foundry_lint_baseline.sh

clippy-strict:
	bash ./tools/ci/foundry_lint_baseline.sh

clippy-baseline-soft:
	LINT_ALLOW_WARNINGS=1 bash ./tools/ci/foundry_lint_baseline.sh

clippy-strict-tranche1:
	bash ./tools/ci/foundry_lint_strict_tranche1.sh

clippy-strict-tranche2:
	bash ./tools/ci/foundry_lint_strict_tranche2.sh

clippy-strict-tranche3:
	bash ./tools/ci/foundry_lint_strict_tranche3.sh

clippy-strict-tranche4:
	bash ./tools/ci/foundry_lint_strict_tranche4.sh

clippy-strict-tranche5:
	bash ./tools/ci/foundry_lint_strict_tranche5.sh

preflight:
	bash ./tools/ci/foundry_preflight.sh

build-host: codegen
	cargo build --workspace --exclude kernel_uefi --exclude kernel_aarch64

# --- IDL / Codegen ---
codegen:
	bash ./tools/ci/run_codegen.sh

idl-lint:
	bash ./tools/ci/foundry_idl_lint.sh

# --- Store S0 ---
store-s0:
	cargo run -p store_cli -- emit-plan --catalog store/catalog.json --program-id ramen.demo.hello --out out/store/launch_plan.json

# --- Target builds (no bootable image yet; just checks we compile for targets) ---
build-targets:
	cargo build -p kernel_api --target x86_64-unknown-none
	cargo build -p kernel_api --target aarch64-unknown-none
	cargo build -p kernel --target x86_64-unknown-none
	cargo build -p kernel --target aarch64-unknown-none
	cargo build -p kernel_aarch64 --target aarch64-unknown-none

# --- UEFI kernel builds ---
build-uefi:
	cargo build -p kernel_uefi --target x86_64-unknown-uefi
	cargo build -p kernel_uefi --target aarch64-unknown-uefi

# --- Foundry gates (placeholder until QEMU boot exists) ---
foundry-s0:
	./tools/ci/foundry_s0.sh

foundry-store-s0:
	./tools/ci/foundry_store_s0.sh

foundry-all-s0:
	./tools/ci/foundry_all_s0.sh

# --- S1 gates ---
foundry-artifact-s1:
	./tools/ci/foundry_artifact_s1.sh

foundry-all-s0-s1:
	./tools/ci/foundry_all_s0_s1.sh

foundry-all-s0-s1-s2:
	./tools/ci/foundry_all_s0_s1_s2.sh

foundry-compat-s2:
	./tools/ci/foundry_compat_s2.sh

foundry-init-s2-2:
	./tools/ci/foundry_init_s2_2.sh

foundry-trace-s3:
	./tools/ci/foundry_trace_s3.sh

foundry-portal-file-ro-s3:
	./tools/ci/foundry_portal_file_ro_s3.sh

foundry-driver-capsule-s3x:
	./tools/ci/foundry_driver_capsule_s3x.sh

foundry-store-s4:
	./tools/ci/foundry_store_s4.sh

foundry-store-s5:
	./tools/ci/foundry_store_s5.sh

foundry-posix-s5:
	./tools/ci/foundry_posix_s5.sh

foundry-domain-manager-s6:
	./tools/ci/foundry_domain_manager_s6.sh

foundry-portal-suite-s6:
	./tools/ci/foundry_portal_suite_s6.sh

foundry-gpu-quarantine-s7:
	./tools/ci/foundry_gpu_quarantine_s7.sh

# --- S7 Security Hardening Phase 3: Security Gates ---
foundry-s7-posix-runner-security:
	./tools/ci/foundry_s7_posix_runner_security.sh

foundry-s7-store-signature-security:
	./tools/ci/foundry_s7_store_signature_security.sh

foundry-s7-access-control-security:
	./tools/ci/foundry_s7_access_control_security.sh

# Run all S7 security gates
foundry-s7-all-security:
	./tools/ci/foundry_s7_all_security.sh

# --- NEW-001: Refcount Overflow Security Regression Test ---
foundry-refcount-overflow-security:
	./tools/ci/foundry_refcount_overflow_security.sh

foundry-shmem-contract-s8-phase1:
	bash ./tools/ci/foundry_shmem_contract_s8_phase1.sh

foundry-shmem-dataplane-s8-phase4-integration:
	bash ./tools/ci/foundry_shmem_dataplane_s8_phase4_integration.sh

foundry-all-s0-s1-s2-s3-s4-s5:
	./tools/ci/foundry_all_s0_s1_s2_s3_s4_s5.sh

foundry-all-s0-s1-s2-s3-s4-s5-s6:
	./tools/ci/foundry_all_s0_s1_s2_s3_s4_s5_s6.sh

compat-s2-initrd:
	./tools/compat/build_compat_initrd.sh

compat-s2-artifact-img:
	./tools/compat/build_compat_artifact_img.sh

# --- Wave B Batch 1 hardening gate ---
foundry-hardening-wave-b-batch1:
	./tools/ci/foundry_hardening_wave_b_batch1.sh

# --- V-006 Phase 3: POSIX Runner Store Integration ---
foundry-posix-runner-s9-2-store-integration:
	./tools/ci/foundry_posix_runner_s9_2_store_integration.sh

# --- V-007 Phase 4: Cryptographic Signatures and SO_PEERCRED ---
foundry-v007-phase4-crypto-signatures:
	./tools/ci/foundry_v007_phase4_crypto_signatures.sh

# --- V-012 Phase 2: Domain-Scoped Trace Writers ---
foundry-v012-phase2-domain-scoped-writers:
	./tools/ci/foundry_v012_phase2_domain_scoped_writers.sh

# --- V-007 Phase 2: Store Service IPC ---
foundry-v007-phase2-store-service-ipc:
	./tools/ci/foundry_v007_phase2_store_service_ipc.sh

# --- V-007 Phase 3: Store Service Hardening ---
foundry-v007-phase3-store-hardening:
	./tools/ci/foundry_v007_phase3_store_hardening.sh

# --- V-012 Phase 3: Trace Capability-Based Access Control ---
foundry-v012-phase3-trace-capabilities:
	./tools/ci/foundry_v012_phase3_trace_capabilities.sh

# --- V-007 Phase 5: Enhanced Store Security ---
foundry-v007-phase5-enhanced-store-security:
	./tools/ci/foundry_v007_phase5_enhanced_store_security.sh

# --- V-012 Phase 5: User-space Trace Service Client ---
foundry-v012-phase5-trace-client:
	./tools/ci/foundry_v012_phase5_trace_client.sh

# --- V-006 Phase 4: Native WASM SDK ---
foundry-native-wasm-s9-3:
	./tools/ci/foundry_native_wasm_s9_3.sh

# --- S10.0 Native Runner ---
foundry-native-runner-s10-0:
	./tools/ci/foundry_native_runner_s10_0.sh

# --- S10.1 Native Runner Production Integration ---
foundry-native-runner-s10-1:
	./tools/ci/foundry_native_runner_s10_1.sh

# CI-safe subset (no services required)
foundry-native-runner-s10-1-ci:
	SKIP_E2E_ASSERTIONS=1 ./tools/ci/foundry_native_runner_s10_1.sh

# --- S10.2 Semantic State Substrate ---
foundry-semantic-state-s10-2:
	bash ./tools/ci/foundry_semantic_state_s10_2.sh

# --- S10.3 Projection Storage ---
foundry-projection-storage-s10-3:
	bash ./tools/ci/foundry_projection_storage_s10_3.sh

# --- S10.4 Execution Fabric ---
foundry-execution-fabric-s10-4:
	bash ./tools/ci/foundry_execution_fabric_s10_4.sh

# --- S10.5 Host-to-Target Integration ---
foundry-host-target-s10-5:
	bash ./tools/ci/foundry_host_target_s10_5.sh

# --- S10.5.1 Broker / Kernel Harness Bridge ---
foundry-broker-kernel-bridge-s10-5-1:
	bash ./tools/ci/foundry_broker_kernel_bridge_s10_5_1.sh

# --- S10.5.2 QEMU IPC Bridge ---
foundry-qemu-ipc-bridge-s10-5-2:
	bash ./tools/ci/foundry_qemu_ipc_bridge_s10_5_2.sh

foundry-s11-driver-factory-s11-0:
	bash ./tools/ci/foundry_s11_driver_factory_s11_0.sh

foundry-s11-replay:
	bash ./tools/ci/foundry_s11_replay.sh

foundry-s11-reference-vault-s11-3:
	bash ./tools/ci/foundry_s11_reference_vault_s11_3.sh

capture-virtio-net-oracle:
	bash ./tools/trace/capture_virtio_net_oracle.sh

capture-virtio-net-packet-oracle:
	bash ./tools/trace/capture_virtio_net_packet_oracle.sh

capture-virtio-blk-oracle:
	bash ./tools/trace/capture_virtio_blk_oracle.sh

capture-virtio-blk-sector-oracle:
	bash ./tools/trace/capture_virtio_blk_sector_oracle.sh

foundry-s11-reference-vault-s11-3-live:
	REQUIRE_LIVE_ORACLE_TRACE=1 bash ./tools/ci/foundry_s11_reference_vault_s11_3.sh

foundry-s11-runtime-net-s11-8:
	bash ./tools/ci/foundry_s11_runtime_net_s11_8.sh

# Fast-path S11 gates: host replay + live Oracle vault + runtime harness I/O
s11: foundry-s11-replay foundry-s11-reference-vault-s11-3-live foundry-s11-runtime-net-s11-8

foundry-s12-golden-machine-s12-0:
	bash ./tools/ci/foundry_s12_golden_machine_s12_0.sh

foundry-s12-gop-probe-s12-1:
	bash ./tools/ci/foundry_s12_gop_probe_s12_1.sh

foundry-s12-hil-boot-s12-2:
	bash ./tools/ci/foundry_s12_hil_boot_s12_2.sh

foundry-s12-iommu-inventory-s12-3:
	bash ./tools/ci/foundry_s12_iommu_inventory_s12_3.sh

foundry-hil-appliance-s12-4:
	bash ./tools/ci/foundry_hil_appliance_s12_4.sh

# Fast-path S12 gates: contract smoke + QEMU GOP probe + HIL appliance docs/manifest gate
s12: foundry-s12-golden-machine-s12-0 foundry-s12-gop-probe-s12-1 foundry-hil-appliance-s12-4

# Opt-in HIL legs (requires RAMEN_HIL_GOLDEN_MACHINE=1 + serial env)
s12-hil: foundry-s12-hil-boot-s12-2 foundry-s12-iommu-inventory-s12-3

# HIL appliance inventory: docs/manifest by default; serial/relay dry-run with RAMEN_HIL_APPLIANCE=1
hil-appliance: foundry-hil-appliance-s12-4

foundry-org-governance-g0:
	bash ./tools/ci/foundry_org_governance_g0.sh

org-g0: foundry-org-governance-g0

foundry-s13-persistent-storage-s13-0:
	bash ./tools/ci/foundry_s13_persistent_storage_s13_0.sh

foundry-s13-virtio-blk-oracle-s13-2:
	bash ./tools/ci/foundry_s13_virtio_blk_oracle_s13_2.sh

foundry-s13-replay:
	bash ./tools/ci/foundry_s13_replay.sh

foundry-s13-runtime-block-s13-6:
	bash ./tools/ci/foundry_s13_runtime_block_s13_6.sh

foundry-s13-block-sector-oracle-s13-4:
	bash ./tools/ci/foundry_s13_block_sector_oracle_s13_4.sh

foundry-s13-nvme-boot-s13-7:
	bash ./tools/ci/foundry_s13_nvme_boot_s13_7.sh

# Fast-path S13 gates: contract + Oracle capture + replay + runtime harness.block I/O
s13: foundry-s13-persistent-storage-s13-0 foundry-s13-virtio-blk-oracle-s13-2 foundry-s13-block-sector-oracle-s13-4 foundry-s13-replay foundry-s13-runtime-block-s13-6

foundry-s13-atomic-update-s13-8:
	bash ./tools/ci/foundry_s13_atomic_update_s13_8.sh

# Opt-in HIL legs: NVMe ESP boot + A/B atomic update (RAMEN_HIL_GOLDEN_MACHINE=1 + serial env)
s13-hil: foundry-s13-nvme-boot-s13-7 foundry-s13-atomic-update-s13-8

clippy-strict-tranche6:
	bash ./tools/ci/foundry_lint_strict_tranche6.sh

# --- CI extended gates (security + native runner + semantic state) ---
foundry-ci-extended:
	bash ./tools/ci/foundry_ci_extended.sh
