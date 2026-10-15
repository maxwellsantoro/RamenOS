# Ordinary host Store authority epochs

**Date:** 2026-10-14
**Status:** Offline revoke-all rollover; host scope only

The ordinary strict host Store retains at most 1024 distinct capability IDs per
epoch. Expiry and revocation do not reclaim an ID. Old verification keys remain
necessary for every retained v1 signed command, including revoked grants. Adding
keys to a legacy root supports overlap provisioning; removing historical verifiers
is not a supported retirement operation by itself.

The supported capacity/key-retirement operation is an explicit **offline,
revoke-all epoch transition**. It preserves artifact CAS, signed manifests,
ownership and projections, and discards the old epoch's authority records only
after retiring all of its verification keys. Every client needs a newly signed
token and externally authorized grant under a fresh exclusive key. Reusing a
numeric capability ID in the new epoch cannot make an old signed token valid.
There is no automatic compaction, online rollover or preservation of live grants.

## Trust and bounds

`AuthorityTransition` and `AuthorityEpochFloor` in
[`live_authority.rs`](../services/store_service/src/live_authority.rs) define the
public encoding. No signing secret is loaded by the Store or migration tool.
The external issuer supplies a signed transition; the operator supplies an exact
checkpoint floor outside the Store root. The root and every epoch must use
exclusive authority keys. Operators/issuers must ensure first-enrollment root-ID
uniqueness and must never use an epoch key for another root. One Store instance
cannot establish global key uniqueness across independently operated roots.

Legacy state is epoch 0 and has no root ID. First enrollment assigns a nonzero
32-byte root ID and binds the exact legacy database digest. Each later transition
keeps that ID and advances the epoch by exactly one without wrapping. The signed
history retains every prior configured authority key, sorted uniquely by raw
bytes. The new key must be nonweak and absent from current/retired keys. History
is bounded to 64 retired keys; exhaustion denies without deleting history.

This uses fresh key identity to separate epochs while preserving the existing v1
capability and management-command wire formats. Artifact publisher keys remain
separate. Target Domain Manager grants and native Store transport are unfinished.

The external floor format is:

```text
lowercase_root_hex:positive_decimal_epoch:lowercase_checkpoint_sha256_hex
```

Strict epoch-state startup requires exactly that root ID, epoch and checkpoint
digest through `RAMEN_STORE_AUTHORITY_EPOCH_FLOOR`, and exactly the checkpoint's
single active key through `RAMEN_STORE_CAP_TRUSTED_KEYS`. Missing/wrong floors,
legacy backups under a pinned epoch, wrong or retired/overlap keys, malformed
state and duplicate records deny. The operator-controlled configuration is the
rollback anchor. This is not hardware monotonic storage or protection from an
actor able to replace both Store files and trusted process configuration.

## Signed transition

A JSON proof carries these typed fields, with no unknown or duplicate fields:

| Field | Meaning |
|-------|---------|
| version | Transition format 1 |
| root_id | Nonzero 32-byte enrolled identity |
| from_epoch / to_epoch | Current epoch and exactly its successor |
| from_state_digest | SHA-256 of the exact current canonical authority database |
| new_public_key | Fresh 32-byte Ed25519 key, exclusive to this root/epoch |
| retired_public_keys | Exact sorted cumulative history including all current keys |
| old_signature / new_signature | 64-byte signatures by a current issuer and the new issuer |

Both issuers sign `AuthorityTransition::signing_bytes()`: the domain separator
`RamenOS.Store.AuthorityEpochTransition.v1` followed by NUL, then version(u16LE),
root ID, from-epoch(u64LE), source digest, to-epoch(u64LE), new key,
retired-key count(u64LE), and each sorted 32-byte key. Signatures are excluded from
those signing bytes. `checkpoint_digest()` hashes the canonical fixed-integer
bincode encoding of the complete proof, including both signatures. External
issuers should call these public helpers rather than reproduce another encoding.

Proof input is a bounded regular non-symlink file, at most 32768 bytes. The
reader rejects malformed shape, unsupported version, noncanonical key history,
invalid signature lengths and trailing JSON data. Migration verifies both signatures
against the current and new keys before persistence. The source state must already
exist with its initialization marker; migration cannot initialize an empty root.

## Operator procedure

1. Stop and join the Store owner. Its held root `flock` makes migration fail while
   any Store instance remains live; running readers/publications cannot cross an
   offline transition.
2. Preserve the exact source database and current trusted configuration for
   investigation. Obtain the external dual-signed proof binding that database's
   SHA-256, current epoch, enrolled ID and complete key history. No state may
   change between signing and migration.
3. Run the tool with the **source** authority-key configuration and source floor
   (no floor for legacy epoch 0):

   ```bash
   cargo run -p store_service --bin store_authority_rotate -- ROOT PROOF_JSON
   ```

   This is an explicit operator action on the named root. It reads only public
   proof material. It checks the source keys, signatures, digest, exact history
   and epoch, then writes the canonical v2 checkpoint plus an empty authority
   registry using file sync, atomic rename, directory sync and exact readback.
   Only after success does stdout contain the public next floor.
4. Verify that output against `proof.checkpoint_digest()`. Update trusted
   deployment configuration to the new single authority key and exact new floor;
   keep the pin outside the Store root. Restart strict Store and issue new signed
   grants/tokens. Existing signed artifacts remain readable through new grants.
5. Retain the proof, pin and source-bound migration result. Old tokens, grant
   commands and revoke commands must deny in the new epoch. Retain the pin when
   restoring backups; an older database cannot satisfy it.

## Interrupted transition and availability

An error after rename may leave the old or new database. It never prints a
success floor. Preserve files and proof, verify which complete checkpoint is
present, and recover using the matching externally authorized configuration.
Old/new floor mismatch denies startup. Never delete authority state or its
initialization marker, silently enable development mode, remove tombstones in
place, or restore an older floor to make validation pass. An offline command
retry against a changed source digest is a denial, not permission to repeat an
uncertain mutation. Ordinary authority persistence uncertainty remains fail
closed until a verified restart.

Runtime operation leases are per capability ID. Revoke closes admission, persists
a tombstone and drains all already-admitted operations for that ID before
acknowledging; every concurrent retry joins that drain. The state mutex is
released while waiting. Unrelated grant/revoke can complete while a GetBlob read
is held in filesystem IO. Reads recheck global health before success; publication
holds health exclusion through its actual durable commit. Panic retires lease
accounting and poisons authority. This does not promise bounded disk latency,
management progress during stuck authority fsync/publication, or unrelated data
progress through the separate domain registry lock.

The host still returns GetBlob paths. Revocation cannot retract already delivered
paths/bytes or prevent same-UID filesystem access. These bounds and the external
pin/key-exclusivity assumptions remain part of the supported prototype.
