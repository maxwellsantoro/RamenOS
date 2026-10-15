//! P0 preparation, deliberately compile RED until the frozen process API exists.
//! These genuine consumer assertions cover only the public happy path, alias
//! fencing, budget construction and prompt consuming owner return. They do not implement all thirty families,
//! the private negative-control fixture, common SaveChains, or a process gate.
#![cfg(feature = "host_editor_process_v0_dev")]

#[path = "process_support/child_owner.rs"]
mod child_owner;
#[allow(dead_code)]
#[path = "process_support/fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "native_task_support/mod.rs"]
mod support;

use desktop_service::dev::Status;
use desktop_service::editor_dev::{self as desktop, ChildBudget, OwnedEditorChild};
use sha2::{Digest, Sha256};
use std::{
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    time::{Duration, Instant},
};
use support::*;

#[test]
fn process_p0_child_budget_has_closed_bounds() {
    for milliseconds in [0, 5001, u32::MAX] {
        assert!(matches!(
            ChildBudget::checked(milliseconds),
            Err(Status::Invalid)
        ));
    }
    for milliseconds in [1, 5000] {
        assert!(ChildBudget::checked(milliseconds).is_ok());
    }
}

#[test]
fn process_p0_genuine_enrollment_fences_legacy_alias_and_repeated_read() {
    let mut f = fixture::fixture(
        "process_p0_genuine_enrollment_fences_legacy_alias_and_repeated_read",
        1,
    );
    let (launch, editor) = fixture::initialize(&mut f, 1);
    let (mut alias_reply, _, _) = fixture::read(&mut f, &launch, 401, 1);
    let alias = ok(alias_reply.take_editor());
    let before = ok(f.h.editor_observation(&alias, 1));
    let enrollment = match f.h.prepare_process_editor(editor) {
        Ok(enrollment) => enrollment,
        Err(failure) => {
            let (_, editor) = failure.into_parts();
            assert_eq!(ok(f.h.editor_observation(&editor, 1)), before);
            drop((editor, alias, alias_reply, launch));
            f.finish();
            panic!("genuine initialized approved editor must enroll");
        }
    };
    assert!(matches!(
        f.h.step_native_editor(&alias, 1),
        Err(Status::Denied)
    ));
    assert!(matches!(
        f.h.render_native_editor(&alias, 1),
        Err(Status::Denied)
    ));
    assert_eq!(ok(f.h.editor_observation(&alias, 1)), before);
    let mut repeated = fixture::read_reply(&mut f, &launch, 402, 1);
    assert!(
        repeated.take_editor().is_err(),
        "process-owned document must not mint a new legacy editor"
    );
    assert_eq!(f.counts(101), (0, 0, 0));
    drop((repeated, alias_reply, alias));
    // The same consumed enrollment remains usable after every denied legacy
    // operation; merely dropping it would not establish that witness.
    let mut owner = spawn_owned(f, enrollment, launch);
    let witness = catch_unwind(AssertUnwindSafe(|| {
        let (f, child) = owner.borrow();
        until_publication(f, child, 1);
        assert_eq!(ok(f.h.observe_process_editor(child, 1)), before);
    }));
    match owner.finish(1) {
        Ok(proof) => assert!(proof.supervisor_joined()),
        Err(retained) => {
            drop(retained);
            panic!("INCOMPLETE: alias-denial witness child reap unproved");
        }
    }
    if let Err(payload) = witness {
        resume_unwind(payload);
    }
}

fn until_publication(f: &Fixture, child: &mut OwnedEditorChild, now: u64) {
    let end = Instant::now() + Duration::from_millis(1000);
    loop {
        let step = ok(f.h.pump_process_editor(child, now));
        if step.publication_accepted() {
            return;
        }
        assert!(
            !step.child_exit_observed(),
            "child exited before valid publication"
        );
        assert!(
            Instant::now() < end,
            "original bounded publication observation"
        );
        std::thread::yield_now();
    }
}

fn event(f: &mut Fixture, child: &mut OwnedEditorChild, usage: u32, phase: u32, now: u64) {
    f.drivers[0].send(&f.h, usage, phase, now);
    let sequence = f.drivers[0].next - 1;
    let end = Instant::now() + Duration::from_millis(1000);
    loop {
        let step = ok(f.h.pump_process_editor(child, now));
        let observed = ok(f.h.observe_process_child(child));
        assert!(observed.acknowledged_sequence() <= sequence);
        if observed.acknowledged_sequence() == sequence {
            return;
        }
        assert!(!step.child_exit_observed());
        assert!(
            Instant::now() < end,
            "genuine Focus delivery acknowledgment"
        );
        std::thread::yield_now();
    }
}

fn spawn_owned(
    f: Fixture,
    enrollment: desktop::ProcessEditorEnrollment,
    launch: store_service::editor_store::PairedNativeSaveLaunch,
) -> child_owner::OwnedProcessFixture {
    let executable = ok(f.h.registered_editor_process_executable(&enrollment));
    match f
        .h
        .spawn_process_editor(enrollment, executable, ok(ChildBudget::checked(5000)), 1)
    {
        Ok(child) => child_owner::OwnedProcessFixture::new(f, child, launch),
        Err(failure) => {
            let (_, recovery) = failure.into_parts();
            match recovery {
                desktop::ProcessLaunchRecovery::Unspawned {
                    enrollment,
                    executable,
                } => {
                    drop((enrollment, executable, launch));
                    f.finish();
                }
                desktop::ProcessLaunchRecovery::Spawned(child) => {
                    let owner = child_owner::OwnedProcessFixture::new(f, child, launch);
                    if let Err(retained) = owner.finish(1) {
                        drop(retained);
                    }
                }
            }
            panic!("actual registered child launch failed; no runtime acceptance");
        }
    }
}

#[test]
fn process_p0_consuming_join_returns_live_owner_without_waiting() {
    let mut f = fixture::fixture(
        "process_p0_consuming_join_returns_live_owner_without_waiting",
        1,
    );
    let (launch, editor) = fixture::initialize(&mut f, 1);
    let enrollment = match f.h.prepare_process_editor(editor) {
        Ok(enrollment) => enrollment,
        Err(failure) => {
            let (_, editor) = failure.into_parts();
            drop((editor, launch));
            f.finish();
            panic!("approved editor preparation failed");
        }
    };
    let mut owner = spawn_owned(f, enrollment, launch);
    let assertion = catch_unwind(AssertUnwindSafe(|| {
        let (f, child) = owner.borrow();
        until_publication(f, child, 1);
        let before = ok(f.h.observe_process_child(child));
        // No retire, expiry, or child fault has occurred. Repeated consuming
        // calls must promptly return the same sole child, not a second waiter.
        for _ in 0..3 {
            owner.assert_live_join_not_ready();
        }
        let (f, child) = owner.borrow();
        assert_eq!(
            ok(f.h.observe_process_child(child)).process_id(),
            before.process_id()
        );
        assert_eq!(
            ok(f.h.observe_process_child(child)).birth_ticks(),
            before.birth_ticks()
        );
        // Returned ownership must remain usable, not merely carry an ID.
        event(f, child, 4, 1, 2);
        event(f, child, 4, 2, 2);
        ok(f.h.request_process_editor_frame(child, 2));
        until_publication(f, child, 2);
        assert_eq!(
            ok(f.h.observe_process_editor(child, 2)).bytes,
            b"note=old\na"
        );
        before
    }));
    let proof = match owner.finish(2) {
        Ok(proof) => proof,
        Err(retained) => {
            drop(retained);
            panic!("INCOMPLETE: consuming join did not return a provably reaped owner");
        }
    };
    let before = match assertion {
        Ok(before) => before,
        Err(payload) => resume_unwind(payload),
    };
    assert!(proof.supervisor_joined());
    assert_eq!(proof.process_id(), before.process_id());
    assert_eq!(proof.birth_ticks(), before.birth_ticks());
    assert_eq!(proof.executable_sha256(), before.executable_sha256());
}

#[test]
fn process_p0_genuine_child_edits_raster_and_single_use_ctrl_s_capture() {
    let mut f = fixture::fixture(
        "process_p0_genuine_child_edits_raster_and_single_use_ctrl_s_capture",
        1,
    );
    let (launch, editor) = fixture::initialize(&mut f, 1);
    let enrollment = match f.h.prepare_process_editor(editor) {
        Ok(enrollment) => enrollment,
        Err(failure) => {
            let (_, editor) = failure.into_parts();
            drop((editor, launch));
            f.finish();
            panic!("approved editor preparation failed");
        }
    };
    let mut owner = spawn_owned(f, enrollment, launch);
    // Only borrower assertions unwind. The actual owners stay outside this
    // boundary, and consuming cleanup always precedes resuming a failed test.
    let result = catch_unwind(AssertUnwindSafe(|| {
        let (f, child) = owner.borrow();
        let birth = ok(f.h.observe_process_child(child));
        assert_ne!(birth.process_id(), std::process::id());
        assert!(birth.birth_ticks() > 0);
        until_publication(f, child, 1);
        assert_eq!(ok(f.h.observe_process_editor(child, 1)).bytes, OLD);
        let first = ok(f.h.compose_native_save(&f.drivers[0].s.compositor, 1));
        fixture::expected_crop(&first, false);
        assert!(matches!(
            f.h.take_process_save_intent(child),
            Err(Status::NotReady)
        ));
        for (usage, phase) in [(224, 1), (4, 1), (4, 2), (224, 2)] {
            event(f, child, usage, phase, 2);
        }
        for &byte in NEW {
            event(f, child, usage(byte), 1, 2);
            event(f, child, usage(byte), 2, 2);
        }
        ok(f.h.request_process_editor_frame(child, 2));
        until_publication(f, child, 2);
        assert_eq!(ok(f.h.observe_process_editor(child, 2)).bytes, NEW);
        let edited = ok(f.h.compose_native_save(&f.drivers[0].s.compositor, 2));
        fixture::expected_crop(&edited, true);
        event(f, child, 224, 1, 2);
        let capture_deadline = Instant::now() + Duration::from_millis(1000);
        event(f, child, 22, 1, 2);
        loop {
            let step = ok(f.h.pump_process_editor(child, 2));
            if step.save_intent_available() {
                break;
            }
            assert!(
                Instant::now() < capture_deadline,
                "original Ctrl+S capture bound"
            );
            std::thread::yield_now();
        }
        let intent = ok(f.h.take_process_save_intent(child));
        let view = intent.snapshot().view();
        assert_eq!(view.byte_len, NEW.len() as u32);
        assert_eq!(view.expected_revision, 1);
        let expected_hash: [u8; 32] = Sha256::digest(NEW).into();
        assert_eq!(view.source_content_hash, expected_hash);
        assert!(Instant::now() < capture_deadline);
        assert_eq!(
            intent.deadline().duration_since(intent.entered()),
            Duration::from_millis(1000)
        );
        assert!(matches!(
            f.h.take_process_save_intent(child),
            Err(Status::NotReady)
        ));
        event(f, child, 22, 2, 2);
        event(f, child, 224, 2, 2);
        assert_eq!(f.counts(101), (0, 0, 0), "capture is not Store mutation");
        drop(intent);
        birth
    }));
    let proof = match owner.finish(2) {
        Ok(proof) => proof,
        Err(retained) => {
            drop(retained);
            panic!("INCOMPLETE: sole process owner and all Store/root owners retained");
        }
    };
    assert!(proof.supervisor_joined());
    let birth = match result {
        Ok(birth) => birth,
        Err(payload) => resume_unwind(payload),
    };
    assert_eq!(proof.process_id(), birth.process_id());
    assert_eq!(proof.birth_ticks(), birth.birth_ticks());
    assert_eq!(proof.executable_sha256(), birth.executable_sha256());
    assert!(matches!(
        proof.wait_outcome(),
        desktop::ProcessWaitOutcome::Exited(_) | desktop::ProcessWaitOutcome::Signaled(_)
    ));
}
