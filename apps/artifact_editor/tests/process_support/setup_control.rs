//! Independently runnable, in-process P0 fixture preparation. These checks prove
//! real setup/Read/reopen and pause-unwind staging, never editor-child execution.
#![cfg(feature = "host_native_save_v0_dev")]

#[allow(dead_code)]
#[path = "fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "../native_task_support/mod.rs"]
mod support;

use store_service::editor_store as store;
use support::*;

#[test]
fn process_preparation_genuine_read_and_owned_reopen() {
    let mut f = fixture::fixture("process_preparation_genuine_read_and_owned_reopen", 1);
    let (launch, editor) = fixture::initialize(&mut f, 1);
    assert_eq!(ok(f.h.editor_observation(&editor, 1)).bytes, OLD);
    assert_eq!(f.counts(101), (0, 0, 0));
    drop((launch, editor));
    f.reopen();
    let launch = f.launch(0, 101, 2);
    let (mut reply, revision, bytes) = fixture::read(&mut f, &launch, 401, 2);
    assert_eq!((revision, bytes.as_slice()), (1, OLD));
    let editor = ok(reply.take_editor());
    assert_eq!(ok(f.h.editor_observation(&editor, 2)).bytes, OLD);
    assert_eq!(f.counts(101), (0, 0, 0));
    drop((reply, editor, launch));
    f.finish();
}

#[test]
fn process_preparation_save_pause_unwind_retains_original_writer() {
    let mut f = fixture::fixture(
        "process_preparation_save_pause_unwind_retains_original_writer",
        1,
    );
    // This is the accepted in-process control, kept separate from process-mode
    // fixture initialization, which must never call legacy step/render.
    let raw = raw_editor(&mut f, 0, 101, 1);
    raw_replace(&mut f, 0, &raw, NEW, 2);
    let attempt = attempt(&mut f, 0, &raw, 410, 2);
    let pause = store_pause(&mut f, &raw, store::PausePoint::AfterCommitPermit);
    let execution = ok(f.host().start_native_save(&attempt.commit));
    assert!(ok(pause.wait_until_entered(1000)).permit_issued);
    assert_eq!(f.counts(101), (1, 1, 0));
    let owner = f.prepared.as_ref().unwrap().clone();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _release = fixture::ReleaseStorePause {
            owner,
            pause: pause.clone(),
        };
        panic!("supported borrower panic while genuine permitted writer is paused");
    }));
    assert!(caught.is_err());
    assert!(ok(pause.wait_until_settled(1000)).settled);
    let reply = pending_reply(&f, &execution);
    let record = receipt(&reply, attempt.operation).0;
    assert_eq!(record.operation_id, attempt.operation);
    join_attempt(&mut f, &attempt, &reply);
    assert_eq!(f.counts(101), (1, 1, 1));
    assert_eq!(selected_blob(&f, 101), NEW);
    drop((reply, execution, attempt, raw, pause));
    f.reopen();
    let launch = f.launch(0, 101, 3);
    let (mut reply, revision, body) = fixture::read(&mut f, &launch, 411, 3);
    assert_eq!((revision, body.as_slice()), (2, NEW));
    let editor = ok(reply.take_editor());
    assert_eq!(ok(f.h.editor_observation(&editor, 3)).bytes, NEW);
    drop((reply, editor, launch));
    f.finish();
}
