# Store Service IPC Reference

**Last Updated:** 2026-10-03
**Status:** Implemented host service boundary; stable gate/reference path

The canonical control contract is
[store_service_v1.toml](../../idl/services/store_service_v1.toml).
[store_service](../../services/store_service/) owns host artifact I/O;
[artifact_store_schema](../../artifact_store_schema/) provides shared types.
Consumers use the typed [Store client](../../services/store_service/src/client.rs) rather
than importing Artifact Store I/O across the service boundary.

## Current contract

The service supports manifest/blob lookup, verification, projection queries,
and descriptor-scoped ingestion. Requests bind caller authority, artifact IDs,
and domain ownership where applicable. Ingestion consumes opened descriptors
and verifies the snapshot actually read; an arbitrary client path is not an
ambient grant to read the host filesystem.

Publication uses durable recovery records. A recovered host Store transaction
is distinct from target block durability and S13's two-boot metal evidence.
Native runner code validates the verified WASM snapshot it consumes; a valid
manifest alone cannot substitute for checking that content.

## Operation and evidence

The service's default socket is `out/store_service.sock`; several clients default
to `/tmp/store_service.sock`. Set the same explicit socket for both sides.
[Development Reference](../DEVELOPMENT_REFERENCE.md) supplies a working local
example and names the signature/access-policy settings. Default authorization
is fail closed; `AllowAll` and unsigned-artifact mode are explicit dev opt-ins.

Run `just foundry-store-s0` for a self-contained Store demo. Additional Store
access, signature, recovery, and runner gates are defined in the
[justfile](../../justfile); their evidence is bounded host behavior.

The [original IPC design and implementation sketches](../archive/plans/2026-02-09-store-service-ipc-design.md)
remain historical. [Security Status](../../SECURITY_STATUS.md) owns current
security claims; [Current Status](../../CURRENT_STATUS.md) owns landed state.
