# Desktop editor wire v1: coordinator allocation

**Status:** Independently reviewed canonical IDLs and generated Rust bindings
registered. The original 43-message allocation passed cross-interface lint,
`kernel_api` and the existing desktop-launch consumer checks. The native Save
candidate adds Artifact messages 9/10, bringing the canonical inventory to 45;
generation, lint and both bare-metal API target checks pass. Existing Artifact
messages 1–8 retain their bytes. Integrated Save/task runtime acceptance remains
pending; these definitions alone do not implement the task.

**Base:** `c1e7f8d0d60062ea21872480fcc1aeacec45e8fc`.

The [reviewed editor proposal](plans/desktop-editor-v0.md) owns the human task,
grant profile, data-object layouts, resource bounds, save linearization and
separate a/b/c/d assertion inventories. This packet freezes the native control
allocation before dependent writers. It preserves protocol 336 unchanged.

## Ownership and compatibility

One delegated writer authored the five canonical IDL files below. The coordinator
registered and generated their outputs and owns gates, shared host APIs and
status/history. Independent review checked both definitions and generated
correspondence. Incompatible field/layout/meaning changes need a new protocol;
the envelope has no separate interface-version field. Future adapter code must
use explicit little-endian codecs, exact lengths and strict canonical validation.
Generated `repr(C)` types alone do not perform those checks or grant authority.

| Canonical file | Namespace / version | Protocol |
|---|---|---|
| `idl/harness/input_v1.toml` | `harness.input` / 1 | 802 (`0x322`) |
| `idl/services/desktop_focus_v1.toml` | `services.desktop_focus` / 1 | 832 (`0x340`) |
| `idl/services/desktop_surface_v1.toml` | `services.desktop_surface` / 1 | 833 (`0x341`) |
| `idl/portals/desktop_editor_session_v1.toml` | `portal.desktop_editor_session` / 1 | 352 (`0x160`) |
| `idl/portals/desktop_artifact_v1.toml` | `portal.desktop_artifact` / 1 | 368 (`0x170`) |

Before allocation, no other repository IDL owned these five protocol values.
Cross-interface lint and independent review verified uniqueness.

## Exact message allocation

Fields and ordered widths come from the proposal's message table, with the
expansions below. Request/reply pairs have their own message IDs even if they
share a layout. All messages are at most 64 bytes and group `u64` fields before
their final `u32` fields; no implicit padding is a wire format.

| Interface | Message ID / name |
|---|---|
| Input | 1 `attach`, 2 `attach_reply`, 3 `produce_key`, 4 `produce_key_reply` |
| Focus | 1 `assign`, 2 `assign_reply`, 3 `poll_keys`, 4 `poll_keys_reply` |
| Surface | 1 `create`, 2 `create_reply`, 3 `acquire`, 4 `acquire_reply`, 5 `present`, 6 `present_reply`, 7 `consume`, 8 `consume_reply`, 9 `destroy`, 10 `destroy_reply` |
| Editor | 1 `prepare_launch`, 2 `prepare_launch_reply`, 3 `cancel_preview`, 4 `cancel_preview_reply`, 5 `confirm_launch`, 6 `confirm_launch_reply`, 7 `get_status`, 8 `get_status_reply`, 9 `close_instance`, 10 `close_instance_reply`, 11 `revoke_instance`, 12 `revoke_instance_reply`, 13 `prepare_restart`, 14 `prepare_restart_reply`, 15 `observe_focus`, 16 `observe_focus_reply`, 17 `instance_bootstrap` |
| Artifact | 1 `read_selected`, 2 `read_selected_reply`, 3 `allocate_save_id`, 4 `allocate_save_id_reply`, 5 `commit`, 6 `commit_reply`, 7 `save_status`, 8 `save_status_reply`, 9 `observe_allocation`, 10 `observe_allocation_reply` |

Expand proposal abbreviations consistently:

- `req` → `request_id`, `session` → `session_id`, `sg` → `session_generation`.
- `instance` → `instance_id`, `ig` → `instance_generation`.
- `surface` → `surface_id`, `sf_g` → `surface_generation`, `mg` → `mapping_generation`.
- `plan` → `plan_id`, `operation` → `operation_id`.
- Text `len` → `byte_len`; `src_shm` → `source_shm`; `data_shm`, `receipt_shm`,
  `grants_shm`, `preview_shm` and their explicit length fields keep those names.
- `buffer0_shm`, `buffer1_shm`, `buffer_shm`, `object_generation`, `sequence`,
  `usage`, `phase`, `modifiers`, `format`, `stride` and other full names stay as
  specified in the proposal.

Editor `revoke_instance` has exactly the same 40-byte request shape as
`close_instance` (request/session/session-generation/instance/instance-generation)
and the 16-byte short reply (request/status/reserved). Its endpoint class is
trusted chrome; the editor's self endpoint cannot revoke or close. This makes
the reviewed revocation boundary an explicit typed operation. Restart reply
reuses the prepare reply's 48-byte shape. Bootstrap is service-to-client only
and has the proposal's exact 48-byte fields; it is not an accepted request.
`observe_focus` belongs to the Editor protocol's own observation endpoint, not
the Focus assignment endpoint. It grants no focus control.

Artifact `observe_allocation` is a 32-byte original-only lookup: request, session,
session generation and original Allocate request IDs are four `u64` fields.
Its 32-byte reply contains request, original Allocate request and operation IDs
followed by status and reserved `u32` fields. Status is at byte 24 and reserved
at byte 28. A lookup observes the authentic original; it grants no allocation,
source write, Commit replay or renewed deadline. The
[live Save/task contract](contracts/editor-native-save-task-v0.json) owns its
opaque-owner admission and definitive absence requirements.

## Status and authority boundary

Statuses are exactly UI1.0's values: Ok=0, Denied=1, Invalid=2, Unsupported=3,
Stale=4, Exhausted=5, NotReady=6, Conflict=7, Timeout=8, Disconnected=9,
Unknown=10, Internal=11. Known status values do not imply successful mutation;
receipt outcomes distinguish committed and definitive noncommit as specified
in the proposal. Every explicit `reserved` field and unused payload byte is zero.

Payload IDs are references, never owner, role or PID authentication. Endpoint
class/owner/session/instance authority comes from opaque trusted peer contexts.
Driver-class key production, chrome-class launch/focus/revoke, editor self
observation, focused-key polling, owned surface work, compositor consumption and
selected-artifact access remain separately enforced classes. Equal numeric rights
across classes cannot authorize each other's operations. Unauthorized replies
expose only request correlation and status, with all other fields zero.

The exact native host codecs, input phase/usage/modifier corpus, object-kind and
endpoint-class enums, peer/lease APIs, commit-permit API and locking remain
coordinator freeze prerequisites. Wire allocation does not authorize dependent
handler dispatch. In particular, no request may bypass the immutable save permit
or resolve Unknown by replay; no bootstrap, artifact identity or shared-object
bytes may be treated as caller-issued authority.

## Acceptance for this bounded packet

The contract writer returns only the five IDLs and their digests. Independent
review checks IDs, names, widths/order, status/reserved comments and ownership.
The coordinator then registers/generates outputs once, runs `just codegen`,
`just idl-lint`, `cargo build -p kernel_api` and affected consumers, and inspects
the generated diff. Output types are committed for reproducibility.

Executable UI1.1a acceptance remains its 13 prewritten behavior/denial/failure
cases, after the full shared API freeze and initial RED. This wire packet adds
no editor execution, Store IO, surface/input enforcement, process containment,
guest execution or physical-device evidence. No hardware or model run is needed
to prepare or verify these definitions.
