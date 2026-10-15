//! Independently runnable, in-process P0 fixture preparation. These checks prove
//! real setup/Read/reopen and pause-unwind staging, never editor-child execution.
#![cfg(feature = "host_native_save_v0_dev")]

#[path = "cleanup_owners.rs"]
mod cleanup_owners;
#[allow(dead_code)]
#[path = "fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "../native_task_support/mod.rs"]
mod support;

use artifact_store_schema::editor_save::EditorSaveReceiptV0;
use desktop_service::editor_dev as desktop;
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use store_service::editor_store::{self as store, StoreStatus};
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
        2,
    );
    // Keep the genuine app, its private SaveChains and the parent Fixture outside
    // catch_unwind. This control borrows them; it never reconstructs a Save owner.
    let (mut app, endpoint) = f.app_with_endpoint(0, 101, 1);
    let mut other = f.app(1, 102, 1);
    assert_eq!(ok(other.observe(1)).bytes, B);
    f.replace(0, &mut app, NEW, 2);
    let actor = ok(app.observe(2)).actor;
    let owner = f.prepared.as_ref().unwrap().clone();
    let pause = Arc::new(ok(owner
        .controller()
        .pause_native_save(&endpoint, store::PausePoint::AfterCommitPermit)));
    f.store_pauses.push((owner.clone(), pause.clone()));
    let release = fixture::ReleaseStorePause {
        owner,
        pause: pause.clone(),
    };
    // Only real keyboard delivery and app.step allocate and dispatch this Save.
    f.chord(0, &mut app, 22, 2);
    let end = Instant::now() + Duration::from_millis(1000);
    loop {
        match pause.wait_until_entered(1) {
            Ok(state) => {
                assert!(state.entered && state.permit_issued);
                break;
            }
            Err(error) => assert_eq!(error.status, StoreStatus::Timeout),
        }
        ok(app.step(2));
        assert!(
            Instant::now() < end,
            "genuine app reaches permitted writer pause"
        );
        thread::yield_now();
    }
    assert_eq!(f.counts(101), (1, 1, 0));
    let before = object(&f, 101);
    assert_eq!(before.operations.len(), 1);
    let original = before.operations[0].allocation.operation_id;
    assert!(before.operations[0].receipt.is_none());
    assert_eq!(selected_blob(&f, 101), OLD);

    // A separate real app on the shared Desktop/Store remains editable and
    // renderable while the first app's admitted writer is still paused.
    f.replace(1, &mut other, C, 2);
    f.composed(1, &mut other, 2);
    assert_eq!(f.counts(102), (0, 0, 0));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _release = release;
        ok(app.reconcile(2));
        assert_eq!(ok(app.observe(2)).bytes, NEW);
        assert_eq!(f.counts(101), (1, 1, 0));
        panic!("supported borrower panic while genuine app SaveChain writer is paused");
    }));
    let panic = caught.expect_err("the explicit borrower panic must be observed");
    assert_eq!(
        panic.downcast_ref::<&str>().copied(),
        Some("supported borrower panic while genuine app SaveChain writer is paused")
    );
    assert!(ok(pause.wait_until_settled(1000)).settled);
    // Pause release and a settled writer are not join proofs. Collect actual
    // Core/Store producer joins, then let the retained app resolve its own chain.
    f.join_all();
    ok(app.reconcile(3));
    f.until(&mut app, 3, |o| o.confirmed_revision == 2);
    f.composed(0, &mut app, 3);
    assert!(matches!(
        ok(app.observe(3)).phase,
        desktop::NativeEditorPhase::Saved
    ));
    assert_eq!(f.counts(101), (1, 1, 1));
    assert_eq!(
        object(&f, 101).operations[0].allocation.operation_id,
        original
    );
    app_chain_artifacts(&f, 101, NEW, original, &actor, endpoint.handle);
    assert_eq!(selected_blob(&f, 101), NEW);

    // Recovery must leave the same live app usable for a fresh explicit Save,
    // without replaying the operation that survived the borrower panic.
    f.replace(0, &mut app, OLD, 4);
    f.chord(0, &mut app, 22, 4);
    f.until(&mut app, 4, |o| o.confirmed_revision == 3);
    f.composed(0, &mut app, 4);
    assert!(matches!(
        ok(app.observe(4)).phase,
        desktop::NativeEditorPhase::Saved
    ));
    assert_eq!(f.counts(101), (2, 2, 2));
    let saved = object(&f, 101);
    assert_eq!(saved.operations.len(), 2);
    let next = &saved.operations[1];
    assert_ne!(next.allocation.operation_id, original);
    assert_eq!(next.allocation.expected_revision, 2);
    let binding = next.binding.as_ref().expect("genuine second app binding");
    let captures = ok(f.prepared().controller().native_save_artifacts());
    let captured = captures
        .iter()
        .find(|r| {
            r.operation_id == Some(next.allocation.operation_id)
                && matches!(r.kind, store::NativeSaveArtifactKind::ReceiptCopy)
        })
        .expect("actual full second receipt copy");
    assert_eq!(captured.bytes.len(), 176);
    let receipt = ok(EditorSaveReceiptV0::decode_le(&captured.bytes));
    ok(receipt.validate_binding(binding));
    assert_eq!(
        &receipt,
        next.receipt.as_ref().expect("durable second receipt")
    );
    assert_eq!(receipt.result_revision, 3);
    assert_eq!(captured.actor, actor);
    assert_eq!(selected_blob(&f, 101), OLD);

    // The unrelated app can also finish its own genuine Save on the same Store.
    f.save(1, &mut other, 4);
    assert_eq!(f.counts(102), (1, 1, 1));
    assert_eq!(selected_blob(&f, 102), C);
    assert_eq!(f.counts(101), (2, 2, 2));
    drop((app, other, endpoint, pause));
    f.reopen();
    let launch = f.launch(0, 101, 5);
    let (mut reply, revision, body) = fixture::read(&mut f, &launch, 411, 5);
    assert_eq!((revision, body.as_slice()), (3, OLD));
    let editor = ok(reply.take_editor());
    assert_eq!(ok(f.h.editor_observation(&editor, 5)).bytes, OLD);
    drop((reply, editor, launch));
    f.finish();
}

#[test]
fn process_preparation_constructor_panic_joins_before_root_cleanup() {
    let name = "process_preparation_constructor_panic_joins_before_root_cleanup";
    let recording = ok(recording::CaseRecorder::open(name));
    let staged = std::cell::RefCell::new(None);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Fixture::recorded_with_setup_probe(&recording, fixture::preparation_spec(name), 1, |f| {
            let (mut app, endpoint) = f.app_with_endpoint(0, 101, 1);
            f.replace(0, &mut app, NEW, 2);
            let pause = Arc::new(ok(f
                .prepared()
                .controller()
                .pause_native_save(&endpoint, store::PausePoint::AfterCommitPermit)));
            // The partially constructed Fixture is the only cleanup owner.
            // Its Drop must release the actual pause and join the writer
            // before removing the Store root during this constructor unwind.
            f.store_pauses
                .push((f.prepared.as_ref().unwrap().clone(), pause.clone()));
            f.chord(0, &mut app, 22, 2);
            let end = Instant::now() + Duration::from_millis(1000);
            loop {
                match pause.wait_until_entered(1) {
                    Ok(state) => {
                        assert!(state.entered && state.permit_issued);
                        break;
                    }
                    Err(error) => assert_eq!(error.status, StoreStatus::Timeout),
                }
                ok(app.step(2));
                assert!(Instant::now() < end, "actual constructor writer pause");
                thread::yield_now();
            }
            assert_eq!(f.counts(101), (1, 1, 0));
            let counts = f.h.native_producer_counts();
            assert!(counts.held > 0);
            *staged.borrow_mut() = Some((
                f.root
                    .as_ref()
                    .expect("actual setup root")
                    .path()
                    .to_path_buf(),
                Arc::downgrade(f.prepared.as_ref().expect("actual setup Store owner")),
                f.h.clone(),
                counts,
            ));
            panic!("supported constructor panic with actual permitted writer");
        })
    }));
    let panic = caught.err().expect("the constructor callback must unwind");
    assert_eq!(
        panic.downcast_ref::<&str>().copied(),
        Some("supported constructor panic with actual permitted writer")
    );
    let (path, weak, desktop, before) = staged.into_inner().expect("genuine setup owner captured");
    let after = desktop.native_producer_counts();
    assert_eq!((after.held, after.io, after.joining), (0, 0, 0));
    assert!(after.joined_total >= before.joined_total + u64::from(before.held));
    assert!(
        !path.exists(),
        "root removed only after actual producer joins"
    );
    assert!(
        weak.upgrade().is_none(),
        "settled constructor owner released"
    );

    // The next independently owned setup can still perform an actual authorized
    // Read, rather than treating the terminal constructor panic as sufficient.
    let mut spec = fixture::preparation_spec(name);
    spec.scenario = "after_constructor_panic";
    spec.leg = 1;
    let mut fresh = Fixture::recorded(&recording, spec, 1);
    let (launch, editor) = fixture::initialize(&mut fresh, 1);
    assert_eq!(ok(fresh.h.editor_observation(&editor, 1)).bytes, OLD);
    assert_eq!(fresh.counts(101), (0, 0, 0));
    drop((launch, editor));
    fresh.finish();
}
