//! Gate-first actual-owner fixtures. No issuer, permit, publisher or draft setter.
#[allow(clippy::duplicate_mod)]
#[allow(dead_code)]
#[path = "../../../../services/desktop/tests/support/editor_host_assertions.rs"]
pub mod volatile;

use artifact_editor::{NativeArtifactEditor, NativeEditorError};
#[allow(dead_code)]
#[path = "../../../../services/store_service/tests/editor_native_read_support/mod.rs"]
pub mod read_control;
pub mod recording;

use artifact_store_schema::editor_save::*;
use desktop_service::dev::Status;
use desktop_service::editor_dev::{self as desktop, EditorMessage};
use kernel_api::generated::{
    desktop_artifact_v1 as artifact, desktop_editor_session_v1 as session,
    desktop_focus_v1 as focus,
};
use kernel_api::{cap::Handle, ipc::Envelope};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use store_service::editor_store::{self as store, StoreStatus};

pub const OLD: &[u8] = b"note=old\n";
pub const NEW: &[u8] = b"note=new\n";
pub const B: &[u8] = b"other=live\n";
pub const C: &[u8] = b"third=usable\n";
pub fn scenario(
    case: &'static str,
    stem: &'static str,
    leg: u32,
    scope: recording::ScenarioScope,
) -> recording::ScenarioSpec {
    recording::ScenarioSpec {
        case,
        scenario: stem,
        leg,
        scope,
        checkpoints: match case {
            "ui1_1_keyboard_edits_ascii_and_renders_owned_frame" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
            ],
            "ui1_1_ascii_bounds_preserve_prior_draft" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
            ],
            "ui1_1_focus_routes_reserved_keys_to_trusted_chrome" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_000",
                "control_main_001",
                "control_main_002",
                "control_main_003",
                "control_main_004",
                "control_main_005",
                "control_main_006",
                "inactive_desktop",
                "inactive_store",
            ],
            "ui1_1_focus_epoch_reset_overflow_and_detach" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_007",
                "control_main_008",
            ],
            "ui1_1_surface_freeze_alias_quota_and_chrome_clip" => &[
                "surface_writer_retired",
                "surface_alias_retired",
                "surface_copy_consumed",
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "surface_frozen_copy",
            ],
            "ui1_1_wire_schema_descriptor_and_identity_fail_closed" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_009",
                "control_main_010",
                "foreign_registration",
            ],
            "ui1_1_editor_grants_are_exact_and_preview_single_use" => &[
                "approval_invalidated",
                "selection_invalidated",
                "read_control_allocate_unsupported",
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_011",
                "control_main_012",
            ],
            "ui1_1_volatile_save_and_reopen_are_explicitly_labeled" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
            ],
            "ui1_1_revoke_paused_key_frame_and_save" => &[
                "source_write_retired",
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
            ],
            "ui1_1_fault_restart_discards_unsaved_draft_and_old_grants" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_013",
            ],
            "ui1_1_focus_compositor_restart_retires_old_epochs" => &[
                "service_restart_read_retired",
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_014",
            ],
            "ui1_1_stalled_session_leaves_other_session_usable" => &[
                "paused_join_not_ready",
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
            ],
            "ui1_1_store_commit_reopen_preserves_prior_blob" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
            ],
            "ui1_1_store_same_base_conflict_and_operation_reuse" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_015",
                "control_main_016",
                "control_main_017",
                "control_main_018",
            ],
            "ui1_1_store_lost_reply_reconciles_without_mutation_replay" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_019",
                "control_main_020",
                "control_main_021",
                "permitted_pre_selection",
            ],
            "ui1_1_store_recovery_validates_atomic_selection_and_receipt" => &[
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "permitted_pre_selection",
                "pause_pair_denial",
            ],
            "ui1_1_unknown_save_survives_editor_recovery" => &[
                "source_write_retired",
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_022",
                "control_main_023",
                "control_main_024",
                "control_main_025",
            ],
            "ui1_1_clock_capacity_and_counter_limits_fail_closed" => &[
                "session_capacity",
                "entry_join_pending",
                "entry_wait_timeout",
                "context_confirm_exhausted",
                "initial",
                "before_transfer",
                "after_transfer",
                "before_storage_fault",
                "after_storage_fault",
                "final",
                "producer_checkpoint",
                "helper_selector_replay",
                "helper_prepare_replay",
                "helper_confirm_replay",
                "helper_editor_take_replay",
                "helper_intent_take_replay",
                "helper_execution_poll_replay",
                "native_counter_boundary",
                "native_counter_secondary",
                "store_counter_boundary",
                "store_counter_closed_intent",
                "initial_frame",
                "rendered_frame",
                "source_reserved",
                "source_digest",
                "source_ascii",
                "source_snapshot",
                "source_schema",
                "wire_header_reserved",
                "wire_payload_reserved",
                "wire_tail_reserved",
                "control_main_026",
                "control_main_027",
                "control_main_028",
                "control_main_029",
                "control_main_030",
                "original_record_registration",
                "store_identity_expose",
                "capture_full_copy_exhausted",
                "capture_capacity_before_commit",
                "capture_capacity_after_commit",
                "io_two_writers_held",
                "io_third_allocation_denied",
            ],
            _ => panic!("exact eighteen source cases"),
        },
        requirements: match case {
            "ui1_1_keyboard_edits_ascii_and_renders_owned_frame" => {
                &["ui1_1_keyboard_edits_ascii_and_renders_owned_frame"]
            }
            "ui1_1_ascii_bounds_preserve_prior_draft" => {
                &["ui1_1_ascii_bounds_preserve_prior_draft"]
            }
            "ui1_1_focus_routes_reserved_keys_to_trusted_chrome" => {
                &["ui1_1_focus_routes_reserved_keys_to_trusted_chrome"]
            }
            "ui1_1_focus_epoch_reset_overflow_and_detach" => {
                &["ui1_1_focus_epoch_reset_overflow_and_detach"]
            }
            "ui1_1_surface_freeze_alias_quota_and_chrome_clip" => {
                &["ui1_1_surface_freeze_alias_quota_and_chrome_clip"]
            }
            "ui1_1_wire_schema_descriptor_and_identity_fail_closed" => {
                &["ui1_1_wire_schema_descriptor_and_identity_fail_closed"]
            }
            "ui1_1_editor_grants_are_exact_and_preview_single_use" => {
                &["ui1_1_editor_grants_are_exact_and_preview_single_use"]
            }
            "ui1_1_volatile_save_and_reopen_are_explicitly_labeled" => {
                &["ui1_1_volatile_save_and_reopen_are_explicitly_labeled"]
            }
            "ui1_1_revoke_paused_key_frame_and_save" => &["ui1_1_revoke_paused_key_frame_and_save"],
            "ui1_1_fault_restart_discards_unsaved_draft_and_old_grants" => {
                &["ui1_1_fault_restart_discards_unsaved_draft_and_old_grants"]
            }
            "ui1_1_focus_compositor_restart_retires_old_epochs" => {
                &["ui1_1_focus_compositor_restart_retires_old_epochs"]
            }
            "ui1_1_stalled_session_leaves_other_session_usable" => {
                &["ui1_1_stalled_session_leaves_other_session_usable"]
            }
            "ui1_1_store_commit_reopen_preserves_prior_blob" => {
                &["ui1_1_store_commit_reopen_preserves_prior_blob"]
            }
            "ui1_1_store_same_base_conflict_and_operation_reuse" => {
                &["ui1_1_store_same_base_conflict_and_operation_reuse"]
            }
            "ui1_1_store_lost_reply_reconciles_without_mutation_replay" => {
                &["ui1_1_store_lost_reply_reconciles_without_mutation_replay"]
            }
            "ui1_1_store_recovery_validates_atomic_selection_and_receipt" => {
                &["ui1_1_store_recovery_validates_atomic_selection_and_receipt"]
            }
            "ui1_1_unknown_save_survives_editor_recovery" => {
                &["ui1_1_unknown_save_survives_editor_recovery"]
            }
            "ui1_1_clock_capacity_and_counter_limits_fail_closed" => {
                &["ui1_1_clock_capacity_and_counter_limits_fail_closed"]
            }
            _ => panic!("exact eighteen requirement identities"),
        },
    }
}
pub fn desktop_pause_leg(p: desktop::NativeSavePausePoint) -> u32 {
    match p {
        desktop::NativeSavePausePoint::BeforeKeyDelivery => 0,
        desktop::NativeSavePausePoint::BeforePresentFreeze => 1,
        desktop::NativeSavePausePoint::BeforeFramePublish => 2,
        _ => panic!("source-fixed retirement points"),
    }
}
pub fn service_leg(p: desktop::ServiceKind) -> u32 {
    match p {
        desktop::ServiceKind::Focus => 0,
        desktop::ServiceKind::Compositor => 1,
        _ => panic!("source-fixed service restart"),
    }
}
pub fn commit_pause_leg(p: store::PausePoint) -> u32 {
    match p {
        store::PausePoint::BeforeCommitPermit => 0,
        store::PausePoint::AfterCommitPermit => 1,
        _ => panic!("source-fixed permit points"),
    }
}
pub fn storage_fault_leg(p: store::StorageFault) -> u32 {
    match p {
        store::StorageFault::MissingJournal => 0,
        store::StorageFault::TruncatedJournal => 1,
        store::StorageFault::UnknownJournalVersion => 2,
        store::StorageFault::ExtraJournalField => 3,
        store::StorageFault::DuplicateJournalField => 4,
        store::StorageFault::WrongJournalBinding => 5,
        store::StorageFault::BrokenJournalChain => 6,
        store::StorageFault::EarlierAllocatedOnlyJournal => 7,
        store::StorageFault::CorruptBlob => 8,
        store::StorageFault::UnsignedManifest => 9,
        store::StorageFault::WrongSigningKey => 10,
        store::StorageFault::UnboundPublicationIntent => 11,
        store::StorageFault::UnsignedPublicationIntent => 12,
        store::StorageFault::SymlinkEntry => 13,
    }
}
pub fn ok<T, E>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(_) => panic!("expected actual successful owner call"),
    }
}
pub fn err<T, E: std::fmt::Debug + PartialEq>(r: Result<T, E>, e: E) {
    match r {
        Err(v) => assert_eq!(v, e),
        Ok(_) => panic!("denial exposed value"),
    }
}
pub trait RecordedStatus: std::fmt::Debug + PartialEq + Copy {
    fn recording_space(self) -> &'static str;
    fn recording_code(self) -> u32;
}
impl RecordedStatus for Status {
    fn recording_space(self) -> &'static str {
        "DesktopStatus"
    }
    fn recording_code(self) -> u32 {
        self as u32
    }
}
impl RecordedStatus for StoreStatus {
    fn recording_space(self) -> &'static str {
        "StoreStatus"
    }
    fn recording_code(self) -> u32 {
        self as u32
    }
}
pub fn canonical(q: Envelope) -> Envelope {
    let b = ok(desktop::encode_envelope_wire(&q));
    let r = ok(desktop::decode_envelope_wire(&b));
    assert_eq!(ok(desktop::encode_envelope_wire(&r)), b);
    r
}
pub fn temporary() -> tempfile::TempDir {
    #[cfg(target_os = "linux")]
    {
        ok(tempfile::Builder::new()
            .prefix("ramenos-native-task-")
            .tempdir())
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        let p = Path::new("/System/Volumes/Data/private/tmp");
        let m = ok(fs::symlink_metadata(p));
        assert!(m.is_dir() && !m.file_type().is_symlink());
        assert_eq!((m.uid(), m.gid(), m.mode() & 0o7777), (0, 0, 0o1777));
        ok(tempfile::Builder::new()
            .prefix("ramenos-native-task-")
            .tempdir_in(p))
    }
}
pub fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(
        root: &Path,
        p: &Path,
        rows: &mut BTreeMap<PathBuf, Vec<u8>>,
        total: &mut usize,
        depth: u32,
    ) {
        assert!(depth <= 8);
        for e in ok(fs::read_dir(p)) {
            assert!(rows.len() < 2048);
            let p = ok(e).path();
            let m = ok(fs::symlink_metadata(&p));
            assert!(!m.file_type().is_symlink());
            if m.is_dir() {
                walk(root, &p, rows, total, depth + 1)
            } else {
                assert!(m.is_file() && m.len() <= 131072);
                *total = total.checked_add(m.len() as usize).unwrap();
                assert!(*total <= 32 * 1024 * 1024);
                let b = ok(fs::read(&p));
                assert_eq!(b.len() as u64, m.len());
                assert!(
                    rows.insert(ok(p.strip_prefix(root)).to_path_buf(), b)
                        .is_none()
                );
            }
        }
    }
    let (mut rows, mut total) = (BTreeMap::new(), 0);
    walk(root, root, &mut rows, &mut total, 0);
    rows
}
pub fn profile(n: usize) -> store::NativeSaveFixtureProfile {
    assert!((1..=3).contains(&n));
    store::NativeSaveFixtureProfile {
        origin_ms: 1,
        initial_objects: (0..n)
            .map(|i| store::InitialObject {
                owner_id: if i == 1 { 2 } else { 1 },
                object_id: 101 + i as u64,
                generation: 1,
                revision: 1,
                bytes: match i {
                    0 => OLD,
                    1 => B,
                    _ => C,
                }
                .to_vec(),
            })
            .collect(),
        fixture_signing_seed: [0x53; 32],
        fixture_key_id: "native-task-v0-fixture".into(),
    }
}
// Only the app_unknown incomplete-quiesce wrapper creates this scope. These
// are real opaque aliases; diagnostic IDs only deduplicate already owned tokens.
struct RecordedQuiesceJoins {
    owner: Arc<store::PreparedNativeSaveStore>,
    core_ordinal: u32,
    closed_after_incomplete: bool,
    pending: Vec<desktop::ProducerId>,
}
pub struct Fixture {
    pub recording: std::cell::RefCell<Option<recording::ScenarioRecorder>>,
    pub core_ordinal: u32,
    pub snapshot_ordinal: std::cell::Cell<u32>,
    pub recorded_fences: Vec<store::NativeSaveFenceSnapshot>,
    pub archived_cores: BTreeSet<u32>,
    pub h: desktop::HostDesktop,
    pub dc: Arc<desktop::FixtureController>,
    pub drivers: Vec<volatile::Driver>,
    pub prepared: Option<Arc<store::PreparedNativeSaveStore>>,
    pub root: Option<tempfile::TempDir>,
    pub path: PathBuf,
    pub initial: BTreeMap<PathBuf, Vec<u8>>,
    pub joined: BTreeSet<u64>,
    recorded_quiesce_joins: Option<RecordedQuiesceJoins>,
    pub desktop_pauses: Vec<Arc<desktop::NativeSavePause>>,
    pub store_pauses: Vec<(Arc<store::PreparedNativeSaveStore>, Arc<store::PauseToken>)>,
}
fn latest_request<T: EditorMessage>(e: &desktop::Evidence) -> T {
    e.exchanges
        .iter()
        .rev()
        .find_map(|(q, _)| T::decode(q).ok())
        .expect("actual routed typed request")
}
impl Fixture {
    pub fn record(
        &self,
        f: impl FnOnce(&mut recording::ScenarioRecorder) -> Result<(), recording::RecordingError>,
    ) {
        let mut held = self.recording.borrow_mut();
        ok(f(held
            .as_mut()
            .expect("actual source scenario recorder retained")));
    }
    pub fn producer_counts(&self, purpose: &'static str) -> desktop::ProducerCounts {
        let actual = self.h.native_producer_counts();
        self.record(|r| r.producer_counts(purpose, &actual));
        actual
    }
    pub fn denied_at<T, E: RecordedStatus>(
        &self,
        purpose: &'static str,
        call: &'static str,
        result: Result<T, E>,
        expected: E,
    ) {
        match result {
            Err(actual) => {
                assert_eq!(actual, expected);
                self.record(|r| {
                    r.unit_control(
                        purpose,
                        call,
                        "Err",
                        actual.recording_space(),
                        actual.recording_code(),
                        None,
                    )
                });
            }
            Ok(_) => panic!("denial exposed value"),
        }
    }
    pub fn record_grant(&self, phase: &'static str, reply: &Envelope, bytes: &[u8]) {
        let evidence = ok(self.dc.evidence());
        let expected = ok(desktop::encode_envelope_wire(reply));
        let (q, r) = evidence
            .exchanges
            .iter()
            .rev()
            .find(|(_, r)| ok(desktop::encode_envelope_wire(r)) == expected)
            .expect("actual corresponding grant exchange retained");
        self.record(|rec| rec.grant(phase, q, r, bytes));
    }
    pub fn observe(
        &self,
        app: &NativeArtifactEditor,
        now: u64,
    ) -> Result<desktop::NativeEditorObservation, NativeEditorError> {
        let actual = app.observe(now)?;
        self.record(|r| r.draft(&actual));
        Ok(actual)
    }
    pub fn editor_observation(
        &self,
        editor: &desktop::NativeEditor,
        now: u64,
    ) -> Result<desktop::NativeEditorObservation, Status> {
        let actual = self.h.editor_observation(editor, now)?;
        self.record(|r| r.draft(&actual));
        Ok(actual)
    }
    pub fn reattach_input(&mut self, i: usize, now: u64) {
        let (input, device_generation) =
            ok(self.dc.reattach_keyboard(self.drivers[i].s.session_id, now));
        self.drivers[i].s.input = input;
        self.drivers[i].s.device_generation = device_generation;
        self.drivers[i].mods = 0;
        self.drivers[i].attach(&self.h, now);
        // The actual successful Attach creates a new queue with next_sequence=1.
        self.drivers[i].next = 1;
    }
    pub fn compose(&self, i: usize, now: u64) -> Result<desktop::ComposedFrame, Status> {
        let frame = self
            .h
            .compose_native_save(&self.drivers[i].s.compositor, now)?;
        let chrome = ok(self
            .dc
            .save_chrome_observation(&self.drivers[i].s.compositor));
        self.record(|r| r.frame(&frame, None, None, Some(&chrome)));
        Ok(frame)
    }
    pub fn store_snapshot(
        &self,
        purpose: &'static str,
        phase: &'static str,
        fence: Option<&store::NativeSaveFenceSnapshot>,
        failure: Option<&store::FixtureError>,
    ) {
        let actual = ok(self.prepared().controller().evidence());
        let index = self.snapshot_ordinal.get();
        self.snapshot_ordinal.set(
            index
                .checked_add(1)
                .expect("bounded source Store snapshot ID"),
        );
        self.record(|r| {
            r.store_snapshot(
                self.core_ordinal,
                index,
                purpose,
                phase,
                &actual,
                &self.path,
                fence,
                failure,
            )
        });
    }
    pub fn archive_core(&mut self, actual: store::NativeSaveFenceSnapshot) {
        assert!(self.archived_cores.insert(self.core_ordinal));
        let captures = ok(self.prepared().controller().native_save_artifacts());
        self.record(|r| r.captures(self.core_ordinal, &captures));
        self.store_snapshot(
            "before_transfer",
            "after_owned_join_before_transfer",
            Some(&actual),
            None,
        );
        self.recorded_fences.push(actual);
    }
    pub fn replace_prepared(&mut self, actual: store::PreparedNativeSaveStore) {
        assert!(
            self.recorded_quiesce_joins.is_none(),
            "old Core proof cache settled before replacement"
        );
        self.core_ordinal = self
            .core_ordinal
            .checked_add(1)
            .expect("actual new Core ordinal");
        assert!(self.core_ordinal < 4);
        self.snapshot_ordinal.set(0);
        self.prepared = Some(Arc::new(actual));
    }
    pub fn recorded(
        case: &Arc<recording::CaseRecorder>,
        spec: recording::ScenarioSpec,
        n: usize,
    ) -> Self {
        Self::recorded_ttl(case, spec, n, 30000, 600000)
    }
    pub fn recorded_ttl(
        case: &Arc<recording::CaseRecorder>,
        spec: recording::ScenarioSpec,
        n: usize,
        preview: u64,
        instance: u64,
    ) -> Self {
        Self::configured(case, spec, n, preview, instance, profile(n))
    }
    pub fn recorded_profile(
        case: &Arc<recording::CaseRecorder>,
        spec: recording::ScenarioSpec,
        p: store::NativeSaveFixtureProfile,
    ) -> Self {
        Self::configured(case, spec, p.initial_objects.len(), 30000, 600000, p)
    }
    fn configured(
        case: &Arc<recording::CaseRecorder>,
        spec: recording::ScenarioSpec,
        n: usize,
        preview: u64,
        instance: u64,
        p: store::NativeSaveFixtureProfile,
    ) -> Self {
        let (h, dc, witness, enrollment) = ok(desktop::HostDesktop::new_store_save(
            volatile::config(preview, instance),
        ));
        let mut drivers = Vec::new();
        for i in 0..n.min(2) {
            let s = ok(dc.register_store_session(
                i as u64 + 1,
                volatile::APP,
                volatile::MANIFEST,
                101 + i as u64,
                1,
                1,
            ));
            let mut d = volatile::Driver {
                s,
                queue: 0,
                next: 1,
                mods: 0,
            };
            d.attach(&h, 1);
            drivers.push(d);
        }
        let root = temporary();
        let path = root.path().join("owner");
        let prepared = Arc::new(ok(store::StoreFixture::prepare_native_save(
            &path, p, witness, enrollment,
        )));
        let initial = files(&path);
        let this = Self {
            recording: std::cell::RefCell::new(Some(ok(case.scenario(spec)))),
            core_ordinal: 0,
            snapshot_ordinal: std::cell::Cell::new(0),
            recorded_fences: vec![],
            archived_cores: BTreeSet::new(),
            h,
            dc: Arc::new(dc),
            drivers,
            prepared: Some(prepared),
            root: Some(root),
            path,
            initial,
            joined: BTreeSet::new(),
            recorded_quiesce_joins: None,
            desktop_pauses: vec![],
            store_pauses: vec![],
        };
        this.store_snapshot("initial", "source_checkpoint", None, None);
        this
    }
    pub fn prepared(&self) -> &store::PreparedNativeSaveStore {
        self.prepared.as_ref().unwrap()
    }
    pub fn host(&self) -> store::StoreHost {
        self.prepared().host().clone()
    }
    pub fn selector(&mut self, i: usize, now: u64) -> desktop::SaveUiTicket {
        self.drivers[i].launcher(&self.h, now);
        self.drivers[i].send(&self.h, 40, 1, now);
        let ticket = ok(self
            .h
            .take_save_preview_action(&self.drivers[i].s.chrome.peer));
        self.denied_at(
            "helper_selector_replay",
            "take_save_preview_action",
            self.h
                .take_save_preview_action(&self.drivers[i].s.chrome.peer),
            Status::NotReady,
        );
        ticket
    }
    pub fn prepare(
        &mut self,
        i: usize,
        object: u64,
        now: u64,
    ) -> (
        desktop::SaveUiTicket,
        desktop::SaveSelectionPin,
        session::PrepareLaunchReply,
    ) {
        let mut ticket = self.selector(i, now);
        let pin = ok(self
            .prepared()
            .pin_selection(&self.drivers[i].s.chrome.peer, object, 30000));
        let wire = canonical(ok(self.h.prepare_save_preview(
            &self.drivers[i].s.chrome.peer,
            &mut ticket,
            &pin,
            now,
        )));
        let r = ok(session::PrepareLaunchReply::decode(&wire));
        assert_eq!(r.status, 0);
        assert_eq!(r.granted_rights, 63);
        assert_eq!(r.preview_len, 464);
        assert_eq!(wire.handle, Handle::INVALID);
        let bytes = volatile::read(
            &self.h,
            &self.drivers[i].s.chrome,
            &volatile::desc(r.preview_shm, desktop::ObjectKind::Preview, r.preview_len),
            now,
        );
        self.record_grant("Preview", &wire, &bytes);
        assert_eq!(
            r.request_id,
            latest_request::<session::PrepareLaunch>(&ok(self.dc.evidence())).request_id
        );
        self.denied_at(
            "helper_prepare_replay",
            "prepare_save_preview",
            self.h
                .prepare_save_preview(&self.drivers[i].s.chrome.peer, &mut ticket, &pin, now),
            Status::NotReady,
        );
        (ticket, pin, r)
    }
    pub fn approval(&mut self, i: usize, now: u64) -> desktop::SaveUiTicket {
        self.drivers[i].send(&self.h, 40, 2, now);
        self.drivers[i].send(&self.h, 40, 1, now);
        let ticket = ok(self
            .h
            .take_save_preview_action(&self.drivers[i].s.chrome.peer));
        self.drivers[i].send(&self.h, 40, 2, now);
        ticket
    }
    pub fn launch(&mut self, i: usize, object: u64, now: u64) -> store::PairedNativeSaveLaunch {
        let (selector, pin, p) = self.prepare(i, object, now);
        let mut approval = self.approval(i, now);
        let pending =
            ok(self
                .h
                .confirm_save_preview(&self.drivers[i].s.chrome.peer, &mut approval, now));
        self.denied_at(
            "helper_confirm_replay",
            "confirm_save_preview",
            self.h
                .confirm_save_preview(&self.drivers[i].s.chrome.peer, &mut approval, now),
            Status::NotReady,
        );
        let active = ok(self.prepared().activate_save(&pending));
        let mut launch = None;
        ok(self
            .h
            .take_save_launch(&self.drivers[i].s.chrome.peer, &mut launch, now));
        let paired = ok(active.pair_launch(&mut launch));
        assert!(launch.is_none());
        let bootstrap = boot(&paired);
        let bytes = volatile::read(
            &self.h,
            paired.desktop().bindings().self_status(),
            &volatile::desc(
                bootstrap.grants_shm,
                desktop::ObjectKind::Grants,
                bootstrap.grants_len,
            ),
            now,
        );
        self.record_grant("Active", paired.desktop().confirm_reply(), &bytes);
        let reply = ok(session::ConfirmLaunchReply::decode(&canonical(
            *paired.desktop().confirm_reply(),
        )));
        assert_eq!((reply.status, reply.grants_len), (0, 464));
        assert_eq!(
            reply.request_id,
            latest_request::<session::ConfirmLaunch>(&ok(self.dc.evidence())).request_id
        );
        assert!(p.plan_id > 0 && p.preview_revision > 0);
        drop((selector, pin, approval, pending, active));
        paired
    }
    pub fn app(&mut self, i: usize, object: u64, now: u64) -> NativeArtifactEditor {
        self.app_with_endpoint(i, object, now).0
    }
    pub fn app_with_endpoint(
        &mut self,
        i: usize,
        object: u64,
        now: u64,
    ) -> (NativeArtifactEditor, store::EditorEndpoint) {
        let (app, endpoint, _) = self.app_with_controls(i, object, now);
        (app, endpoint)
    }
    pub fn app_with_controls(
        &mut self,
        i: usize,
        object: u64,
        now: u64,
    ) -> (
        NativeArtifactEditor,
        store::EditorEndpoint,
        desktop::Endpoint,
    ) {
        let launch = self.launch(i, object, now);
        let focus = launch.desktop().bindings().focus_read().clone();
        let endpoint = store::EditorEndpoint {
            peer: launch.endpoint().peer.clone(),
            handle: launch.endpoint().handle,
        };
        let mut app = ok(NativeArtifactEditor::open(
            self.h.clone(),
            self.host(),
            launch,
            now,
        ));
        let initial_join = ok(app.render(now));
        let frame = ok(self
            .h
            .compose_native_save(&self.drivers[i].s.compositor, now));
        assert_eq!(frame.session_id, self.drivers[i].s.session_id);
        let obs = ok(app.observe(now));
        let chrome = ok(self
            .dc
            .save_chrome_observation(&self.drivers[i].s.compositor));
        self.record(|r| r.draft(&obs));
        self.record(|r| r.frame(&frame, Some(&obs), Some(&initial_join), Some(&chrome)));
        let r: focus::AssignReply = volatile::call(
            &self.h,
            &self.drivers[i].s.chrome,
            focus::Assign {
                request_id: 300,
                session_id: obs.actor.session_id,
                session_generation: obs.actor.session_generation,
                instance_id: obs.actor.instance_id,
                instance_generation: obs.actor.instance_generation,
                surface_id: volatile::latest::<
                    kernel_api::generated::desktop_surface_v1::CreateReply,
                >(&ok(self.dc.evidence()))
                .surface_id,
                surface_generation: volatile::latest::<
                    kernel_api::generated::desktop_surface_v1::CreateReply,
                >(&ok(self.dc.evidence()))
                .surface_generation,
            },
            now,
        );
        assert_eq!(r.status, 0);
        self.join_all();
        (app, endpoint, focus)
    }
    pub fn stroke(&mut self, i: usize, app: &mut NativeArtifactEditor, usage: u32, now: u64) {
        for phase in [1, 2] {
            self.drivers[i].send(&self.h, usage, phase, now);
            ok(app.step(now));
        }
        self.join_all();
    }
    pub fn chord(&mut self, i: usize, app: &mut NativeArtifactEditor, usage: u32, now: u64) {
        self.drivers[i].send(&self.h, 224, 1, now);
        ok(app.step(now));
        self.stroke(i, app, usage, now);
        self.drivers[i].send(&self.h, 224, 2, now);
        ok(app.step(now));
    }
    pub fn replace(&mut self, i: usize, app: &mut NativeArtifactEditor, body: &[u8], now: u64) {
        self.chord(i, app, 4, now);
        for &b in body {
            let usage = usage(b);
            self.stroke(i, app, usage, now);
        }
        assert_eq!(ok(app.observe(now)).bytes, body);
    }
    pub fn protected_golden(
        &self,
        frame: &desktop::ComposedFrame,
        label: &'static str,
        banner: &'static str,
    ) {
        protected_golden(frame, label, banner);
        self.record(|rec| rec.frame_expectation(frame, label, banner));
    }
    pub fn composed(
        &mut self,
        i: usize,
        app: &mut NativeArtifactEditor,
        now: u64,
    ) -> desktop::ComposedFrame {
        let o = ok(app.observe(now));
        let r = ok(app.render(now));
        assert_eq!(&r.actor, &o.actor);
        assert_eq!(
            (r.document_id, r.text_generation, r.view_version),
            (o.document_id, o.text_generation, o.view_version)
        );
        let f = ok(self
            .h
            .compose_native_save(&self.drivers[i].s.compositor, now));
        assert_eq!(
            (f.instance_id, f.sequence),
            (o.actor.instance_id, r.sequence)
        );
        let current = ok(app.observe(now));
        let chrome = ok(self
            .dc
            .save_chrome_observation(&self.drivers[i].s.compositor));
        self.record(|rec| rec.draft(&current));
        self.record(|rec| rec.frame(&f, Some(&current), Some(&r), Some(&chrome)));
        if matches!(current.phase, desktop::NativeEditorPhase::Saved) {
            saved_publication(self, i, &current, &r, &f);
        }
        f
    }
    pub fn until(
        &mut self,
        app: &mut NativeArtifactEditor,
        now: u64,
        p: impl Fn(&desktop::NativeEditorObservation) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_millis(1500);
        loop {
            ok(app.step(now));
            if p(&ok(app.observe(now))) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "bounded real asynchronous completion"
            );
            thread::sleep(Duration::from_millis(1));
        }
        self.join_all();
    }
    pub fn save(&mut self, i: usize, app: &mut NativeArtifactEditor, now: u64) {
        self.chord(i, app, 22, now);
        self.until(app, now, |o| {
            matches!(
                o.phase,
                desktop::NativeEditorPhase::Unsaved | desktop::NativeEditorPhase::Saved
            ) && o.confirmed_revision > 1
        });
        self.composed(i, app, now);
        assert!(matches!(
            ok(app.observe(now)).phase,
            desktop::NativeEditorPhase::Saved
        ));
    }
    pub fn join(&mut self, id: &desktop::ProducerId) {
        let end = Instant::now() + Duration::from_millis(2000);
        loop {
            match self.h.join_native_finished(id) {
                Ok(proof) => {
                    assert!(matches!(proof.outcome(), desktop::JoinOutcome::Returned));
                    assert!(self.joined.insert(proof.producer_id()));
                    let actual = ok(self.h.native_join_observation(&proof));
                    self.record(|r| r.native_join(&actual));
                    break;
                }
                Err(Status::NotReady) => {
                    assert!(Instant::now() < end);
                    thread::yield_now()
                }
                Err(_) => panic!("authentic producer join failed"),
            }
        }
    }
    fn recorded_quiesce_matches_current(&self) -> bool {
        self.recorded_quiesce_joins.as_ref().is_some_and(|tracked| {
            assert_eq!(tracked.core_ordinal, self.core_ordinal);
            assert!(Arc::ptr_eq(
                &tracked.owner,
                self.prepared
                    .as_ref()
                    .expect("actual current Core retained"),
            ));
            tracked.closed_after_incomplete
        })
    }
    fn retain_quiesce_ids(&mut self, ids: Vec<desktop::ProducerId>) {
        assert!(
            ids.len() <= 64,
            "bounded actual current Core producer aliases"
        );
        if self.recorded_quiesce_joins.is_none() {
            self.recorded_quiesce_joins = Some(RecordedQuiesceJoins {
                owner: self.prepared.as_ref().expect("actual current Core").clone(),
                core_ordinal: self.core_ordinal,
                closed_after_incomplete: false,
                pending: Vec::new(),
            });
        }
        let tracked = self.recorded_quiesce_joins.as_mut().unwrap();
        assert_eq!(tracked.core_ordinal, self.core_ordinal);
        assert!(Arc::ptr_eq(
            &tracked.owner,
            self.prepared.as_ref().expect("actual current Core"),
        ));
        // Preflight the complete unique addition before growing retained storage.
        let mut unique: BTreeSet<_> = tracked
            .pending
            .iter()
            .map(desktop::ProducerId::producer_id)
            .collect();
        assert!(unique.len() <= 64);
        for id in &ids {
            if !self.joined.contains(&id.producer_id()) && !unique.contains(&id.producer_id()) {
                assert!(unique.len() < 64, "retained opaque token cap before growth");
                unique.insert(id.producer_id());
            }
        }
        for id in ids {
            if !self.joined.contains(&id.producer_id())
                && !tracked
                    .pending
                    .iter()
                    .any(|held| held.producer_id() == id.producer_id())
            {
                assert!(tracked.pending.len() < 64);
                tracked.pending.push(id);
            }
        }
    }
    fn collect_quiesce_joins_once(&mut self) {
        if !self.recorded_quiesce_matches_current() {
            return;
        }
        let count = self.recorded_quiesce_joins.as_ref().unwrap().pending.len();
        assert!(count <= 64);
        for index in 0..count {
            let id = &self.recorded_quiesce_joins.as_ref().unwrap().pending[index];
            let expected_id = id.producer_id();
            if self.joined.contains(&expected_id) {
                assert!(matches!(
                    self.host().join_native_save_finished(id),
                    Err(StoreStatus::NotReady)
                ));
                continue;
            }
            match self.host().join_native_save_finished(id) {
                Ok(proof) => {
                    assert_eq!(proof.producer_id(), expected_id);
                    assert!(matches!(proof.outcome(), desktop::JoinOutcome::Returned));
                    let actual = ok(self.h.native_join_observation(&proof));
                    assert_eq!(actual.producer_id, expected_id);
                    assert_eq!(actual.producer_generation, 1);
                    assert!(matches!(actual.outcome, desktop::JoinOutcome::Returned));
                    assert!(self.joined.insert(expected_id));
                    self.record(|r| r.native_join(&actual));
                }
                Err(StoreStatus::NotReady) => {}
                Err(_) => panic!("own retained Core producer proof denied"),
            }
        }
        // NotReady IO aliases stay available, without claiming a completion.
        self.recorded_quiesce_joins
            .as_mut()
            .unwrap()
            .pending
            .retain(|id| !self.joined.contains(&id.producer_id()));
    }
    pub fn quiesce_recorded(
        &mut self,
        timeout_ms: u64,
    ) -> Result<store::QuiescedNativeSaveOwner, store::FixtureError> {
        let ids = ok(self.host().native_save_producers());
        self.retain_quiesce_ids(ids);
        // Preserve exactly one actual controller invocation and its original Result.
        let result = self.prepared().controller().quiesce_native_save(timeout_ms);
        if matches!(&result, Err(error) if error.status == StoreStatus::NotReady) {
            self.recorded_quiesce_joins
                .as_mut()
                .unwrap()
                .closed_after_incomplete = true;
            self.collect_quiesce_joins_once();
        }
        result
    }
    fn drain_recorded_quiesce_joins(&mut self) {
        assert!(self.recorded_quiesce_matches_current());
        let deadline = Instant::now() + Duration::from_millis(2000);
        loop {
            self.join_all();
            if self.h.native_producer_counts().held == 0 {
                // Final authentic alias pass before successful proof transfer.
                self.collect_quiesce_joins_once();
                assert!(Instant::now() < deadline, "bounded recorded Core cleanup");
                break;
            }
            assert!(Instant::now() < deadline, "bounded recorded Core cleanup");
            thread::yield_now();
        }
    }
    pub fn join_all(&mut self) {
        self.collect_quiesce_joins_once();
        for id in ok(self.host().native_save_producers()) {
            match self.host().join_native_save_finished(&id) {
                Ok(proof) => {
                    assert!(matches!(proof.outcome(), desktop::JoinOutcome::Returned));
                    assert!(self.joined.insert(proof.producer_id()));
                    let actual = ok(self.h.native_join_observation(&proof));
                    self.record(|r| r.native_join(&actual));
                }
                Err(StoreStatus::NotReady) => {}
                Err(_) => panic!("own Store join denied"),
            }
        }
        // Remaining Core-associated Desktop pairs must be joined by Core too;
        // entry-only saturation pairs are explicitly joined through Host below.
        for id in self.h.native_producers() {
            match self.host().join_native_save_finished(&id) {
                Ok(proof) => {
                    assert!(matches!(proof.outcome(), desktop::JoinOutcome::Returned));
                    assert!(self.joined.insert(proof.producer_id()));
                    let actual = ok(self.h.native_join_observation(&proof));
                    self.record(|r| r.native_join(&actual));
                }
                Err(StoreStatus::NotReady) => {}
                Err(_) => panic!("own registered shared join denied"),
            }
        }
    }
    pub fn counts(&self, object: u64) -> (u32, u32, u32) {
        let e = ok(self.prepared().controller().evidence());
        let o = e
            .objects
            .iter()
            .find(|v| v.selected.selected_object_id == object)
            .unwrap();
        (
            o.mutation_dispatch_count,
            o.permit_count,
            o.transition_count,
        )
    }
    pub fn unchanged(&self) {
        assert_eq!(files(&self.path), self.initial)
    }
    pub fn reopen(&mut self) {
        let tracked = self.recorded_quiesce_matches_current();
        if tracked {
            self.drain_recorded_quiesce_joins();
        } else {
            self.join_all();
        }
        let token = ok(self.prepared().controller().quiesce_native_save(2000));
        let actual = token.fence_snapshot();
        assert_eq!(actual.base.live_ownership_after_join, 0);
        if tracked {
            // Coverage only: every row must already have an authentic recorded
            // proof. Never construct a caller observation from this snapshot.
            for producer in &actual.base.service_producers {
                assert!(
                    self.joined.contains(&producer.producer_id),
                    "recorded actual service proof before final transfer"
                );
            }
        }
        self.archive_core(actual);
        if tracked {
            self.recorded_quiesce_joins = None;
        }
        let reopened = ok(store::StoreFixture::reopen_native_save(token));
        self.replace_prepared(reopened);
    }
    fn release_owned_pauses(&self) {
        for p in &self.desktop_pauses {
            match p.wait_until_entered(1) {
                Ok(s) if !s.settled => ok(self.dc.release_native_save(p)),
                Ok(_) | Err(Status::Timeout) => {}
                Err(_) => panic!("own Desktop pause observation failed"),
            }
        }
        for (owner, p) in &self.store_pauses {
            match p.wait_until_entered(1) {
                Ok(s) if !s.released && !s.settled => ok(owner.controller().release(p)),
                Ok(_) => {}
                Err(e) if e.status == StoreStatus::Timeout => {}
                Err(_) => panic!("own Store pause observation failed"),
            }
        }
    }
    pub fn finish(mut self) {
        let held_before = self.h.native_producer_counts().held;
        self.release_owned_pauses();
        let deadline = Instant::now() + Duration::from_millis(2000);
        while self.h.native_producer_counts().held > 0 {
            self.join_all();
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        assert_eq!(self.h.native_producer_counts().held, 0);
        let fence = ok(self.prepared().controller().quiesce_native_save(2000));
        assert_eq!(fence.fence_snapshot().base.live_ownership_after_join, 0);
        let snap = fence.fence_snapshot();
        if self.archived_cores.insert(self.core_ordinal) {
            let captures = ok(self.prepared().controller().native_save_artifacts());
            self.record(|r| r.captures(self.core_ordinal, &captures));
        }
        self.store_snapshot("final", "final_before_cleanup", Some(&snap), None);
        self.recorded_fences.push(snap);
        let desktop = ok(self.dc.evidence());
        self.record(|r| r.desktop(&desktop));
        let actual_counts = self.h.native_producer_counts();
        self.record(|r| r.producer_counts("final", &actual_counts));
        drop(fence);
        self.prepared.take();
        let root = self.root.take().unwrap();
        let recorder = self
            .recording
            .borrow_mut()
            .take()
            .expect("owned recorder final transfer");
        ok(recorder.finish_native_save(
            root,
            held_before,
            actual_counts.held,
            &self.recorded_fences,
            false,
        ));
    }
    pub fn finish_reopen_failure(mut self, failure: store::NativeSaveReopenFailure) {
        assert_eq!(self.h.native_producer_counts().held, 0);
        let actual = failure.owner.fence_snapshot();
        self.store_snapshot(
            "final",
            "actual_failed_reopen_closed",
            Some(&actual),
            Some(&failure.error),
        );
        let desktop = ok(self.dc.evidence());
        self.record(|r| r.desktop(&desktop));
        self.record(|r| r.producer_counts("final", &self.h.native_producer_counts()));
        drop(failure);
        self.prepared.take();
        let root = self.root.take().expect("actual owned failed-reopen root");
        let recorder = self
            .recording
            .borrow_mut()
            .take()
            .expect("owned failed-reopen recorder");
        ok(recorder.finish_native_save(root, 0, 0, &self.recorded_fences, true));
    }
}
pub fn usage(b: u8) -> u32 {
    match b {
        b'a'..=b'z' => 4 + (b - b'a') as u32,
        b'1'..=b'9' => 30 + (b - b'1') as u32,
        b'0' => 39,
        b'\n' => 40,
        b'\t' => 43,
        b' ' => 44,
        b'=' => 46,
        _ => panic!("explicit ASCII producer corpus has no usage for byte"),
    }
}
pub fn boot(p: &store::PairedNativeSaveLaunch) -> session::InstanceBootstrap {
    ok(session::InstanceBootstrap::decode(&canonical(
        *p.desktop().bindings().bootstrap(),
    )))
}
pub fn selected_request(p: &store::PairedNativeSaveLaunch, id: u64) -> Envelope {
    let b = boot(p);
    canonical(ok(artifact::ReadSelected {
        request_id: id,
        session_id: b.session_id,
        session_generation: b.session_generation,
        instance_id: b.instance_id,
        instance_generation: b.instance_generation,
    }
    .encode(p.endpoint().handle)))
}
pub fn selected(
    f: &mut Fixture,
    p: &store::PairedNativeSaveLaunch,
    id: u64,
    now: u64,
) -> (u64, Vec<u8>) {
    let q = selected_request(p, id);
    let entry =
        ok(f.h.begin_save_read(p.desktop().bindings().artifact_origin(), &q, now));
    let reg = ok(f
        .host()
        .register_native_save(p.endpoint(), ok(entry.wait_once())));
    let reply = ok(f.host().execute_native_save(&reg));
    let r = ok(artifact::ReadSelectedReply::decode(reply.wire()));
    assert_eq!(r.status, 0);
    let lease = reply.selected().unwrap();
    let mut b = vec![0xa5; r.byte_len as usize];
    ok(lease.copy_into(0, &mut b));
    let header = ok(EditorTextHeaderV0::decode_le(&b[..TEXT_HEADER_LEN]));
    ok(header.validate_bytes(&b[TEXT_HEADER_LEN..]));
    assert!(header.selected_object_id >= 101 && header.selected_object_id <= 103);
    let result = (header.revision, b[TEXT_HEADER_LEN..].to_vec());
    drop((reply, reg, entry));
    f.join_all();
    result
}

pub fn text_bytes(object: u64, base: u64, body: &[u8]) -> Vec<u8> {
    let header = EditorTextHeaderV0 {
        schema_version: 1,
        total_len: (64 + body.len()) as u32,
        selected_object_id: object,
        revision: base,
        content_hash: Sha256::digest(body).into(),
        byte_len: body.len() as u32,
        reserved: 0,
    };
    let mut bytes = ok(header.encode_le()).to_vec();
    bytes.extend_from_slice(body);
    bytes
}
pub struct RawEditor {
    pub launch: store::PairedNativeSaveLaunch,
    pub editor: desktop::NativeEditor,
}
pub fn raw_editor(f: &mut Fixture, i: usize, object: u64, now: u64) -> RawEditor {
    let launch = f.launch(i, object, now);
    let q = selected_request(&launch, 400);
    let entry =
        ok(f.h.begin_save_read(launch.desktop().bindings().artifact_origin(), &q, now));
    let reg = ok(f
        .host()
        .register_native_save(launch.endpoint(), ok(entry.wait_once())));
    let mut reply = ok(f.host().execute_native_save(&reg));
    let editor = ok(reply.take_editor());
    f.denied_at(
        "helper_editor_take_replay",
        "NativeSaveReply.take_editor",
        reply.take_editor(),
        StoreStatus::NotReady,
    );
    for id in entry.producer_ids() {
        join_core(f, id);
    }
    for id in reply.producers() {
        join_core(f, id);
    }
    let initial_frame = ok(f.h.render_native_editor(&editor, now));
    let composed = ok(f.h.compose_native_save(&f.drivers[i].s.compositor, now));
    let current = ok(f.editor_observation(&editor, now));
    let chrome = ok(f.dc.save_chrome_observation(&f.drivers[i].s.compositor));
    f.record(|r| {
        r.frame(
            &composed,
            Some(&current),
            Some(&initial_frame),
            Some(&chrome),
        )
    });
    let actor = current.actor;
    let r: focus::AssignReply = volatile::call(
        &f.h,
        &f.drivers[i].s.chrome,
        focus::Assign {
            request_id: 401,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            instance_id: actor.instance_id,
            instance_generation: actor.instance_generation,
            surface_id: initial_frame.surface_id,
            surface_generation: initial_frame.surface_generation,
        },
        now,
    );
    assert_eq!(r.status, 0);
    drop((reply, reg, entry));
    RawEditor { launch, editor }
}
pub fn join_core(f: &mut Fixture, id: &desktop::ProducerId) {
    if f.joined.contains(&id.producer_id()) {
        assert!(matches!(
            f.host().join_native_save_finished(id),
            Err(StoreStatus::NotReady)
        ));
        return;
    }
    let _ = join_core_observed(f, id);
}
pub fn join_core_observed(f: &mut Fixture, id: &desktop::ProducerId) -> u64 {
    let end = Instant::now() + Duration::from_millis(2000);
    loop {
        match f.host().join_native_save_finished(id) {
            Ok(p) => {
                assert!(matches!(p.outcome(), desktop::JoinOutcome::Returned));
                assert!(f.joined.insert(p.producer_id()));
                let actual = ok(f.h.native_join_observation(&p));
                f.record(|r| r.native_join(&actual));
                return actual.producer_id;
            }
            Err(StoreStatus::NotReady) => {
                assert!(Instant::now() < end);
                thread::yield_now()
            }
            Err(_) => panic!("real Core-associated join failed"),
        }
    }
}
pub fn raw_stroke(f: &mut Fixture, i: usize, raw: &RawEditor, usage: u32, now: u64) {
    for phase in [1, 2] {
        f.drivers[i].send(&f.h, usage, phase, now);
        ok(f.h.step_native_editor(&raw.editor, now));
    }
}
pub fn raw_chord(f: &mut Fixture, i: usize, raw: &RawEditor, usage: u32, now: u64) {
    f.drivers[i].send(&f.h, 224, 1, now);
    ok(f.h.step_native_editor(&raw.editor, now));
    raw_stroke(f, i, raw, usage, now);
    f.drivers[i].send(&f.h, 224, 2, now);
    ok(f.h.step_native_editor(&raw.editor, now));
}
pub fn raw_replace(f: &mut Fixture, i: usize, raw: &RawEditor, body: &[u8], now: u64) {
    raw_chord(f, i, raw, 4, now);
    for &b in body {
        raw_stroke(f, i, raw, usage(b), now);
    }
    assert_eq!(ok(f.h.editor_observation(&raw.editor, now)).bytes, body);
}
pub struct Attempt {
    pub intent: desktop::NativeSaveIntent,
    #[allow(dead_code)] // Retain the actual original owner until Attempt drops.
    pub alloc_entry: desktop::NativeSaveEntry,
    #[allow(dead_code)] // Retain the actual original owner until Attempt drops.
    pub alloc: store::RegisteredNativeSave,
    pub alloc_reply: store::NativeSaveReply,
    pub source: store::NativeDraftSource,
    pub commit_entry: desktop::NativeSaveEntry,
    pub commit: store::RegisteredNativeSave,
    pub request: Envelope,
    pub operation: u64,
}
pub fn attempt(f: &mut Fixture, i: usize, raw: &RawEditor, id: u64, now: u64) -> Attempt {
    raw_chord(f, i, raw, 22, now);
    let intent = ok(f.h.take_save_intent(&raw.editor));
    f.denied_at(
        "helper_intent_take_replay",
        "take_save_intent",
        f.h.take_save_intent(&raw.editor),
        Status::NotReady,
    );
    let v = intent.snapshot().view();
    assert_eq!(
        intent.deadline().duration_since(intent.entered()),
        Duration::from_millis(1000)
    );
    let q = canonical(ok(artifact::AllocateSaveId {
        request_id: id,
        session_id: v.actor.session_id,
        session_generation: v.actor.session_generation,
        expected_revision: v.expected_revision,
    }
    .encode(raw.launch.endpoint().handle)));
    let alloc_entry = ok(f.h.begin_save_allocate(
        raw.launch.desktop().bindings().artifact_origin(),
        &intent,
        &q,
        now,
    ));
    let alloc = ok(f
        .host()
        .register_native_save(raw.launch.endpoint(), ok(alloc_entry.wait_once())));
    let alloc_reply = ok(f.host().execute_native_save(&alloc));
    let a = ok(artifact::AllocateSaveIdReply::decode(alloc_reply.wire()));
    assert_eq!((a.request_id, a.status), (id, 0));
    assert!(a.operation_id > 0);
    let source = ok(f.host().source_for_intent(
        raw.launch.endpoint(),
        alloc_reply.allocation().unwrap(),
        &intent,
    ));
    let descriptor = source.descriptor();
    let bytes = text_bytes(v.object_id, v.expected_revision, intent.snapshot().body());
    assert_eq!(descriptor.byte_len as usize, bytes.len());
    ok(ok(source.write_lease()).copy_from(0, &bytes));
    let request = canonical(ok(artifact::Commit {
        request_id: id + 1,
        session_id: v.actor.session_id,
        session_generation: v.actor.session_generation,
        expected_revision: v.expected_revision,
        operation_id: a.operation_id,
        source_shm: descriptor.handle.pack(),
        byte_len: descriptor.byte_len,
        reserved: 0,
    }
    .encode(raw.launch.endpoint().handle)));
    let commit_entry = ok(f.h.begin_save_commit(
        raw.launch.desktop().bindings().artifact_origin(),
        source.bound(),
        &request,
        now,
    ));
    let commit = ok(f
        .host()
        .register_native_save(raw.launch.endpoint(), ok(commit_entry.wait_once())));
    for id in alloc_entry.producer_ids() {
        join_core(f, id)
    }
    for id in alloc_reply.producers() {
        join_core(f, id)
    }
    Attempt {
        intent,
        alloc_entry,
        alloc,
        alloc_reply,
        source,
        commit_entry,
        commit,
        request,
        operation: a.operation_id,
    }
}
pub fn receipt(reply: &store::NativeSaveReply, op: u64) -> (EditorSaveReceiptV0, Vec<u8>) {
    let wire = ok(artifact::CommitReply::decode(reply.wire()));
    assert_eq!(
        (wire.status, wire.operation_id, wire.receipt_len),
        (0, op, 176)
    );
    let lease = reply.receipt().unwrap();
    let d = lease.descriptor();
    assert_eq!(
        (d.handle.pack(), d.object_generation, d.byte_len),
        (wire.receipt_shm, d.handle.generation, 176)
    );
    let mut bytes = vec![0xa5; 176];
    ok(lease.copy_into(0, &mut bytes));
    let record = ok(EditorSaveReceiptV0::decode_le(&bytes));
    assert_eq!(record.operation_id, op);
    (record, bytes)
}
pub fn settle(f: &mut Fixture, attempt: &Attempt) -> store::NativeSaveReply {
    let reply = ok(f.host().execute_native_save(&attempt.commit));
    for id in attempt.commit_entry.producer_ids() {
        join_core(f, id)
    }
    for id in reply.producers() {
        join_core(f, id)
    }
    reply
}
pub fn object(f: &Fixture, id: u64) -> store::ObjectEvidence {
    ok(f.prepared().controller().evidence())
        .objects
        .into_iter()
        .find(|o| o.selected.selected_object_id == id)
        .unwrap()
}
pub fn revoke(f: &Fixture, i: usize, actor: &EditorActorV0, now: u64) {
    let q = ok(session::RevokeInstance {
        request_id: 900,
        session_id: actor.session_id,
        session_generation: actor.session_generation,
        instance_id: actor.instance_id,
        instance_generation: actor.instance_generation,
    }
    .encode(f.drivers[i].s.chrome.handle));
    let r =
        f.h.dispatch_store_save_chrome(&f.drivers[i].s.chrome.peer, &q, now);
    let r = ok(session::RevokeInstanceReply::decode(&r));
    assert_eq!(r.status, 0);
}
pub fn no_unknown(f: &Fixture, i: usize) {
    let o = ok(f.dc.save_chrome_observation(&f.drivers[i].s.compositor));
    assert_eq!(o.outstanding_original_count, 0);
    assert!(o.pending_originals.is_empty());
}
pub fn unknown(f: &Fixture, i: usize, op: Option<u64>) {
    let o = ok(f.dc.save_chrome_observation(&f.drivers[i].s.compositor));
    assert_eq!(
        o.outstanding_original_count,
        o.pending_originals.len() as u32
    );
    assert!(o.pending_originals.iter().any(|r| r.operation_id == op));
    for r in o.pending_originals {
        assert_eq!(r.original_actor.session_id, f.drivers[i].s.session_id);
        assert!(r.intent_id > 0 && r.original_backend_epoch > 0);
    }
}
mod protected_goldens;
pub fn protected(f: &desktop::ComposedFrame) -> Vec<u8> {
    assert_eq!(f.bgra.len(), 640 * 568 * 4);
    let mut b = f.bgra[..48 * 2560].to_vec();
    b.extend_from_slice(&f.bgra[528 * 2560..]);
    b
}
pub fn protected_golden(frame: &desktop::ComposedFrame, label: &str, banner: &str) {
    let expected = protected_goldens::PROTECTED_GOLDENS
        .iter()
        .find(|(l, b, _)| *l == label && *b == banner)
        .unwrap();
    assert_eq!(volatile::hash(&protected(frame)), expected.2);
}
pub struct OwnedCaller<T> {
    handle: Option<std::thread::JoinHandle<T>>,
    release: Option<Box<dyn FnOnce() + Send>>,
    pub actual_thread_id: String,
}
impl<T: Send + 'static> OwnedCaller<T> {
    pub fn start(
        body: impl FnOnce() -> T + Send + 'static,
        release: impl FnOnce() + Send + 'static,
    ) -> Self {
        let handle = thread::spawn(body);
        let actual_thread_id = format!("{:?}", handle.thread().id());
        Self {
            handle: Some(handle),
            release: Some(Box::new(release)),
            actual_thread_id,
        }
    }
    pub fn join(mut self, fixture: &Fixture) -> T {
        let deadline = Instant::now() + Duration::from_millis(2000);
        while !self.handle.as_ref().unwrap().is_finished() {
            assert!(
                Instant::now() < deadline,
                "owned external caller settled deadline"
            );
            thread::yield_now();
        }
        let h = self.handle.take().unwrap();
        self.release.take();
        let actual = h.join();
        fixture.record(|r| r.external_join(&self.actual_thread_id, actual.is_err()));
        match actual {
            Ok(v) => v,
            Err(e) => std::panic::resume_unwind(e),
        }
    }
}
impl<T> Drop for OwnedCaller<T> {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            release();
        }
        if let Some(handle) = self.handle.take() {
            let deadline = Instant::now() + Duration::from_millis(2000);
            while !handle.is_finished() && Instant::now() < deadline {
                thread::yield_now();
            }
            if handle.is_finished() {
                let _ = handle.join();
            } else {
                std::mem::forget(handle);
                eprintln!("INCOMPLETE: supported caller did not reap; no cleanup claim");
            }
        }
    }
}
pub fn desktop_pause(
    f: &mut Fixture,
    i: usize,
    p: desktop::NativeSavePausePoint,
) -> Arc<desktop::NativeSavePause> {
    let p = Arc::new(ok(f
        .dc
        .pause_next_native_save(f.drivers[i].s.session_id, p)));
    f.desktop_pauses.push(p.clone());
    p
}
pub fn store_pause(
    f: &mut Fixture,
    raw: &RawEditor,
    p: store::PausePoint,
) -> Arc<store::PauseToken> {
    let p = Arc::new(ok(f
        .prepared()
        .controller()
        .pause_native_save(raw.launch.endpoint(), p)));
    f.store_pauses
        .push((f.prepared.as_ref().unwrap().clone(), p.clone()));
    p
}
pub fn join_attempt(f: &mut Fixture, a: &Attempt, r: &store::NativeSaveReply) {
    for id in a.commit_entry.producer_ids() {
        join_core(f, id)
    }
    for id in r.producers() {
        join_core(f, id)
    }
}
pub fn pending_reply(
    f: &Fixture,
    execution: &store::NativeSaveExecution,
) -> store::NativeSaveReply {
    let deadline = Instant::now() + Duration::from_millis(1500);
    loop {
        if let Some(r) = ok(execution.poll_once()) {
            f.denied_at(
                "helper_execution_poll_replay",
                "NativeSaveExecution.poll_once",
                execution.poll_once(),
                StoreStatus::NotReady,
            );
            return r;
        }
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
}
pub fn observe_commit(
    f: &mut Fixture,
    raw: &RawEditor,
    a: &Attempt,
    id: u64,
    now: u64,
) -> store::NativeOriginalReply {
    let b = boot(&raw.launch);
    let q = canonical(ok(artifact::SaveStatus {
        request_id: id,
        session_id: b.session_id,
        session_generation: b.session_generation,
        operation_id: a.operation,
    }
    .encode(raw.launch.endpoint().handle)));
    let entry =
        ok(f.h.begin_save_recovery(a.commit.original().observation_origin(), &q, now));
    let r = ok(f
        .host()
        .observe_original(ok(entry.wait_once()), a.commit.original()));
    for pid in entry.producer_ids() {
        join_core(f, pid)
    }
    for pid in r.producers() {
        join_core(f, pid)
    }
    r
}
pub fn original_receipt(r: &store::NativeOriginalReply) -> (EditorSaveReceiptV0, Vec<u8>) {
    match r.observation() {
        store::NativeOriginalObservation::Receipt { binding, receipt } => {
            let mut b = vec![0; 176];
            ok(receipt.copy_into(0, &mut b));
            let decoded = ok(EditorSaveReceiptV0::decode_le(&b));
            ok(decoded.validate_binding(binding));
            (decoded, b)
        }
        _ => panic!("actual original did not return a receipt"),
    }
}
pub fn capture_signature(rows: &[store::NativeSaveArtifactCapture]) -> Vec<serde_json::Value> {
    assert!(rows.len() <= 256);
    rows.iter().map(|r| serde_json::json!({
        "capture_id": r.capture_id,
        "kind": match r.kind { store::NativeSaveArtifactKind::SelectedTextCopy => 0,
            store::NativeSaveArtifactKind::DraftSourceFreeze => 1, store::NativeSaveArtifactKind::ReceiptCopy => 2 },
        "actor": r.actor, "object_id": r.object_id, "object_generation": r.object_generation,
        "original_backend_epoch": r.original_backend_epoch,
        "original_request": r.original_request.as_slice(),
        "original_reply": r.original_reply.as_ref().map(|b| b.as_slice()),
        "capture_request": r.capture_request.as_slice(),
        "capture_reply": r.capture_reply.as_ref().map(|b| b.as_slice()),
        "operation_id": r.operation_id, "intent_id": r.intent_id,
        "descriptor": { "kind": match r.descriptor.kind { store::SharedObjectKind::SelectedText => 0,
            store::SharedObjectKind::DraftSource => 1, store::SharedObjectKind::Receipt => 2 },
            "handle": r.descriptor.handle.pack(), "generation": r.descriptor.object_generation, "byte_len": r.descriptor.byte_len },
        "bytes": r.bytes
    })).collect()
}

pub fn saved_publication(
    f: &Fixture,
    i: usize,
    current: &desktop::NativeEditorObservation,
    rendered: &desktop::NativeFrameObservation,
    composed: &desktop::ComposedFrame,
) {
    use artifact_store_schema::editor_native_save::{
        EditorNativeSaveChromeStateV0, EditorNativeSaveChromeV0,
    };
    let chrome = ok(f.dc.save_chrome_observation(&f.drivers[i].s.compositor));
    let bytes = ok(f.dc.save_chrome_record(&f.drivers[i].s.compositor));
    assert_eq!(chrome.record248, Some(bytes));
    assert!(matches!(chrome.label, desktop::NativeEditorPhase::Saved));
    assert!(matches!(
        chrome.current_document_phase,
        desktop::NativeEditorPhase::Saved
    ));
    assert_eq!(&chrome.current_frame_actor, &current.actor);
    assert_eq!(&rendered.actor, &current.actor);
    assert_eq!(
        (
            chrome.document_id,
            chrome.text_generation,
            chrome.view_version
        ),
        (
            Some(current.document_id),
            Some(current.text_generation),
            Some(current.view_version)
        )
    );
    assert_eq!(
        (
            rendered.document_id,
            rendered.text_generation,
            rendered.view_version
        ),
        (
            current.document_id,
            current.text_generation,
            current.view_version
        )
    );
    assert_eq!(
        (chrome.frame_sequence, chrome.focus_epoch),
        (rendered.sequence, rendered.focus_epoch)
    );
    assert_eq!(
        (composed.sequence, composed.instance_id),
        (rendered.sequence, current.actor.instance_id)
    );
    assert_eq!(
        rendered.app_crop_sha256,
        <Sha256 as Digest>::digest(&composed.bgra[48 * 2560..528 * 2560]).as_slice()
    );
    let record = ok(EditorNativeSaveChromeV0::decode_le(&bytes));
    assert_eq!(record.state, EditorNativeSaveChromeStateV0::Committed);
    assert_eq!(
        (record.attachment_version, record.status_version),
        (chrome.attachment_version, chrome.status_version)
    );
    assert_eq!(chrome.original_actor.as_ref(), Some(&record.actor));
    let actual = object(f, record.selected_object_id);
    let row = actual
        .operations
        .iter()
        .find(|r| r.allocation.operation_id == record.operation_id)
        .unwrap();
    let binding = row.binding.as_ref().unwrap();
    let captures = ok(f.prepared().controller().native_save_artifacts());
    let receipt = captures
        .iter()
        .find(|r| {
            matches!(r.kind, store::NativeSaveArtifactKind::ReceiptCopy)
                && r.operation_id == Some(record.operation_id)
                && r.actor == record.actor
        })
        .unwrap();
    ok(record.validate_binding(binding, receipt.original_backend_epoch));
    ok(record.validate_receipt(binding, receipt.original_backend_epoch, &receipt.bytes));
    assert_eq!(record.current_store_epoch, actual.service_epoch);
    assert_eq!(record.result_revision, current.confirmed_revision);
    assert_eq!(
        (
            binding.source.byte_len as usize,
            binding.source.content_hash
        ),
        (
            current.bytes.len(),
            <Sha256 as Digest>::digest(&current.bytes).into()
        )
    );
    assert_eq!(current.confirmed_hash, binding.source.content_hash);
    f.record(|r| r.saved_frame_witness(composed, current, rendered));
}

pub fn app_chain_artifacts(
    f: &Fixture,
    object_id: u64,
    body: &[u8],
    operation: u64,
    actor: &EditorActorV0,
    endpoint_handle: Handle,
) {
    let rows = ok(f.prepared().controller().native_save_artifacts());
    assert!(rows.len() <= 256);
    let mut ids = BTreeSet::new();
    for r in &rows {
        assert!(r.capture_id > 0 && ids.insert(r.capture_id));
        assert_eq!(r.descriptor.byte_len as usize, r.bytes.len());
        assert!(r.descriptor.handle.index <= u16::MAX as u32);
        assert!(
            r.descriptor.handle.generation > 0 && r.descriptor.handle.generation <= u32::MAX as u64
        );
        assert_eq!(
            r.descriptor.handle.generation,
            r.descriptor.object_generation
        );
        assert!(r.original_backend_epoch > 0);
        let original = ok(desktop::decode_envelope_wire(&r.original_request));
        let capture = ok(desktop::decode_envelope_wire(&r.capture_request));
        assert_eq!(
            ok(desktop::encode_envelope_wire(&original)),
            r.original_request
        );
        assert_eq!(
            ok(desktop::encode_envelope_wire(&capture)),
            r.capture_request
        );
        assert_eq!((original.protocol, capture.protocol), (368, 368));
        if r.object_id == object_id && r.actor == *actor {
            assert_eq!(original.handle, endpoint_handle);
            assert_eq!(capture.handle, endpoint_handle);
        }
        if let Some(w) = r.original_reply {
            let decoded = ok(desktop::decode_envelope_wire(&w));
            assert_eq!(ok(desktop::encode_envelope_wire(&decoded)), w);
        }
        if let Some(w) = r.capture_reply {
            let decoded = ok(desktop::decode_envelope_wire(&w));
            assert_eq!(ok(desktop::encode_envelope_wire(&decoded)), w);
        }
    }
    let durable = object(f, object_id);
    let row = durable
        .operations
        .iter()
        .find(|r| r.allocation.operation_id == operation)
        .unwrap();
    let binding = row.binding.as_ref().unwrap();
    let source = rows
        .iter()
        .find(|r| {
            r.object_id == object_id
                && r.operation_id == Some(operation)
                && matches!(r.kind, store::NativeSaveArtifactKind::DraftSourceFreeze)
        })
        .unwrap();
    assert!((64..=4160).contains(&source.bytes.len()));
    let header = ok(EditorTextHeaderV0::decode_le(&source.bytes[..64]));
    ok(header.validate_bytes(&source.bytes[64..]));
    assert_eq!(&source.bytes[64..], body);
    assert_eq!(
        (header.selected_object_id, header.revision),
        (object_id, binding.allocation.expected_revision)
    );
    assert_eq!(&source.actor, actor);
    assert_eq!(source.actor, binding.allocation.actor);
    assert_eq!(
        source.object_generation,
        binding.allocation.selected_generation
    );
    assert_eq!(
        (
            source.descriptor.handle.pack(),
            source.descriptor.object_generation,
            body.len() as u32,
            header.content_hash
        ),
        (
            binding.source.source_object_id,
            binding.source.source_generation,
            binding.source.byte_len,
            binding.source.content_hash
        )
    );
    assert!(source.intent_id.is_some());
    assert!(source.capture_reply.is_none());
    let captured = rows
        .iter()
        .find(|r| {
            r.object_id == object_id
                && r.operation_id == Some(operation)
                && matches!(r.kind, store::NativeSaveArtifactKind::ReceiptCopy)
        })
        .unwrap();
    assert_eq!(captured.bytes.len(), 176);
    let receipt = ok(EditorSaveReceiptV0::decode_le(&captured.bytes));
    ok(receipt.validate_binding(binding));
    assert_eq!(receipt, *row.receipt.as_ref().unwrap());
    assert_eq!(
        (receipt.operation_id, receipt.backend_service_epoch),
        (operation, captured.original_backend_epoch)
    );
    assert_eq!(
        row.submitted_service_epoch,
        Some(captured.original_backend_epoch)
    );
    assert_eq!(
        source.original_backend_epoch,
        captured.original_backend_epoch
    );
    assert_eq!(source.actor, captured.actor);
    assert_eq!(source.intent_id, captured.intent_id);
    let original = ok(desktop::decode_envelope_wire(&captured.original_request));
    let q = ok(artifact::Commit::decode(&original));
    assert_eq!(source.original_request, captured.original_request);
    assert_eq!(source.capture_request, captured.original_request);
    assert_eq!(
        (q.session_id, q.session_generation),
        (actor.session_id, actor.session_generation)
    );
    assert_eq!(
        (
            q.operation_id,
            q.expected_revision,
            q.source_shm,
            q.byte_len
        ),
        (
            operation,
            binding.allocation.expected_revision,
            binding.source.source_object_id,
            binding.source.byte_len + 64
        )
    );
    let original_reply = ok(desktop::decode_envelope_wire(
        &captured.original_reply.unwrap(),
    ));
    let original_reply = ok(artifact::CommitReply::decode(&original_reply));
    assert_eq!(
        (original_reply.request_id, original_reply.operation_id),
        (q.request_id, operation)
    );
    let capture_request = ok(desktop::decode_envelope_wire(&captured.capture_request));
    let capture_reply = ok(desktop::decode_envelope_wire(
        &captured.capture_reply.unwrap(),
    ));
    match capture_request.msg_type {
        5 => {
            let q = ok(artifact::Commit::decode(&capture_request));
            assert_eq!(captured.capture_request, captured.original_request);
            assert_eq!(
                (q.session_id, q.session_generation),
                (actor.session_id, actor.session_generation)
            );
            let r = ok(artifact::CommitReply::decode(&capture_reply));
            assert_eq!(
                (
                    q.operation_id,
                    r.operation_id,
                    r.request_id,
                    r.status,
                    r.revision,
                    r.receipt_shm,
                    r.receipt_len
                ),
                (
                    operation,
                    operation,
                    q.request_id,
                    0,
                    receipt.result_revision,
                    captured.descriptor.handle.pack(),
                    176
                )
            );
        }
        7 => {
            let q = ok(artifact::SaveStatus::decode(&capture_request));
            assert_eq!(
                (q.session_id, q.session_generation),
                (actor.session_id, actor.session_generation)
            );
            assert_ne!(q.request_id, original_reply.request_id);
            let r = ok(artifact::SaveStatusReply::decode(&capture_reply));
            assert_eq!(
                (
                    q.operation_id,
                    r.operation_id,
                    r.request_id,
                    r.status,
                    r.revision,
                    r.receipt_shm,
                    r.receipt_len
                ),
                (
                    operation,
                    operation,
                    q.request_id,
                    0,
                    receipt.result_revision,
                    captured.descriptor.handle.pack(),
                    176
                )
            );
        }
        _ => panic!("receipt copied outside actual Commit/SaveStatus"),
    }
    let selected = rows
        .iter()
        .find(|r| {
            r.object_id == object_id
                && r.actor == source.actor
                && matches!(r.kind, store::NativeSaveArtifactKind::SelectedTextCopy)
        })
        .unwrap();
    assert!((64..=4160).contains(&selected.bytes.len()));
    let h = ok(EditorTextHeaderV0::decode_le(&selected.bytes[..64]));
    ok(h.validate_bytes(&selected.bytes[64..]));
    assert_eq!(
        (h.selected_object_id, h.revision, h.content_hash),
        (
            object_id,
            binding.allocation.expected_revision,
            binding.allocation.expected_content_hash
        )
    );
    let q = ok(desktop::decode_envelope_wire(&selected.capture_request));
    let q = ok(artifact::ReadSelected::decode(&q));
    assert_eq!(
        (
            q.session_id,
            q.session_generation,
            q.instance_id,
            q.instance_generation
        ),
        (
            actor.session_id,
            actor.session_generation,
            actor.instance_id,
            actor.instance_generation
        )
    );
    let r = ok(desktop::decode_envelope_wire(
        &selected.capture_reply.unwrap(),
    ));
    let r = ok(artifact::ReadSelectedReply::decode(&r));
    assert_eq!(
        (
            r.request_id,
            r.status,
            r.revision,
            r.data_shm,
            r.object_generation,
            r.byte_len
        ),
        (
            q.request_id,
            0,
            h.revision,
            selected.descriptor.handle.pack(),
            selected.descriptor.object_generation,
            selected.descriptor.byte_len
        )
    );
}

pub struct Allocation {
    pub intent: desktop::NativeSaveIntent,
    pub entry: desktop::NativeSaveEntry,
    pub registered: store::RegisteredNativeSave,
    pub request: Envelope,
}
pub fn allocate_start(f: &mut Fixture, i: usize, raw: &RawEditor, id: u64, now: u64) -> Allocation {
    raw_chord(f, i, raw, 22, now);
    let intent = ok(f.h.take_save_intent(&raw.editor));
    let s = intent.snapshot().view();
    let request = canonical(ok(artifact::AllocateSaveId {
        request_id: id,
        session_id: s.actor.session_id,
        session_generation: s.actor.session_generation,
        expected_revision: s.expected_revision,
    }
    .encode(raw.launch.endpoint().handle)));
    let entry = ok(f.h.begin_save_allocate(
        raw.launch.desktop().bindings().artifact_origin(),
        &intent,
        &request,
        now,
    ));
    let registered = ok(f
        .host()
        .register_native_save(raw.launch.endpoint(), ok(entry.wait_once())));
    Allocation {
        intent,
        entry,
        registered,
        request,
    }
}
pub fn observe_allocate(
    f: &mut Fixture,
    a: &Allocation,
    id: u64,
    now: u64,
) -> store::NativeOriginalReply {
    let s = a.intent.snapshot().view();
    let q = canonical(ok(artifact::ObserveAllocation {
        request_id: id,
        session_id: s.actor.session_id,
        session_generation: s.actor.session_generation,
        original_allocate_request_id: volatile::u64_at(&a.request.payload, 0),
    }
    .encode(a.request.handle)));
    let entry =
        ok(f.h.begin_save_recovery(a.registered.original().observation_origin(), &q, now));
    let reply = ok(f
        .host()
        .observe_original(ok(entry.wait_once()), a.registered.original()));
    for pid in entry.producer_ids() {
        join_core(f, pid)
    }
    for pid in reply.producers() {
        join_core(f, pid)
    }
    reply
}
pub fn read_reply_status(reply: &store::NativeSaveReply, expected: StoreStatus) {
    let r = ok(artifact::ReadSelectedReply::decode(reply.wire()));
    assert_eq!(r.status, expected as u32);
    assert_eq!(reply.wire().handle, Handle::INVALID);
    assert_eq!(
        (r.object_generation, r.revision, r.data_shm, r.byte_len),
        (0, 0, 0, 0)
    );
}
pub fn allocation_status(reply: &store::NativeSaveReply, expected: StoreStatus) {
    let r = ok(artifact::AllocateSaveIdReply::decode(reply.wire()));
    assert_eq!(r.status, expected as u32);
    assert_eq!(reply.wire().handle, Handle::INVALID);
    assert_eq!(r.operation_id, 0);
}
pub fn commit_status(reply: &store::NativeSaveReply, expected: StoreStatus, op: u64) {
    let r = ok(artifact::CommitReply::decode(reply.wire()));
    assert_eq!(r.status, expected as u32);
    assert_eq!(reply.wire().handle, Handle::INVALID);
    assert_eq!(r.operation_id, op);
    assert_eq!((r.receipt_shm, r.receipt_len), (0, 0));
    if expected != StoreStatus::Conflict {
        assert_eq!(r.revision, 0)
    }
}
pub fn selected_blob(f: &Fixture, id: u64) -> Vec<u8> {
    let digest = object(f, id).blob_hash;
    let name = volatile::hex(&digest);
    let matching = files(&f.path)
        .into_iter()
        .filter(|(p, _)| {
            p.to_string_lossy().contains(&name) && p.extension().is_some_and(|n| n == "blob")
        })
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1);
    matching[0].1.clone()
}
pub fn fence(f: &mut Fixture) -> store::QuiescedNativeSaveOwner {
    f.join_all();
    let token = ok(f.prepared().controller().quiesce_native_save(2000));
    let snap = token.fence_snapshot();
    assert_eq!(
        (snap.schema_version, snap.base.live_ownership_after_join),
        (1, 0)
    );
    assert!(snap.base.service_producers.len() + snap.base.object_workers.len() <= 64);
    for row in snap
        .base
        .service_producers
        .iter()
        .chain(&snap.base.object_workers)
    {
        assert!(row.completed_join && !row.live_ownership_after_join);
        assert_eq!(row.producer_generation, 1);
        assert_eq!(row.native_pid, std::process::id());
        assert!(!row.rust_thread_id.is_empty());
    }
    f.archive_core(token.fence_snapshot());
    token
}
pub fn no_late_io(f: &Fixture, op: u64, before: &[(u64, u64)]) {
    let after = ok(f.prepared().controller().evidence())
        .io_events
        .into_iter()
        .filter(|v| v.operation_id == op)
        .map(|v| (v.sequence, v.writer_id))
        .collect::<Vec<_>>();
    assert_eq!(after, before)
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.root.is_none() {
            return;
        }
        let settled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.release_owned_pauses();
            let end = Instant::now() + Duration::from_millis(2000);
            while self.prepared.is_some()
                && self.h.native_producer_counts().held > 0
                && Instant::now() < end
            {
                self.join_all();
                thread::yield_now();
            }
            self.h.native_producer_counts().held == 0
        }))
        .unwrap_or(false);
        if !settled {
            // Failure cleanup is deliberately not a quiescence/absence witness.
            if let Some(root) = self.root.take() {
                std::mem::forget(root)
            }
            if let Some(core) = self.prepared.take() {
                std::mem::forget(core)
            }
            eprintln!(
                "INCOMPLETE: fixture retains root/Core because actual producer joins are unproved"
            );
        }
    }
}
pub fn recover_commit(
    f: &mut Fixture,
    raw: &RawEditor,
    a: &Attempt,
    owner: &store::NativeSaveRecovery,
    id: u64,
    now: u64,
) -> store::NativeOriginalReply {
    let actor = a.intent.snapshot().view().actor;
    let q = canonical(ok(artifact::SaveStatus {
        request_id: id,
        session_id: actor.session_id,
        session_generation: actor.session_generation,
        operation_id: a.operation,
    }
    .encode(raw.launch.endpoint().handle)));
    let entry = ok(f.h.begin_save_recovery(owner.origin(), &q, now));
    let r = ok(f.host().recover_original(ok(entry.wait_once()), owner));
    for id in entry.producer_ids() {
        join_core(f, id)
    }
    for id in r.producers() {
        join_core(f, id)
    }
    r
}
pub fn same_document(a: &desktop::NativeEditorObservation, b: &desktop::NativeEditorObservation) {
    assert_eq!(
        (
            &a.actor,
            a.document_id,
            &a.bytes,
            a.cursor,
            a.selection,
            a.first_visible_line,
            a.text_generation,
            a.view_version,
            a.confirmed_revision,
            a.confirmed_hash,
            std::mem::discriminant(&a.phase)
        ),
        (
            &b.actor,
            b.document_id,
            &b.bytes,
            b.cursor,
            b.selection,
            b.first_visible_line,
            b.text_generation,
            b.view_version,
            b.confirmed_revision,
            b.confirmed_hash,
            std::mem::discriminant(&b.phase)
        )
    );
}
pub fn native_max_controls(recording: &Arc<recording::CaseRecorder>) {
    for (counter_index, counter) in [
        desktop::NativeSaveCounter::Intent,
        desktop::NativeSaveCounter::Document,
        desktop::NativeSaveCounter::TextGeneration,
        desktop::NativeSaveCounter::ViewVersion,
        desktop::NativeSaveCounter::FrameJoin,
        desktop::NativeSaveCounter::StatusVersion,
        desktop::NativeSaveCounter::SourceBinding,
        desktop::NativeSaveCounter::OriginalRecord,
    ]
    .into_iter()
    .enumerate()
    {
        let mut f = Fixture::recorded(
            recording,
            scenario(
                "ui1_1_clock_capacity_and_counter_limits_fail_closed",
                "native_max",
                counter_index as u32,
                recording::ScenarioScope::DirectNative,
            ),
            1,
        );
        let raw = raw_editor(&mut f, 0, 101, 1);
        raw_replace(&mut f, 0, &raw, NEW, 2);
        let prior = attempt(&mut f, 0, &raw, 2000, 2);
        let settled = settle(&mut f, &prior);
        let prior_bytes = receipt(&settled, prior.operation).1;
        let historical_join = ok(f.h.render_native_editor(&raw.editor, 2));
        let historical = ok(f.h.compose_native_save(&f.drivers[0].s.compositor, 2));
        let before = ok(f.editor_observation(&raw.editor, 2));
        let historical_chrome = ok(f.dc.save_chrome_observation(&f.drivers[0].s.compositor));
        f.record(|r| {
            r.frame(
                &historical,
                Some(&before),
                Some(&historical_join),
                Some(&historical_chrome),
            )
        });
        let baseline = files(&f.path);
        let mut document_baseline = None;
        match counter_index {
            0 => {
                f.drivers[0].send(&f.h, 224, 1, 3);
                ok(f.h.step_native_editor(&raw.editor, 3));
                f.drivers[0].send(&f.h, 22, 1, 3);
                ok(f.dc.seed_native_save_counter(counter));
                f.denied_at(
                    "native_counter_boundary",
                    "step_native_editor",
                    f.h.step_native_editor(&raw.editor, 3),
                    Status::Exhausted,
                );
                same_document(&before, &ok(f.editor_observation(&raw.editor, 3)));
                f.denied_at(
                    "native_counter_secondary",
                    "take_save_intent",
                    f.h.take_save_intent(&raw.editor),
                    Status::NotReady,
                );
                f.drivers[0].send(&f.h, 22, 2, 3);
                ok(f.h.step_native_editor(&raw.editor, 3));
                f.drivers[0].send(&f.h, 224, 2, 3);
                ok(f.h.step_native_editor(&raw.editor, 3));
                raw_stroke(&mut f, 0, &raw, 4, 3);
                assert!(
                    ok(f.editor_observation(&raw.editor, 3))
                        .bytes
                        .ends_with(b"a")
                );
                assert_eq!(f.counts(101), (1, 1, 1));
                assert_eq!(files(&f.path), baseline);
            }
            1 => {
                raw_stroke(&mut f, 0, &raw, 4, 3);
                assert_eq!(
                    ok(f.editor_observation(&raw.editor, 3)).bytes,
                    b"note=new\na"
                );
                let fresh = f.launch(0, 101, 3);
                let preserved = ok(f.editor_observation(&raw.editor, 3));
                assert!(matches!(
                    preserved.phase,
                    desktop::NativeEditorPhase::Unsaved
                ));
                document_baseline = Some(preserved);
                let entry = ok(f.h.begin_save_read(
                    fresh.desktop().bindings().artifact_origin(),
                    &selected_request(&fresh, 2010),
                    3,
                ));
                let registered = ok(f
                    .host()
                    .register_native_save(fresh.endpoint(), ok(entry.wait_once())));
                ok(f.dc.seed_native_save_counter(counter));
                f.denied_at(
                    "native_counter_boundary",
                    "execute_native_save",
                    f.host().execute_native_save(&registered),
                    StoreStatus::Exhausted,
                );
                for id in entry.producer_ids() {
                    join_core(&mut f, id)
                }
                f.join_all();
                same_document(
                    document_baseline.as_ref().unwrap(),
                    &ok(f.editor_observation(&raw.editor, 3)),
                );
                assert_eq!(f.counts(101), (1, 1, 1));
                assert_eq!(files(&f.path), baseline);
                drop((registered, entry, fresh));
            }
            2 | 3 => {
                let usage = if counter_index == 2 { 4 } else { 80 };
                f.drivers[0].send(&f.h, usage, 1, 3);
                ok(f.dc.seed_native_save_counter(counter));
                f.denied_at(
                    "native_counter_boundary",
                    "step_native_editor",
                    f.h.step_native_editor(&raw.editor, 3),
                    Status::Exhausted,
                );
                same_document(&before, &ok(f.editor_observation(&raw.editor, 3)));
                f.drivers[0].send(&f.h, usage, 2, 3);
                ok(f.h.step_native_editor(&raw.editor, 3));
                if counter_index == 2 {
                    raw_stroke(&mut f, 0, &raw, 80, 3);
                    assert_eq!(
                        ok(f.editor_observation(&raw.editor, 3)).cursor,
                        before.cursor - 1
                    );
                }
                assert_eq!(f.counts(101), (1, 1, 1));
                assert_eq!(files(&f.path), baseline);
            }
            4 => {
                ok(f.dc.seed_native_save_counter(counter));
                let trace = ok(f.dc.evidence()).exchanges.len();
                f.denied_at(
                    "native_counter_boundary",
                    "render_native_editor",
                    f.h.render_native_editor(&raw.editor, 3),
                    Status::Exhausted,
                );
                assert_eq!(ok(f.dc.evidence()).exchanges.len(), trace);
                same_document(&before, &ok(f.editor_observation(&raw.editor, 3)));
                assert_eq!(f.counts(101), (1, 1, 1));
                assert_eq!(files(&f.path), baseline);
                raw_stroke(&mut f, 0, &raw, 4, 3);
                assert!(
                    ok(f.editor_observation(&raw.editor, 3))
                        .bytes
                        .ends_with(b"a")
                );
            }
            5 => {
                ok(f.h.render_native_editor(&raw.editor, 3));
                let queued_before = ok(f.editor_observation(&raw.editor, 3));
                ok(f.dc.seed_native_save_counter(counter));
                f.denied_at(
                    "native_counter_boundary",
                    "compose_native_save",
                    f.h.compose_native_save(&f.drivers[0].s.compositor, 3),
                    Status::Exhausted,
                );
                f.denied_at(
                    "native_counter_boundary",
                    "save_chrome_observation",
                    f.dc.save_chrome_observation(&f.drivers[0].s.compositor),
                    Status::Exhausted,
                );
                f.denied_at(
                    "native_counter_boundary",
                    "save_chrome_record",
                    f.dc.save_chrome_record(&f.drivers[0].s.compositor),
                    Status::Exhausted,
                );
                assert_eq!(historical.bgra.len(), 1454080);
                same_document(&queued_before, &ok(f.editor_observation(&raw.editor, 3)));
                assert_eq!(f.counts(101), (1, 1, 1));
                assert_eq!(files(&f.path), baseline);
            }
            6 => {
                let allocation = allocate_start(&mut f, 0, &raw, 2020, 3);
                let reply = ok(f.host().execute_native_save(&allocation.registered));
                let allocated = reply.allocation().unwrap();
                let actual = allocated.observation();
                ok(f.dc.seed_native_save_counter(counter));
                f.denied_at(
                    "native_counter_boundary",
                    "source_for_intent",
                    f.host().source_for_intent(
                        raw.launch.endpoint(),
                        allocated,
                        &allocation.intent,
                    ),
                    StoreStatus::Exhausted,
                );
                assert_eq!(f.counts(101), (1, 1, 1));
                assert!(
                    object(&f, 101)
                        .operations
                        .iter()
                        .any(|r| r.allocation == actual
                            && r.binding.is_none()
                            && r.permit.is_none())
                );
                for id in allocation.entry.producer_ids() {
                    join_core(&mut f, id)
                }
                for id in reply.producers() {
                    join_core(&mut f, id)
                }
                let observed = observe_allocate(&mut f, &allocation, 2021, 3);
                assert!(matches!(
                    observed.observation(),
                    store::NativeOriginalObservation::AllocationRetained {
                        commit_dispatched: false,
                        ..
                    }
                ));
                drop((observed, reply, allocation));
            }
            7 => {
                raw_chord(&mut f, 0, &raw, 22, 3);
                let intent = ok(f.h.take_save_intent(&raw.editor));
                let v = intent.snapshot().view();
                let q = ok(artifact::AllocateSaveId {
                    request_id: 2030,
                    session_id: v.actor.session_id,
                    session_generation: v.actor.session_generation,
                    expected_revision: v.expected_revision,
                }
                .encode(raw.launch.endpoint().handle));
                let entry = ok(f.h.begin_save_allocate(
                    raw.launch.desktop().bindings().artifact_origin(),
                    &intent,
                    &q,
                    3,
                ));
                let call = ok(entry.wait_once());
                ok(f.dc.seed_native_save_counter(counter));
                let failure = match f.host().register_native_save(raw.launch.endpoint(), call) {
                    Err(e) => e,
                    Ok(_) => panic!("MAX original row exposed"),
                };
                assert_eq!(failure.status(), StoreStatus::Exhausted);
                assert!(failure.original().is_none());
                assert_eq!(
                    ok(desktop::encode_envelope_wire(&canonical(
                        *failure.call().request()
                    ))),
                    ok(desktop::encode_envelope_wire(&canonical(q)))
                );
                assert_eq!(failure.producers().len(), 2);
                let joined_ids = failure
                    .producers()
                    .iter()
                    .map(|id| join_core_observed(&mut f, id))
                    .collect::<Vec<_>>();
                f.record(|r| {
                    r.registration_failure("original_record_registration", &failure, &joined_ids)
                });
                for id in entry.producer_ids() {
                    f.denied_at(
                        "native_counter_secondary",
                        "join_native_save_finished",
                        f.host().join_native_save_finished(id),
                        StoreStatus::NotReady,
                    );
                }
                assert_eq!(f.counts(101), (1, 1, 1));
                assert_eq!(object(&f, 101).operations.len(), 1);
                assert_eq!(files(&f.path), baseline);
                drop((failure, entry, intent));
            }
            _ => unreachable!("closed eight-row counter inventory"),
        }
        // These genuine existing controls do not advance the exhausted domain.
        assert_eq!(selected(&mut f, &raw.launch, 2040, 3), (2, NEW.to_vec()));
        let observed = observe_commit(&mut f, &raw, &prior, 2041, 3);
        assert_eq!(original_receipt(&observed).1, prior_bytes);
        if let Some(preserved) = &document_baseline {
            same_document(preserved, &ok(f.editor_observation(&raw.editor, 3)));
            assert_eq!(preserved.bytes, b"note=new\na");
        }
        assert_eq!(object(&f, 101).selected.revision, 2);
        if counter_index == 5 {
            revoke(&f, 0, &before.actor, 4);
            f.denied_at(
                "native_counter_secondary",
                "begin_save_read",
                f.h.begin_save_read(
                    raw.launch.desktop().bindings().artifact_origin(),
                    &selected_request(&raw.launch, 2042),
                    4,
                ),
                Status::Stale,
            );
        }
        drop((observed, settled, prior, raw));
        f.finish();
    }
}
pub fn store_max_controls(recording: &Arc<recording::CaseRecorder>) {
    for (counter_index, counter) in [
        store::CounterKind::OperationHighWater,
        store::CounterKind::WriterHighWater,
        store::CounterKind::ServiceEpoch,
        store::CounterKind::JournalSequence,
        store::CounterKind::Identity,
        store::CounterKind::TimeOrigin,
    ]
    .into_iter()
    .enumerate()
    {
        let mut seeded = Fixture::recorded(
            recording,
            scenario(
                "ui1_1_clock_capacity_and_counter_limits_fail_closed",
                "store_max",
                counter_index as u32,
                recording::ScenarioScope::DirectNative,
            ),
            1,
        );
        ok(seeded.prepared().controller().seed_counter(101, counter));
        let before = object(&seeded, 101).selected;
        let mut healthy = Fixture::recorded(
            recording,
            scenario(
                "ui1_1_clock_capacity_and_counter_limits_fail_closed",
                "store_max_healthy",
                counter_index as u32,
                recording::ScenarioScope::DirectNative,
            ),
            1,
        );
        let positive = healthy.launch(0, 101, 1);
        assert_eq!(
            selected(&mut healthy, &positive, 2100, 1),
            (1, OLD.to_vec())
        );
        drop(positive);
        healthy.finish();
        match counter {
            store::CounterKind::OperationHighWater | store::CounterKind::JournalSequence => {
                let raw = raw_editor(&mut seeded, 0, 101, 1);
                let allocated = allocate_start(&mut seeded, 0, &raw, 2101, 2);
                let reply = ok(seeded.host().execute_native_save(&allocated.registered));
                allocation_status(&reply, StoreStatus::Exhausted);
                for id in allocated.entry.producer_ids() {
                    join_core(&mut seeded, id)
                }
                for id in reply.producers() {
                    join_core(&mut seeded, id)
                }
                assert_eq!(
                    selected(&mut seeded, &raw.launch, 2102, 2),
                    (1, OLD.to_vec())
                );
                assert_eq!(seeded.counts(101), (0, 0, 0));
                assert!(object(&seeded, 101).operations.is_empty());
                drop((reply, allocated, raw));
            }
            store::CounterKind::WriterHighWater => {
                let raw = raw_editor(&mut seeded, 0, 101, 1);
                raw_replace(&mut seeded, 0, &raw, NEW, 2);
                let a = attempt(&mut seeded, 0, &raw, 2110, 2);
                let reply = settle(&mut seeded, &a);
                commit_status(&reply, StoreStatus::Exhausted, 0);
                assert_eq!(seeded.counts(101), (1, 0, 0));
                // The actual definitive pre-permit rejection closed this intent.
                seeded.denied_at(
                    "store_counter_closed_intent",
                    "source_for_intent",
                    seeded.host().source_for_intent(
                        raw.launch.endpoint(),
                        a.alloc_reply.allocation().unwrap(),
                        &a.intent,
                    ),
                    StoreStatus::Stale,
                );
                // Prove sticky exhaustion on a genuine fresh CtrlS admission,
                // without reviving the closed source or issuing another permit.
                let files_before_retry = files(&seeded.path);
                let io_before_retry = ok(seeded.prepared().controller().evidence())
                    .io_events
                    .len();
                let fresh = allocate_start(&mut seeded, 0, &raw, 2113, 2);
                assert_ne!(fresh.intent.view().intent_id, a.intent.view().intent_id);
                let rejected = ok(seeded.host().execute_native_save(&fresh.registered));
                allocation_status(&rejected, StoreStatus::Exhausted);
                assert!(rejected.allocation().is_none());
                for id in fresh.entry.producer_ids() {
                    join_core(&mut seeded, id);
                }
                for id in rejected.producers() {
                    join_core(&mut seeded, id);
                }
                assert_eq!(seeded.counts(101), (1, 0, 0));
                assert_eq!(object(&seeded, 101).operations.len(), 1);
                assert_eq!(
                    ok(seeded.prepared().controller().evidence())
                        .io_events
                        .len(),
                    io_before_retry
                );
                assert_eq!(files(&seeded.path), files_before_retry);
                assert_eq!(
                    selected(&mut seeded, &raw.launch, 2112, 2),
                    (1, OLD.to_vec())
                );
                drop((rejected, fresh, reply, a, raw));
            }
            store::CounterKind::ServiceEpoch => {
                let token = fence(&mut seeded);
                assert_eq!(
                    token.fence_snapshot().base.objects[0].reserved_successor_epoch,
                    None
                );
                let failed = match store::StoreFixture::reopen_native_save(token) {
                    Err(e) => e,
                    Ok(_) => panic!("MAX native service epoch wrapped"),
                };
                assert_eq!(failed.error.status, StoreStatus::Exhausted);
                assert_eq!(
                    failed.owner.fence_snapshot().base.live_ownership_after_join,
                    0
                );
                seeded.finish_reopen_failure(failed);
                continue;
            }
            store::CounterKind::Identity => {
                let (selector, pin, _) = seeded.prepare(0, 101, 1);
                let mut approval = seeded.approval(0, 1);
                let pending = ok(seeded.h.confirm_save_preview(
                    &seeded.drivers[0].s.chrome.peer,
                    &mut approval,
                    1,
                ));
                let failure = match seeded.prepared().activate_save(&pending) {
                    Err(e) => e,
                    Ok(_) => panic!("MAX native Store identity exposed"),
                };
                assert_eq!(failure.status(), StoreStatus::Exhausted);
                assert!(failure.observation().pending_retained && !failure.observation().active);
                seeded.record(|r| {
                    r.expose_failure("store_identity_expose", "activate_save", &failure)
                });
                assert_eq!(
                    ok(seeded.prepared().controller().evidence())
                        .ledger
                        .endpoints,
                    0
                );
                assert_eq!(seeded.producer_counts("producer_checkpoint").held, 0);
                drop((failure, pending, approval, pin, selector));
            }
            store::CounterKind::TimeOrigin => {
                seeded.denied_at(
                    "store_counter_boundary",
                    "pin_selection",
                    seeded
                        .prepared()
                        .pin_selection(&seeded.drivers[0].s.chrome.peer, 101, 1),
                    StoreStatus::Exhausted,
                );
                assert_eq!(seeded.producer_counts("producer_checkpoint").held, 0);
            }
        }
        assert_eq!(object(&seeded, 101).selected, before);
        seeded.finish();
    }
    let mut p = profile(1);
    p.initial_objects[0].revision = u64::MAX;
    let mut f = Fixture::recorded_profile(
        recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "selected_revision_max",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        p,
    );
    let raw = raw_editor(&mut f, 0, 101, 1);
    raw_replace(&mut f, 0, &raw, NEW, 2);
    let a = attempt(&mut f, 0, &raw, 2120, 2);
    let r = settle(&mut f, &a);
    commit_status(&r, StoreStatus::Exhausted, 0);
    assert_eq!(f.counts(101), (1, 0, 0));
    assert_eq!(
        selected(&mut f, &raw.launch, 2122, 2),
        (u64::MAX, OLD.to_vec())
    );
    drop((r, a, raw));
    f.finish();
}
