//! Real Store/Read staging shared by the missing-process-API consumer and its
//! independently runnable in-process setup control. This file mints no process
//! or publication witness and never calls the legacy editor renderer.

use crate::support::{self, Fixture, recording, volatile};
use artifact_store_schema::editor_save::EditorTextHeaderV0;
use desktop_service::editor_dev::{self as desktop, EditorMessage};
use kernel_api::generated::desktop_artifact_v1 as artifact;
use store_service::editor_store as store;

pub fn fixture(name: &'static str, sessions: usize) -> Fixture {
    let recording = recording::CaseRecorder::open(name)
        .unwrap_or_else(|error| panic!("actual process-preparation recorder setup: {error:?}"));
    Fixture::recorded(&recording, preparation_spec(name), sessions)
}

pub fn preparation_spec(name: &'static str) -> recording::ScenarioSpec {
    // The shared genuine setup helpers record replay denials and producer
    // checkpoints in addition to the three lifecycle snapshots. Preserve that
    // complete source inventory; this case still claims preparation only.
    let mut spec = support::scenario(
        "ui1_1_clock_capacity_and_counter_limits_fail_closed",
        "process_preparation",
        0,
        recording::ScenarioScope::DirectNative,
    );
    spec.case = name;
    spec.requirements = &["process_preparation_only"];
    spec
}

/// Execute the actual authorized Read and copy its entire genuine lease before
/// taking the Core-issued editor. The copy is diagnostic, never initialization
/// authority. Returning the real reply permits a take-once denial assertion.
pub fn read(
    f: &mut Fixture,
    launch: &store::PairedNativeSaveLaunch,
    request_id: u64,
    now: u64,
) -> (store::NativeSaveReply, u64, Vec<u8>) {
    let reply = read_reply(f, launch, request_id, now);
    let response = support::ok(artifact::ReadSelectedReply::decode(reply.wire()));
    assert_eq!((response.request_id, response.status), (request_id, 0));
    let lease = reply.selected().expect("actual selected lease");
    let descriptor = lease.descriptor();
    assert_eq!(descriptor.handle.pack(), response.data_shm);
    assert_eq!(descriptor.object_generation, response.object_generation);
    assert_eq!(descriptor.byte_len, response.byte_len);
    assert!((64..=4160).contains(&descriptor.byte_len));
    let mut bytes = vec![0xa5; descriptor.byte_len as usize];
    support::ok(lease.copy_into(0, &mut bytes));
    let header = support::ok(EditorTextHeaderV0::decode_le(&bytes[..64]));
    support::ok(header.validate_bytes(&bytes[64..]));
    assert_eq!(header.revision, response.revision);
    assert_eq!(header.selected_object_id, 101);
    (reply, header.revision, bytes[64..].to_vec())
}

pub fn read_reply(
    f: &mut Fixture,
    launch: &store::PairedNativeSaveLaunch,
    request_id: u64,
    now: u64,
) -> store::NativeSaveReply {
    let request = support::selected_request(launch, request_id);
    let entry = support::ok(f.h.begin_save_read(
        launch.desktop().bindings().artifact_origin(),
        &request,
        now,
    ));
    let registered = support::ok(
        f.host()
            .register_native_save(launch.endpoint(), support::ok(entry.wait_once())),
    );
    let reply = support::ok(f.host().execute_native_save(&registered));
    for id in entry.producer_ids().into_iter().chain(reply.producers()) {
        support::join_core(f, id);
    }
    drop((registered, entry));
    reply
}

pub fn initialize(
    f: &mut Fixture,
    now: u64,
) -> (store::PairedNativeSaveLaunch, desktop::NativeEditor) {
    let launch = f.launch(0, 101, now);
    let (mut reply, revision, body) = read(f, &launch, 400, now);
    assert_eq!((revision, body.as_slice()), (1, support::OLD));
    let editor = support::ok(reply.take_editor());
    assert!(matches!(
        reply.take_editor(),
        Err(store::StoreStatus::NotReady)
    ));
    assert_eq!(
        support::ok(f.h.editor_observation(&editor, now)).bytes,
        body
    );
    drop(reply);
    (launch, editor)
}

/// Borrowed-only panic cleanup: the actual parent Store owner and pause remain
/// retained by Fixture. Releasing a pause is not a writer-join proof.
pub struct ReleaseStorePause {
    pub owner: std::sync::Arc<store::PreparedNativeSaveStore>,
    pub pause: std::sync::Arc<store::PauseToken>,
}

impl Drop for ReleaseStorePause {
    fn drop(&mut self) {
        if self.owner.controller().release(&self.pause).is_err() {
            eprintln!("INCOMPLETE: actual Store pause release failed; fixture retains owners");
        }
    }
}

pub fn stroke(f: &mut Fixture, usage: u32, now: u64) {
    // Deliver genuine service events only. The process consumer must pump each
    // press/release separately; this helper does not acknowledge or edit them.
    f.drivers[0].stroke(&f.h, usage, now);
}

pub fn expected_crop(frame: &desktop::ComposedFrame, new: bool) {
    volatile::assert_golden(frame, new);
}
