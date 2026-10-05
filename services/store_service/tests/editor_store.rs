//! UI1.1b: real host CAS transactions; reused UIa legs remain volatile witnesses.
#![cfg(feature = "editor_store_v0_dev")]

#[path = "support/editor_store.rs"]
mod support;
#[allow(dead_code)]
#[path = "../../desktop/tests/support/editor_host_assertions.rs"]
mod ui_a;

use artifact_store_schema::editor_save::*;
use kernel_api::cap::Handle;
use kernel_api::generated::desktop_artifact_v1 as artifact;
use serde_json::json;
use std::time::Instant;
use store_service::editor_store::*;
use support::*;

fn read_request(a: &EditorActorV0, request_id: u64) -> artifact::ReadSelected {
    artifact::ReadSelected {
        request_id,
        session_id: a.session_id,
        session_generation: a.session_generation,
        instance_id: a.instance_id,
        instance_generation: a.instance_generation,
    }
}
fn alloc_request(a: &EditorActorV0, base: u64, request_id: u64) -> artifact::AllocateSaveId {
    artifact::AllocateSaveId {
        request_id,
        session_id: a.session_id,
        session_generation: a.session_generation,
        expected_revision: base,
    }
}
fn committed(
    f: &Fixture,
    e: &RecordedEndpoint,
    a: &EditorActorV0,
    op: &Attempt,
) -> EditorSaveReceiptV0 {
    let reply = exchange(&f.host, e, op.request);
    let receipt = receipt(&f.host, e, &reply);
    let record = f
        .object(receipt.selected_object_id)
        .operations
        .into_iter()
        .find(|record| record.allocation.operation_id == op.operation)
        .unwrap();
    assert_eq!(record.allocation.actor, *a);
    ok(receipt.validate_binding(record.binding.as_ref().unwrap()));
    assert_eq!(receipt.outcome, EditorReceiptOutcomeV0::Committed);
    assert_eq!(receipt.result_revision, op.request.expected_revision + 1);
    assert_eq!(record.receipt.as_ref(), Some(&receipt));
    assert_eq!(
        record.permit.as_ref().unwrap().binding,
        *record.binding.as_ref().unwrap()
    );
    receipt
}
fn no_replay(f: &Fixture, object: u64, operation: u64, expected_dispatches: usize) {
    let evidence = ok(f.control.evidence());
    assert_ledger(&evidence);
    assert_eq!(
        evidence
            .exchanges
            .iter()
            .filter(|event| event.selected_object_id == object
                && event.request_wire[52..60] == operation.to_le_bytes()
                && event.request_wire[4..8] == 5u32.to_le_bytes())
            .count(),
        expected_dispatches,
        "actual Commit wire inventory includes intentional duplicate denial probes"
    );
    let record = evidence
        .objects
        .iter()
        .find(|o| o.selected.selected_object_id == object)
        .unwrap()
        .operations
        .iter()
        .find(|r| r.allocation.operation_id == operation)
        .unwrap();
    if let Some(receipt) = &record.receipt {
        ok(receipt.validate_binding(record.binding.as_ref().unwrap()));
    }
}

fn volatile_retirement_leg(case: &'static str, leg: u32, frame: bool) {
    use desktop_service::dev::Status;
    use desktop_service::editor_dev::PausePoint as UiPause;
    let mut f = ui_a::Fixture::new(OLD);
    let mut live = f.launch(1);
    let before = live.client.snapshot();
    let previous = ui_a::render(&f.h, &f.d, &mut live, 1);
    ui_a::assert_golden(&previous, false);
    ui_a::assert_volatile_label(&previous);
    let bootstrap = live.boot;
    let token = ok(f.c.pause_next(
        f.d.s.session_id,
        if frame {
            UiPause::BeforePresent
        } else {
            UiPause::BeforeKeyDelivery
        },
    ));
    if !frame {
        f.d.send(&f.h, 4, 1, 2);
    }
    let mut client = live.client;
    let join = std::thread::spawn(move || {
        let result = if frame {
            client.render(2).map(|_| ())
        } else {
            client.step(2).map(|_| ())
        };
        (client, result)
    });
    let entered = ok(token.wait_until_entered(2000));
    assert!(entered.entered && !entered.permit_issued);
    ui_a::revoke(&f.h, &f.d.s, &bootstrap, 3);
    ok(f.c.release(&token));
    let settled = ok(token.wait_until_settled(2000));
    assert!(settled.settled);
    let (client, result) = join.join().unwrap();
    assert!(matches!(result, Err(Status::Stale)));
    ui_a::assert_draft_equal(&before, &client.snapshot());
    match f.h.compose_next(&f.d.s.compositor, 3) {
        Ok(recovery) => {
            assert_eq!((recovery.sequence, recovery.focus_epoch), (0, 0));
            assert_eq!(ui_a::crop(&recovery), vec![255; ui_a::FRAME_BYTES]);
        }
        Err(Status::NotReady) => {}
        Err(_) => panic!("unexpected late UIa frame"),
    }
    assert!(matches!(
        f.h.compose_next(&f.d.s.compositor, 3),
        Err(Status::NotReady)
    ));
    let mut record = UiRecord::new(case);
    record.frame(previous);
    record.barrier_note("volatile_before_effect_entered", &entered);
    record.barrier_note("volatile_before_effect_settled", &settled);
    record.note(
        "backend_boundary",
        "{\"backend\":\"volatile\",\"integrated_store_save\":false}",
    );
    record.finish(&f.c, leg);
}

#[test]
fn ui1_1_revoke_paused_key_frame_and_save() {
    let name = "ui1_1_revoke_paused_key_frame_and_save";
    volatile_retirement_leg(name, 0, false);
    volatile_retirement_leg(name, 1, true);
    for (leg, expiry) in [false, true].into_iter().enumerate() {
        let mut f = Fixture::new();
        let a = actor(1, 1);
        let e = f.endpoint(&a, 31, 7, if expiry { 10 } else { 600000 });
        assert_eq!(read(&f.host, &e, &a, 1), (1, OLD.to_vec()));
        let old = f.disk_selected(31);
        let old_attempt = attempt(&f, &e, &a, 1, NEW, 2);
        let alias = ok(f.host.write_lease(&e.peer, &old_attempt.source));
        let index = f.arm(&e, PausePoint::BeforeCommitPermit);
        let pending = Pending::start(&f.host, &e, old_attempt.request);
        let entered = f.entered(index, old_attempt.operation, false);
        assert!(alias.copy_from(64, b"X").is_err());
        assert!(f.host.read_lease(&e.peer, &old_attempt.source).is_err());
        if expiry {
            assert_eq!(ok(f.control.advance_time(12)), 12);
            assert_eq!(ok(f.control.advance_time(3)), 12);
        } else {
            ok(f.control.retire(&e, RetirementReason::Revoked));
        }
        redacted(
            &exchange(&f.host, &e, old_attempt.request),
            32,
            StoreStatus::Stale,
        );
        assert!(matches!(alias.copy_from(64, b"X"), Err(StoreStatus::Stale)));
        f.release(index);
        redacted(&pending.finish(), 32, StoreStatus::Stale);
        assert_eq!(f.counts(31), (1, 0, 0));
        assert_eq!(f.disk_selected(31), old);
        let next_actor = actor(1, 2);
        let fresh = f.endpoint(&next_actor, 31, 7, 600000);
        let new = attempt(&f, &fresh, &next_actor, 1, NEW, 10);
        let r = committed(&f, &fresh, &next_actor, &new);
        assert_ne!(r.operation_id, old_attempt.operation);
        assert_eq!(f.counts(31), (2, 1, 1));
        assert_eq!(read(&f.host, &fresh, &next_actor, 12), (2, NEW.to_vec()));
        no_replay(&f, 31, old_attempt.operation, 2);
        f.emit(name,leg as u32,json!({"expiry":expiry,"entered":barrier_json(&entered),"fresh_operation":r.operation_id,"old_blob_hash":hash(&old)}));
    }
    // Issued authority is an immutable exception for the original transition only.
    let mut f = Fixture::new();
    let a = actor(1, 1);
    let e = f.endpoint(&a, 31, 7, 600000);
    let op = attempt(&f, &e, &a, 1, NEW, 1);
    let index = f.arm(&e, PausePoint::AfterCommitPermit);
    let pending = Pending::start(&f.host, &e, op.request);
    f.entered(index, op.operation, true);
    ok(f.control.retire(&e, RetirementReason::Revoked));
    unknown(&pending.finish(), op.operation);
    redacted(&exchange(&f.host, &e, op.request), 32, StoreStatus::Stale);
    assert_eq!(f.counts(31), (1, 1, 0));
    f.release(index);
    let recovery = f.recovery(a.clone(), 31, op.operation);
    let r = receipt(
        &f.host,
        &recovery,
        &status_reply(&f.host, &recovery, &a, op.operation, 8),
    );
    assert_eq!(r.outcome, EditorReceiptOutcomeV0::Committed);
    assert_eq!(f.counts(31), (1, 1, 1));
    f.emit(
        name,
        2,
        json!({"after_permit_retired":true,"original_receipt":r}),
    );
}

#[test]
fn ui1_1_stalled_session_leaves_other_session_usable() {
    let name = "ui1_1_stalled_session_leaves_other_session_usable";
    let mut f = Fixture::new();
    let a = actor(1, 1);
    let b = actor(2, 2);
    let ea = f.endpoint(&a, 31, 7, 600000);
    let eb = f.endpoint(&b, 32, 7, 600000);
    let aa = attempt(&f, &ea, &a, 1, NEW, 1);
    let ab = attempt(&f, &eb, &b, 1, b"other=new\n", 10);
    let ui = ui_a::Fixture::new(OLD);
    let mut driver = ui_a::Driver::register(&ui.h, &ui.c, 2, b"other\n", 1);
    let mut live = driver.launch(&ui.h, &ui.c, 1);
    // Store actor and UI actor are separately authenticated; this is no forwarding bridge.
    let index = f.arm(&ea, PausePoint::BeforeCandidateWrite);
    let start = Instant::now();
    let pending = Pending::start(&f.host, &ea, aa.request);
    let entered = f.entered(index, aa.operation, true);
    unknown(&pending.finish(), aa.operation);
    let timeout_ms = elapsed_ms(start);
    assert!((900..2500).contains(&timeout_ms));
    redacted(
        &exchange(&f.host, &ea, alloc_request(&a, 1, 20)),
        16,
        StoreStatus::NotReady,
    );
    redacted(
        &exchange(&f.host, &ea, aa.request),
        32,
        StoreStatus::NotReady,
    );
    let progress = Instant::now();
    assert_eq!(read(&f.host, &eb, &b, 21), (1, b"other=old\n".to_vec()));
    let rb = committed(&f, &eb, &b, &ab);
    assert_eq!(read(&f.host, &eb, &b, 22), (2, b"other=new\n".to_vec()));
    driver.key(&ui.h, &mut live, 4, 2);
    assert!(live.client.snapshot().bytes.ends_with(b"a"));
    let frame = ui_a::render(&ui.h, &driver, &mut live, 2);
    let b_ms = elapsed_ms(progress);
    assert!(
        b_ms < 1000,
        "finite B read/save/read plus UI frame deadline"
    );
    ui_a::assert_volatile_label(&frame);
    assert_eq!(f.counts(31), (1, 1, 0));
    assert_eq!(f.counts(32), (1, 1, 1));
    f.release(index);
    let ra = receipt(
        &f.host,
        &ea,
        &status_reply(&f.host, &ea, &a, aa.operation, 23),
    );
    assert_eq!(ra.result_revision, 2);
    assert_eq!(f.counts(31), (1, 1, 1));
    no_replay(&f, 31, aa.operation, 2);
    let mut record = UiRecord::new(name);
    record.frame(frame);
    record.note(
        "backend_boundary",
        "{\"backend\":\"volatile\",\"integrated_store_save\":false}",
    );
    record.finish(&ui.c, 0);
    f.emit(name,0,json!({"entered":barrier_json(&entered),"original_deadline_ms":timeout_ms,"other_progress_ms":b_ms,"a_receipt":ra,"b_receipt":rb}));
}

#[test]
fn ui1_1_clock_capacity_and_counter_limits_fail_closed() {
    let name = "ui1_1_clock_capacity_and_counter_limits_fail_closed";
    let mut f = Fixture::new();
    let a = actor(1, 1);
    let e = f.endpoint(&a, 31, 7, 600000);
    let first = attempt(&f, &e, &a, 1, NEW, 1);
    let original = committed(&f, &e, &a, &first);
    let canceled_actor = actor(1, 2);
    let canceled_endpoint = f.endpoint(&canceled_actor, 31, 7, 600000);
    let canceled_id = allocate(&f.host, &canceled_endpoint, &canceled_actor, 2, 30);
    ok(f.control
        .retire(&canceled_endpoint, RetirementReason::Revoked));
    ok(f.control.drop_next_reply(&e));
    redacted(
        &exchange(&f.host, &e, alloc_request(&a, 2, 31)),
        16,
        StoreStatus::Disconnected,
    );
    let lost_id = f
        .object(31)
        .operations
        .last()
        .unwrap()
        .allocation
        .operation_id;
    let uncertain = attempt(&f, &e, &a, 2, b"note=second\n", 32);
    let barrier = f.arm(&e, PausePoint::AfterCommitPermit);
    let pending = Pending::start(&f.host, &e, uncertain.request);
    f.entered(barrier, uncertain.operation, true);
    unknown(&pending.finish(), uncertain.operation);
    f.release(barrier);
    let second = receipt(
        &f.host,
        &e,
        &status_reply(&f.host, &e, &a, uncertain.operation, 35),
    );
    assert_eq!(second.result_revision, 3);
    for index in 4..16 {
        allocate(&f.host, &e, &a, 3, 100 + index);
    }
    let populated = f.object(31);
    let ids = populated
        .operations
        .iter()
        .map(|r| r.allocation.operation_id)
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 16);
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        populated
            .operations
            .iter()
            .any(|r| r.allocation.operation_id == canceled_id
                && r.closure == EditorClosureV0::Revoked)
    );
    assert!(
        populated
            .operations
            .iter()
            .any(|r| r.allocation.operation_id == lost_id)
    );
    let before = f.object(31);
    let files = f.files();
    redacted(
        &exchange(&f.host, &e, alloc_request(&a, 3, 200)),
        16,
        StoreStatus::Exhausted,
    );
    assert_eq!(f.files(), files);
    assert_eq!(f.object(31).operations.len(), 16);
    assert_eq!(f.object(31).selected, before.selected);
    let got = receipt(
        &f.host,
        &e,
        &status_reply(&f.host, &e, &a, first.operation, 201),
    );
    assert_eq!(got, original);
    assert_eq!(read(&f.host, &e, &a, 202), (3, b"note=second\n".to_vec()));
    // Trusted seeding cannot rewrite populated history.
    let seed = f.control.seed_counter(31, CounterKind::OperationHighWater);
    assert!(matches!(
        seed,
        Err(FixtureError {
            status: StoreStatus::NotReady,
            ..
        })
    ));
    assert_eq!(f.files(), files);
    ok(f.control.retire(&e, RetirementReason::Revoked));
    let token = f.fence();
    let snapshot = f.reopen(token);
    assert_eq!(f.object(31).operations.len(), 16);
    assert_eq!(
        f.object(31)
            .operations
            .iter()
            .map(|r| r.allocation.operation_id)
            .collect::<Vec<_>>(),
        ids
    );
    let fresh_actor = actor(1, 3);
    let fresh = f.endpoint(&fresh_actor, 31, 7, 600000);
    redacted(
        &exchange(&f.host, &fresh, alloc_request(&fresh_actor, 3, 203)),
        16,
        StoreStatus::Exhausted,
    );
    assert_eq!(
        read(&f.host, &fresh, &fresh_actor, 204),
        (3, b"note=second\n".to_vec())
    );
    let recovery = f.recovery(a.clone(), 31, first.operation);
    assert_ne!(first.operation, uncertain.operation);
    assert_eq!(
        f.object(31)
            .operations
            .iter()
            .find(|record| record.allocation.operation_id == uncertain.operation)
            .unwrap()
            .allocation
            .actor,
        a
    );
    let recovery_files = f.files();
    let recovery_counts = f.counts(31);
    redacted(
        &status_reply(&f.host, &recovery, &a, uncertain.operation, 206),
        32,
        StoreStatus::Denied,
    );
    assert_eq!(f.files(), recovery_files);
    assert_eq!(f.counts(31), recovery_counts);
    assert_eq!(
        receipt(
            &f.host,
            &recovery,
            &status_reply(&f.host, &recovery, &a, first.operation, 205)
        ),
        original
    );
    f.emit(
        name,
        0,
        json!({"retained_ids":ids,"fence":snapshot,"capacity":16,"original_receipt":original,"unknown_receipt":second,"canceled_id":canceled_id,"lost_allocated_id":lost_id,"same_actor_wrong_operation_denied":uncertain.operation}),
    );

    // Lost allocation reply consumes and retains its real slot; unused durable range is skipped on reopen.
    let mut lost = Fixture::new();
    let e = lost.endpoint(&a, 31, 7, 600000);
    ok(lost.control.drop_next_reply(&e));
    redacted(
        &exchange(&lost.host, &e, alloc_request(&a, 1, 1)),
        16,
        StoreStatus::Disconnected,
    );
    let journal = lost.journal(31).1;
    assert_eq!(journal.operations.len(), 1);
    let abandoned = journal.operations.as_slice()[0].allocation.operation_id;
    let token = lost.fence();
    lost.reopen(token);
    let fresh = lost.endpoint(&fresh_actor, 31, 7, 600000);
    let next = allocate(&lost.host, &fresh, &fresh_actor, 1, 2);
    assert!(next > journal.operation_high_water && next > abandoned);
    assert_eq!(lost.object(31).operations.len(), 2);
    let before_read_files = lost.files();
    let before_read_counts = lost.counts(31);
    assert_eq!(read(&lost.host, &fresh, &fresh_actor, 3), (1, OLD.to_vec()));
    assert_eq!(lost.files(), before_read_files);
    assert_eq!(lost.counts(31), before_read_counts);
    lost.emit(name,1,json!({"lost_allocated_id":abandoned,"old_high_water":journal.operation_high_water,"fresh_id":next}));

    let clock = Fixture::new();
    let ea = clock.endpoint(&a, 31, 7, 10);
    let b = actor(2, 2);
    let eb = clock.endpoint(&b, 32, 1, 600000);
    assert_eq!(ok(clock.control.advance_time(12)), 12);
    assert_eq!(ok(clock.control.advance_time(2)), 12);
    redacted(
        &exchange(&clock.host, &ea, read_request(&a, 1)),
        36,
        StoreStatus::Stale,
    );
    assert_eq!(read(&clock.host, &eb, &b, 2), (1, b"other=old\n".to_vec()));
    clock.emit(
        name,
        2,
        json!({"clamped_ms":12,"same_registry_b_read":true}),
    );

    for (leg, counter) in [
        CounterKind::OperationHighWater,
        CounterKind::WriterHighWater,
        CounterKind::ServiceEpoch,
        CounterKind::JournalSequence,
        CounterKind::Identity,
        CounterKind::TimeOrigin,
    ]
    .into_iter()
    .enumerate()
    {
        let mut seeded = Fixture::new();
        ok(seeded.control.seed_counter(31, counter));
        // No positive read is fabricated in a genesis domain that cannot issue grants.
        let healthy = Fixture::new();
        let positive = healthy.endpoint(&a, 31, 1, 600000);
        let actual_positive = read(&healthy.host, &positive, &a, 1);
        assert_eq!(actual_positive, (1, OLD.to_vec()));
        healthy.emit(name, leg as u32 + 10, json!({"positive_for_counter_index":leg,"healthy_actual_read_revision":actual_positive.0,"healthy_actual_read_content_hash":hash(&actual_positive.1)}));
        match leg {
            0 | 3 => {
                let endpoint = seeded.endpoint(&a, 31, 7, 600000);
                redacted(
                    &exchange(&seeded.host, &endpoint, alloc_request(&a, 1, 1)),
                    16,
                    StoreStatus::Exhausted,
                );
                redacted(
                    &exchange(&seeded.host, &endpoint, alloc_request(&a, 1, 2)),
                    16,
                    StoreStatus::Exhausted,
                );
                assert_eq!(read(&seeded.host, &endpoint, &a, 3), (1, OLD.to_vec()));
            }
            1 => {
                let endpoint = seeded.endpoint(&a, 31, 7, 600000);
                let op = attempt(&seeded, &endpoint, &a, 1, NEW, 1);
                redacted(
                    &exchange(&seeded.host, &endpoint, op.request),
                    32,
                    StoreStatus::Exhausted,
                );
                assert!(matches!(
                    seeded.host.draft_source(&endpoint.peer, 1, NEW),
                    Err(StoreStatus::Exhausted)
                ));
                assert_eq!(seeded.counts(31), (1, 0, 0));
                assert_eq!(read(&seeded.host, &endpoint, &a, 3), (1, OLD.to_vec()));
            }
            2 => {
                let token = seeded.fence();
                let before = token.fence_snapshot();
                assert_eq!(
                    before
                        .objects
                        .iter()
                        .find(|o| o.selected_object_id == 31)
                        .unwrap()
                        .reserved_successor_epoch,
                    None
                );
                let failure = match seeded.try_reopen(token) {
                    Err(failure) => failure,
                    Ok(_) => panic!("epoch MAX wrapped"),
                };
                assert!(matches!(failure.error.status, StoreStatus::Exhausted));
                assert_fence(&failure.owner.fence_snapshot());
            }
            4 | 5 => {
                assert!(matches!(
                    seeded.control.issue_editor(GrantSpec {
                        actor: a.clone(),
                        selected_object_id: 31,
                        selected_generation: 1,
                        rights: 7,
                        ttl_ms: 600000
                    }),
                    Err(FixtureError {
                        status: StoreStatus::Exhausted,
                        ..
                    })
                ));
            }
            _ => unreachable!(),
        }
        assert_eq!(seeded.object(31).selected.content_hash, hash(OLD));
        seeded.emit(name,leg as u32+3,json!({"genesis_max_counter_index":leg,"positive_control_scope":"independent_healthy_registry","no_history_rewrite":true}));
    }
    let max = Fixture::with_profile(profile(u64::MAX));
    let e = max.endpoint(&a, 31, 7, 600000);
    let op = attempt(&max, &e, &a, u64::MAX, NEW, 1);
    redacted(
        &exchange(&max.host, &e, op.request),
        32,
        StoreStatus::Exhausted,
    );
    assert_eq!(max.object(31).selected.revision, u64::MAX);
    assert_eq!(max.counts(31), (1, 0, 0));
    max.emit(name, 9, json!({"genesis_revision_max":true,"no_wrap":true}));

    // UIa queue saturation reserves Reset; checked sequence exhaustion is separate.
    use desktop_service::dev::Status as UiStatus;
    let mut ui = ui_a::Fixture::new(OLD);
    let mut live = ui.launch(1);
    for _ in 0..32 {
        ui.d.send(&ui.h, 4, 1, 2);
        ui.d.send(&ui.h, 4, 2, 2);
    }
    let prior = ok(ui.c.evidence());
    assert_eq!(prior.ledger.queued_keys, 64);
    let before_reset = live.client.snapshot();
    let overflow_sequence = ui.d.next;
    ui.d.send(&ui.h, 5, 1, 2);
    assert_eq!(ui.d.next, overflow_sequence + 1);
    let reset_queued = ok(ui.c.evidence());
    assert_eq!(reset_queued.ledger.queued_keys, 1);
    ui_a::deny(
        &ui.h,
        &ui.d.s.input,
        ui.d.request(4, 1, 0),
        8,
        2,
        UiStatus::NotReady,
    );
    assert_eq!(ui.d.next, overflow_sequence + 1);
    assert_eq!(ok(ui.c.evidence()).ledger.queued_keys, 1);
    let reset = ok(live.client.step(2));
    assert_eq!(reset.consumed_sequence, Some(overflow_sequence));
    assert!(!reset.draft_changed);
    ui_a::assert_draft_equal(&before_reset, &live.client.snapshot());
    assert_eq!(ok(ui.c.evidence()).ledger.queued_keys, 0);
    // The same producer can deliver its checked successor after Reset was consumed.
    let successor = ui.d.next;
    ui.d.send(&ui.h, 4, 1, 2);
    let fresh = ok(live.client.step(2));
    assert_eq!(fresh.consumed_sequence, Some(successor));
    assert!(fresh.draft_changed);
    ui.d.send(&ui.h, 4, 2, 2);
    assert_eq!(
        ok(live.client.step(2)).consumed_sequence,
        Some(successor + 1)
    );
    ui_a::assert_caps(&ok(ui.c.evidence()));
    let mut record = UiRecord::new(name);
    record.note(
        "backend_boundary",
        &format!("{{\"backend\":\"volatile\",\"queue_limit\":64,\"overflow_sequence\":{overflow_sequence},\"queued_reset\":1,\"production_before_reset\":{},\"reset_preserves_draft\":true,\"fresh_successor_sequence\":{successor},\"integrated_store_save\":false}}", UiStatus::NotReady as u32),
    );
    record.finish(&ui.c, 0);
    let mut frames = ui_a::Fixture::new(OLD);
    let mut live = frames.launch(1);
    let mut record = UiRecord::new(name);
    // Initial launch already consumed one frame; fifteen more reach the runtime cap.
    for _ in 1..16 {
        let frame = ui_a::render(&frames.h, &frames.d, &mut live, 2);
        ui_a::assert_golden(&frame, false);
        ui_a::assert_volatile_label(&frame);
        record.frame(frame);
    }
    ok(live.client.render(2));
    record.note("backend_boundary","{\"backend\":\"volatile\",\"actual_composed_limit\":16,\"retained_frames\":15,\"initial_consumed_frame\":1,\"integrated_store_save\":false}");
    // Retain the last complete snapshot before the sticky frame diagnostic failure.
    record.finish(&frames.c, 1);
    let mut terminal_statuses = Vec::new();
    for _ in 0..2 {
        match frames.h.compose_next(&frames.d.s.compositor, 2) {
            Err(UiStatus::Exhausted) => terminal_statuses.push(UiStatus::Exhausted as u32),
            _ => panic!("frame budget must fail closed without another composed frame"),
        }
    }
    let evidence_status = match frames.c.evidence() {
        Err(UiStatus::Exhausted) => UiStatus::Exhausted as u32,
        _ => panic!("sticky frame exhaustion must deny a later diagnostic snapshot"),
    };
    if let Some(root) = std::env::var_os("RAMEN_DESKTOP_EDITOR_GATE_EVIDENCE") {
        let directory = std::path::PathBuf::from(root).join(name);
        let prior = bounded_read(&directory.join("registry-1.json"), 1024 * 1024);
        let terminal = json!({
            "schema_version": 1,
            "case": name,
            "registry_index": 1,
            "prior_record_sha256": hex::encode(hash(&prior)),
            "compose_terminal_statuses": terminal_statuses,
            "post_exhaustion_evidence_status": evidence_status,
            "intentional_expected_failure": true,
            "integrated_store_save": false
        });
        write_ui_terminal(name, &terminal);
    }
}

#[test]
fn ui1_1_store_commit_reopen_preserves_prior_blob() {
    let name = "ui1_1_store_commit_reopen_preserves_prior_blob";
    let mut f = Fixture::new();
    let a = actor(1, 1);
    let e = f.endpoint(&a, 31, 7, 600000);
    assert_eq!(read(&f.host, &e, &a, 1), (1, OLD.to_vec()));
    assert_eq!(f.disk_selected(31), OLD);
    let canonical = ok(read_request(&a, 900).encode(e.handle));
    let raw = ok(encode_envelope_wire(&canonical));
    let untouched = f.files();
    let before_counts = f.counts(31);
    f.negative_phase("selectedtext_before_allocation");
    for offset in [14usize, 84] {
        let mut invalid = raw;
        invalid[offset] = 1;
        assert!(matches!(
            f.raw_negative(&invalid),
            Err(StoreStatus::Invalid)
        ));
    }
    for kind in 0..3 {
        let mut malformed = canonical;
        match kind {
            0 => malformed.payload_len -= 1,
            1 => malformed.payload[63] = 1,
            2 => malformed.handle.generation = 0,
            _ => unreachable!(),
        }
        assert!(matches!(
            f.host.dispatch(&e.peer, &malformed),
            Err(StoreStatus::Invalid)
        ));
    }
    let mut unsupported = canonical;
    unsupported.protocol = 369;
    assert!(matches!(
        f.host.dispatch(&e.peer, &unsupported),
        Err(StoreStatus::Unsupported)
    ));
    let direction = ok(artifact::ReadSelectedReply {
        request_id: 901,
        revision: 0,
        data_shm: 0,
        object_generation: 0,
        byte_len: 0,
        status: 1,
    }
    .encode(e.handle));
    assert!(matches!(
        f.host.dispatch(&e.peer, &direction),
        Err(StoreStatus::Unsupported)
    ));
    assert_eq!(f.files(), untouched);
    assert_eq!(f.counts(31), before_counts);
    // Typed descriptors must be checked before lossy Handle::pack. These are
    // actual own objects; no malformed wire frame is manufactured or decoded.
    let selected_reply: artifact::ReadSelectedReply =
        decode(&exchange(&f.host, &e, read_request(&a, 902)));
    assert_eq!(selected_reply.status, 0);
    let selected_descriptor = ObjectDescriptor {
        handle: Handle::unpack(selected_reply.data_shm),
        kind: SharedObjectKind::SelectedText,
        object_generation: selected_reply.object_generation,
        byte_len: selected_reply.byte_len,
    };
    assert_eq!(
        selected_descriptor.object_generation,
        selected_descriptor.handle.generation
    );
    assert_eq!(selected_descriptor.handle.pack(), selected_reply.data_shm);
    assert_eq!(selected_descriptor.byte_len as usize, 64 + OLD.len());
    let selected_lease = ok(f.host.read_lease(&e.peer, &selected_descriptor));
    let mut selected_bytes = vec![0; selected_descriptor.byte_len as usize];
    ok(selected_lease.copy_into(0, &mut selected_bytes));
    ok(ok(EditorTextHeaderV0::decode_le(&selected_bytes[..64]))
        .validate_bytes(&selected_bytes[64..]));
    assert_eq!(&selected_bytes[64..], OLD);
    let selected_files = f.files();
    let selected_counts = f.counts(31);
    f.deny_aliases(&e, &selected_descriptor);
    let mut selected_after = vec![0xa5; selected_bytes.len()];
    ok(selected_lease.copy_into(0, &mut selected_after));
    assert_eq!(selected_after, selected_bytes);
    assert_eq!(f.files(), selected_files);
    assert_eq!(f.counts(31), selected_counts);
    let old_cas = f.journal(31).0.parent().unwrap().join("cas");
    let old_file = old_cas.join(format!(
        "{}.blob",
        f.journal(31).1.initial.content_id().unwrap().hash_hex()
    ));
    let initial = bounded_read(&old_file, 4096);
    let read_only = f.endpoint(&a, 31, 1, 600000);
    redacted(
        &exchange(&f.host, &read_only, alloc_request(&a, 1, 2)),
        16,
        StoreStatus::Denied,
    );
    assert!(matches!(
        f.control.issue_editor(GrantSpec {
            actor: ok(EditorActorV0::try_new(2, 2, 1, 9, 1)),
            selected_object_id: 31,
            selected_generation: 1,
            rights: 7,
            ttl_ms: 600000
        }),
        Err(FixtureError {
            status: StoreStatus::Denied,
            ..
        })
    ));
    let mut wrong_instance = read_request(&a, 20);
    wrong_instance.instance_generation = 2;
    redacted(
        &exchange(&f.host, &e, wrong_instance),
        36,
        StoreStatus::Denied,
    );
    let foreign = actor(2, 2);
    let other = f.endpoint(&foreign, 32, 7, 600000);
    assert!(matches!(
        f.host.draft_source(&e.peer, 1, b"\x80"),
        Err(StoreStatus::Invalid)
    ));
    assert!(matches!(
        f.host.draft_source(&e.peer, 1, &vec![b'a'; 4097]),
        Err(StoreStatus::Invalid)
    ));
    let malformed = attempt(&f, &e, &a, 1, NEW, 3);
    let writer = ok(f.host.write_lease(&e.peer, &malformed.source));
    assert!(f.host.write_lease(&other.peer, &malformed.source).is_err());
    ok(writer.copy_from(60, &1u32.to_le_bytes()));
    redacted(
        &exchange(&f.host, &e, malformed.request),
        32,
        StoreStatus::Invalid,
    );
    assert_eq!(f.counts(31), (1, 0, 0));
    assert_eq!(f.disk_selected(31), OLD);
    let invalid_body = attempt(&f, &e, &a, 1, NEW, 40);
    let body_writer = ok(f.host.write_lease(&e.peer, &invalid_body.source));
    let mut invalid_bytes = NEW.to_vec();
    invalid_bytes[0] = 0x80;
    ok(body_writer.copy_from(64, &invalid_bytes));
    ok(body_writer.copy_from(24, &hash(&invalid_bytes)));
    redacted(
        &exchange(&f.host, &e, invalid_body.request),
        32,
        StoreStatus::Invalid,
    );
    assert_eq!(f.object(31).permit_count, 0);
    assert_eq!(f.disk_selected(31), OLD);
    let valid = attempt(&f, &e, &a, 1, NEW, 5);
    let alias = ok(f.host.write_lease(&e.peer, &valid.source));
    ok(alias.copy_from(64, NEW));
    let draft_files = f.files();
    let draft_counts = f.counts(31);
    f.deny_aliases(&e, &valid.source);
    // The retained writer stays live without repairing or changing any bytes.
    ok(alias.copy_from(valid.source.byte_len, b""));
    assert_eq!(f.files(), draft_files);
    assert_eq!(f.counts(31), draft_counts);
    // No source repair follows the denied attempts: the real Commit below must
    // still consume the original header/body/hash and publish exactly NEW.
    let r = committed(&f, &e, &a, &valid);
    assert!(alias.copy_from(64, b"X").is_err());
    assert!(f.host.read_lease(&e.peer, &valid.source).is_err());
    assert_eq!(f.disk_selected(31), NEW);
    assert_eq!(bounded_read(&old_file, 4096), initial);
    assert_io_completed(&f, valid.operation);
    let retained_reply = status_reply(&f.host, &e, &a, valid.operation, 903);
    let (receipt_status, receipt_operation, _, receipt_raw, receipt_len) =
        result_fields(&retained_reply);
    assert_eq!(
        (receipt_status, receipt_operation, receipt_len),
        (0, valid.operation, 176)
    );
    let receipt_handle = Handle::unpack(receipt_raw);
    assert_eq!(receipt_handle.pack(), receipt_raw);
    let receipt_descriptor = ObjectDescriptor {
        handle: receipt_handle,
        kind: SharedObjectKind::Receipt,
        object_generation: receipt_handle.generation,
        byte_len: receipt_len,
    };
    let receipt_lease = ok(f.host.read_lease(&e.peer, &receipt_descriptor));
    let mut receipt_bytes = [0; 176];
    ok(receipt_lease.copy_into(0, &mut receipt_bytes));
    assert_eq!(ok(EditorSaveReceiptV0::decode_le(&receipt_bytes)), r);
    let receipt_files = f.files();
    let receipt_counts = f.counts(31);
    f.deny_aliases(&e, &receipt_descriptor);
    let mut receipt_after = [0xa5; 176];
    ok(receipt_lease.copy_into(0, &mut receipt_after));
    assert_eq!(receipt_after, receipt_bytes);
    assert_eq!(f.files(), receipt_files);
    assert_eq!(f.counts(31), receipt_counts);
    let token = f.fence();
    let fence = f.reopen(token);
    let fresh_actor = actor(1, 3);
    let fresh = f.endpoint(&fresh_actor, 31, 7, 600000);
    assert_eq!(read(&f.host, &fresh, &fresh_actor, 7), (2, NEW.to_vec()));
    assert_eq!(bounded_read(&old_file, 4096), initial);
    let recovery = f.recovery(a.clone(), 31, valid.operation);
    assert_eq!(
        receipt(
            &f.host,
            &recovery,
            &status_reply(&f.host, &recovery, &a, valid.operation, 8)
        ),
        r
    );
    assert!(f.host.read_lease(&e.peer, &valid.source).is_err());
    f.emit(
        name,
        0,
        json!({"fence":fence,"receipt":r,"old_blob_hash":hash(&initial),"new_blob_hash":hash(NEW)}),
    );

    // All three named metadata failures deny reopen before an auto-recovery helper can write.
    for (leg, fault) in [
        StorageFault::UnsignedManifest,
        StorageFault::WrongSigningKey,
        StorageFault::CorruptBlob,
    ]
    .into_iter()
    .enumerate()
    {
        let mut bad = Fixture::new();
        let e = bad.endpoint(&a, 31, 7, 600000);
        let op = attempt(&bad, &e, &a, 1, NEW, 1);
        committed(&bad, &e, &a, &op);
        let mut token = bad.fence();
        ok(StoreFixture::corrupt_fenced(&mut token, 31, fault));
        let before = bad.files();
        let failure = match bad.try_reopen(token) {
            Err(failure) => failure,
            Ok(_) => panic!("unverified CAS accepted"),
        };
        assert!(matches!(
            failure.error.status,
            StoreStatus::Unknown | StoreStatus::NotReady
        ));
        assert_eq!(bad.files(), before);
        assert_fence(&failure.owner.fence_snapshot());
        bad.emit(
            name,
            leg as u32 + 1,
            json!({"corrupt_profile_leg":leg,"preflight_no_mutation":true}),
        );
    }
    // Deliberate corruption of this fixture's candidate slot while its IO owner is idle.
    // A content-addressed filename with different bytes never becomes authority.
    let collision = Fixture::new();
    let e = collision.endpoint(&a, 31, 7, 600000);
    let cas = collision.journal(31).0.parent().unwrap().join("cas");
    let candidate = cas.join(format!("{}.blob", hex::encode(hash(NEW))));
    ok(std::fs::write(
        &candidate,
        b"wrong bytes under expected candidate id",
    ));
    let corrupt_bytes = bounded_read(&candidate, 4096);
    let op = attempt(&collision, &e, &a, 1, NEW, 1);
    unknown(&exchange(&collision.host, &e, op.request), op.operation);
    assert_eq!(collision.counts(31), (1, 1, 0));
    assert_eq!(read(&collision.host, &e, &a, 3), (1, OLD.to_vec()));
    assert_eq!(bounded_read(&candidate, 4096), corrupt_bytes);
    redacted(
        &exchange(&collision.host, &e, alloc_request(&a, 1, 4)),
        16,
        StoreStatus::NotReady,
    );
    collision.emit(name,4,json!({"fixture_corruption":"candidate_blob_collision","invalid_candidate_hash":hash(&corrupt_bytes),"prior_selection_preserved":true,"quarantined":true}));
}

#[test]
fn ui1_1_store_same_base_conflict_and_operation_reuse() {
    let name = "ui1_1_store_same_base_conflict_and_operation_reuse";
    let mut f = Fixture::new();
    let a = actor(1, 1);
    let b = actor(2, 2);
    let ea = f.endpoint(&a, 31, 7, 600000);
    let eb = f.endpoint(&b, 31, 7, 600000);
    let aa = attempt(&f, &ea, &a, 1, NEW, 1);
    let ab = attempt(&f, &eb, &b, 1, b"loser\n", 10);
    let index = f.arm(&ea, PausePoint::BeforeCommitPermit);
    let pending = Pending::start(&f.host, &ea, aa.request);
    f.entered(index, aa.operation, false);
    redacted(
        &exchange(&f.host, &ea, aa.request),
        32,
        StoreStatus::NotReady,
    );
    redacted(
        &exchange(&f.host, &eb, ab.request),
        32,
        StoreStatus::NotReady,
    );
    assert_eq!(f.object(31).permit_count, 0);
    f.release(index);
    let winner = receipt(&f.host, &ea, &pending.finish());
    assert_eq!(winner.result_revision, 2);
    let conflict: artifact::CommitReply = decode(&exchange(&f.host, &eb, ab.request));
    assert_eq!(
        (
            conflict.status,
            conflict.revision,
            conflict.operation_id,
            conflict.receipt_shm,
            conflict.receipt_len
        ),
        (7, 2, 0, 0, 0)
    );
    let before = f.files();
    let counts = f.counts(31);
    for field in 0..6 {
        let mut changed = aa.request;
        changed.request_id = 100 + field;
        match field {
            0 => changed.session_id = b.session_id,
            1 => changed.session_generation = 2,
            2 => changed.expected_revision = 2,
            3 => changed.source_shm = ab.source.handle.pack(),
            4 => changed.byte_len += 1,
            5 => changed.operation_id = u64::MAX,
            _ => unreachable!(),
        }
        redacted(&exchange(&f.host, &ea, changed), 32, StoreStatus::Denied);
    }
    redacted(&exchange(&f.host, &eb, aa.request), 32, StoreStatus::Denied);
    let changed_instance = f.endpoint(&actor(1, 3), 31, 7, 600000);
    redacted(
        &exchange(&f.host, &changed_instance, aa.request),
        32,
        StoreStatus::Denied,
    );
    assert_eq!(f.files(), before);
    assert_eq!(f.counts(31), counts);
    let duplicate = receipt(&f.host, &ea, &exchange(&f.host, &ea, aa.request));
    assert_eq!(duplicate, winner);
    assert_eq!(f.counts(31), counts);
    assert_eq!(f.files(), before);
    ok(f.control.retire(&ea, RetirementReason::Revoked));
    redacted(&exchange(&f.host, &ea, aa.request), 32, StoreStatus::Stale);
    let explicit = attempt(&f, &eb, &b, 2, b"explicit next\n", 200);
    let next = committed(&f, &eb, &b, &explicit);
    assert_ne!(explicit.operation, ab.operation);
    assert_eq!(next.result_revision, 3);
    assert_eq!(f.object(31).permit_count, 2);
    assert_eq!(f.object(31).transition_count, 2);
    let recovery = f.recovery(a.clone(), 31, aa.operation);
    let recovery_receipt = receipt(
        &f.host,
        &recovery,
        &status_reply(&f.host, &recovery, &a, aa.operation, 300),
    );
    assert_eq!(recovery_receipt, winner);
    let files = f.files();
    let before = f.counts(31);
    redacted(
        &status_reply(&f.host, &recovery, &a, explicit.operation, 301),
        32,
        StoreStatus::Denied,
    );
    redacted(
        &exchange(&f.host, &recovery, read_request(&a, 302)),
        36,
        StoreStatus::Denied,
    );
    redacted(
        &exchange(&f.host, &recovery, alloc_request(&a, 3, 303)),
        16,
        StoreStatus::Denied,
    );
    let mut recovery_commit = aa.request;
    recovery_commit.request_id = 304;
    redacted(
        &exchange(&f.host, &recovery, recovery_commit),
        32,
        StoreStatus::Denied,
    );
    assert_eq!(f.files(), files);
    assert_eq!(f.counts(31), before);
    f.emit(name,0,json!({"original_receipt":winner,"duplicate_receipt":duplicate,"fresh_receipt":next,"original_counts":counts,"recovery_receipt":recovery_receipt,"recovery_wrong_operation_and_mutations_denied":true}));
}

#[test]
fn ui1_1_store_lost_reply_reconciles_without_mutation_replay() {
    let name = "ui1_1_store_lost_reply_reconciles_without_mutation_replay";
    // Definitive pre-permit deadline retires the exact source without waiting for the old dispatcher.
    let mut f = Fixture::new();
    let a = actor(1, 1);
    let e = f.endpoint(&a, 31, 7, 600000);
    let op = attempt(&f, &e, &a, 1, NEW, 1);
    let writer = ok(f.host.write_lease(&e.peer, &op.source));
    let index = f.arm(&e, PausePoint::BeforeCommitPermit);
    let start = Instant::now();
    let pending = Pending::start(&f.host, &e, op.request);
    f.entered(index, op.operation, false);
    redacted(&pending.finish(), 32, StoreStatus::Timeout);
    assert!((900..2500).contains(&elapsed_ms(start)));
    assert!(matches!(
        writer.copy_from(64, b"X"),
        Err(StoreStatus::Stale)
    ));
    let fresh = attempt(&f, &e, &a, 1, b"fresh explicit\n", 10);
    let saved = committed(&f, &e, &a, &fresh);
    assert_ne!(fresh.operation, op.operation);
    assert_eq!(f.counts(31), (2, 1, 1));
    let files = f.files();
    let original_io = operation_io(&f, 31, op.operation);
    assert!(
        !original_io.is_empty(),
        "actual submitted metadata IO was already retained"
    );
    f.release(index);
    assert_eq!(f.files(), files);
    assert_eq!(f.counts(31), (2, 1, 1));
    let after_release_io = operation_io(&f, 31, op.operation);
    assert_eq!(
        after_release_io, original_io,
        "closed original dispatcher may not even attempt more IO"
    );
    let closed = receipt(
        &f.host,
        &e,
        &status_reply(&f.host, &e, &a, op.operation, 12),
    );
    assert_eq!(closed.outcome, EditorReceiptOutcomeV0::DefinitiveNoncommit);
    assert_eq!(closed.result_revision, 0);
    no_replay(&f, 31, op.operation, 1);
    f.emit(
        name,
        0,
        json!({"timeout_noncommit":closed,"fresh_receipt":saved,"old_dispatcher_no_late_io":true,"original_io_before_release":original_io,"original_io_after_release":after_release_io}),
    );

    for (leg, after_permit) in [false, true].into_iter().enumerate() {
        let mut lost = Fixture::new();
        let e = lost.endpoint(&a, 31, 7, 600000);
        let op = attempt(&lost, &e, &a, 1, NEW, 1);
        let index = lost.arm(
            &e,
            if after_permit {
                PausePoint::AfterCommitPermit
            } else {
                PausePoint::BeforeCommitPermit
            },
        );
        ok(lost.control.drop_next_reply(&e));
        let pending = Pending::start(&lost.host, &e, op.request);
        lost.entered(index, op.operation, after_permit);
        redacted(&pending.finish(), 32, StoreStatus::Disconnected);
        assert_eq!(lost.counts(31), (1, u32::from(after_permit), 0));
        lost.release(index);
        let result = receipt(
            &lost.host,
            &e,
            &status_reply(&lost.host, &e, &a, op.operation, 4),
        );
        assert_eq!(
            result.outcome,
            if after_permit {
                EditorReceiptOutcomeV0::Committed
            } else {
                EditorReceiptOutcomeV0::DefinitiveNoncommit
            }
        );
        assert_eq!(
            lost.counts(31),
            (1, u32::from(after_permit), u32::from(after_permit))
        );
        no_replay(&lost, 31, op.operation, 1);
        let evidence = ok(lost.control.evidence());
        let suppressed = evidence
            .exchanges
            .iter()
            .find(|x| x.reply_suppressed)
            .unwrap();
        assert_eq!(suppressed.operation_id, op.operation);
        assert!(suppressed.actual_reply_wire.is_some());
        lost.emit(name,leg as u32+1,json!({"after_permit":after_permit,"known_operation":op.operation,"reconciled_receipt":result}));
    }
    // Supported error before a real call leaves the original permit uncertain; no guessed rollback.
    let mut failed = Fixture::new();
    let e = failed.endpoint(&a, 31, 7, 600000);
    let op = attempt(&failed, &e, &a, 1, NEW, 1);
    let pause = failed.arm(&e, PausePoint::AfterCommitPermit);
    let original = Pending::start(&failed.host, &e, op.request);
    failed.entered(pause, op.operation, true);
    ok(failed
        .control
        .fail_next_io(&e, IoPoint::JournalDirectorySync));
    failed.release(pause);
    unknown(&original.finish(), op.operation);
    let before_fence = failed.object(31);
    assert_eq!(before_fence.permit_count, 1);
    assert!(before_fence.transition_count <= 1);
    assert!(before_fence.mutation_quarantined);
    let record = before_fence
        .operations
        .iter()
        .find(|record| record.allocation.operation_id == op.operation)
        .unwrap();
    let binding = record.binding.as_ref().unwrap().clone();
    let permit = record.permit.as_ref().unwrap().clone();
    assert_eq!(binding.allocation.actor, a);
    assert_eq!(binding.allocation.operation_id, op.operation);
    assert_eq!(binding.source.source_object_id, op.source.handle.pack());
    assert_eq!(
        binding.source.source_generation,
        op.source.object_generation
    );
    assert_eq!(binding.source.content_hash, hash(NEW));
    assert_eq!(binding.allocation.expected_content_hash, hash(OLD));
    let io = ok(failed.control.evidence());
    let failures = io
        .io_events
        .iter()
        .filter(|event| {
            event.selected_object_id == 31
                && event.operation_id == op.operation
                && matches!(&event.point, IoPoint::JournalDirectorySync)
                && matches!(&event.outcome, IoOutcome::SimulatedFailure)
        })
        .collect::<Vec<_>>();
    assert_eq!(failures.len(), 1);
    let failed_sync = failures[0];
    assert_eq!(failed_sync.writer_id, permit.writer_id);
    let renamed = io
        .io_events
        .iter()
        .rfind(|event| {
            event.selected_object_id == 31
                && event.operation_id == op.operation
                && matches!(&event.point, IoPoint::JournalRename)
                && matches!(&event.outcome, IoOutcome::Completed)
                && event.sequence < failed_sync.sequence
        })
        .unwrap();
    assert_eq!(renamed.writer_id, permit.writer_id);
    let failure_sequence = failed_sync.sequence;
    let token = failed.fence();
    let fence = failed.reopen(token);
    let recovery = failed.recovery(a.clone(), 31, op.operation);
    let result = receipt(
        &failed.host,
        &recovery,
        &status_reply(&failed.host, &recovery, &a, op.operation, 3),
    );
    ok(result.validate_binding(&binding));
    assert_eq!(result.outcome, EditorReceiptOutcomeV0::Committed);
    assert_eq!(result.backend_service_epoch, permit.service_epoch);
    assert_eq!(result.result_revision, 2);
    assert_eq!(failed.disk_selected(31), NEW);
    let fresh_actor = actor(1, 2);
    let reader = failed.endpoint(&fresh_actor, 31, 1, 600000);
    assert_eq!(
        read(&failed.host, &reader, &fresh_actor, 4),
        (2, NEW.to_vec())
    );
    assert!(
        ok(failed.control.evidence())
            .exchanges
            .iter()
            .all(|x| x.request_wire[4..8] != 5u32.to_le_bytes())
    );
    failed.emit(name,3,json!({"named_io_failure":"journal_directory_sync","simulated_failure_sequence":failure_sequence,"original_binding":binding,"original_permit":permit,"fence":fence,"reconciled_receipt":result,"recovered_selected_hash":hash(NEW),"no_recommit_after_reopen":true}));
}

#[test]
fn ui1_1_store_recovery_validates_atomic_selection_and_receipt() {
    let name = "ui1_1_store_recovery_validates_atomic_selection_and_receipt";
    let a = actor(1, 1);
    for (leg, point) in [
        PausePoint::BeforeCandidateWrite,
        PausePoint::AfterCandidatePublication,
        PausePoint::AfterSelectionRename,
        PausePoint::AfterAcknowledgement,
    ]
    .into_iter()
    .enumerate()
    {
        let mut f = Fixture::new();
        let e = f.endpoint(&a, 31, 7, 600000);
        let op = attempt(&f, &e, &a, 1, NEW, 1);
        let index = f.arm(&e, point);
        let pending = Pending::start(&f.host, &e, op.request);
        let entered = f.entered(index, op.operation, true);
        let old_epoch = f.object(31).service_epoch;
        assert!(matches!(
            f.control.quiesce(1),
            Err(FixtureError {
                status: StoreStatus::NotReady,
                ..
            })
        ));
        assert_eq!(f.object(31).service_epoch, old_epoch);
        assert!(f.object(31).pending_writer_id.is_some());
        f.stop(index);
        let original = pending.finish();
        if leg < 3 {
            unknown(&original, op.operation);
        } else {
            // Acknowledged before retirement is a complete journal+receipt, even if caller publication lost the race.
            assert!(matches!(result_fields(&original).0, 0 | 10));
        }
        let token = f.fence();
        let snapshot = token.fence_snapshot();
        // A successful fence transfers exclusive reopening authority once.
        // The earlier timed-out quiesce may retry; an already issued owner may not.
        let before_second_fence = f.files();
        for _ in 0..2 {
            assert!(matches!(
                f.control.quiesce(2000),
                Err(FixtureError {
                    status: StoreStatus::NotReady,
                    ..
                })
            ));
        }
        assert_eq!(f.files(), before_second_fence);
        assert_eq!(
            ok(serde_json::to_vec(&token.fence_snapshot())),
            ok(serde_json::to_vec(&snapshot))
        );
        let object = snapshot
            .objects
            .iter()
            .find(|o| o.selected_object_id == 31)
            .unwrap();
        assert!(
            object
                .issued_runtime_permits
                .iter()
                .any(|p| p.binding.allocation.operation_id == op.operation)
        );
        assert!(
            object
                .known_durable_operations
                .iter()
                .any(|r| r.allocation.operation_id == op.operation)
        );
        let fence = f.reopen(token);
        let next = actor(1, 2);
        let reader = f.endpoint(&next, 31, 1, 600000);
        let current = read(&f.host, &reader, &next, 3);
        let recovery = f.recovery(a.clone(), 31, op.operation);
        let result = receipt(
            &f.host,
            &recovery,
            &status_reply(&f.host, &recovery, &a, op.operation, 4),
        );
        if leg < 2 {
            assert_eq!(current, (1, OLD.to_vec()));
            assert_eq!(result.outcome, EditorReceiptOutcomeV0::DefinitiveNoncommit);
        } else {
            assert_eq!(current, (2, NEW.to_vec()));
            assert_eq!(result.outcome, EditorReceiptOutcomeV0::Committed);
        }
        assert_eq!(f.disk_selected(31), current.1);
        assert!(f.files().iter().all(|file| {
            !file
                .path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".publication.json")
        }));
        assert!(
            ok(f.control.evidence())
                .exchanges
                .iter()
                .all(|x| x.request_wire[4..8] != 5u32.to_le_bytes())
        );
        f.emit(name,leg as u32,json!({"phase_index":leg,"entered":barrier_json(&entered),"fence":fence,"recovered_receipt":result,"selection_revision":current.0}));
    }
    for (leg, fault) in [
        StorageFault::MissingJournal,
        StorageFault::TruncatedJournal,
        StorageFault::UnknownJournalVersion,
        StorageFault::ExtraJournalField,
        StorageFault::DuplicateJournalField,
        StorageFault::WrongJournalBinding,
        StorageFault::BrokenJournalChain,
        StorageFault::EarlierAllocatedOnlyJournal,
        StorageFault::CorruptBlob,
        StorageFault::UnsignedManifest,
        StorageFault::WrongSigningKey,
        StorageFault::UnboundPublicationIntent,
        StorageFault::UnsignedPublicationIntent,
        StorageFault::SymlinkEntry,
    ]
    .into_iter()
    .enumerate()
    {
        let mut bad = Fixture::new();
        let e = bad.endpoint(&a, 31, 7, 600000);
        let op = attempt(&bad, &e, &a, 1, NEW, 1);
        // EarlierAllocatedOnly loses a known *Submitted* binding, not merely an acknowledged commit.
        if leg == 7 {
            let index = bad.arm(&e, PausePoint::BeforeCommitPermit);
            let pending = Pending::start(&bad.host, &e, op.request);
            bad.entered(index, op.operation, false);
            redacted(&pending.finish(), 32, StoreStatus::Timeout);
            bad.stop(index);
        } else {
            committed(&bad, &e, &a, &op);
        }
        let mut token = bad.fence();
        let original_fence = token.fence_snapshot();
        let witnesses = &original_fence
            .objects
            .iter()
            .find(|o| o.selected_object_id == 31)
            .unwrap()
            .known_durable_operations;
        assert!(
            witnesses
                .iter()
                .any(|r| r.allocation.operation_id == op.operation && r.binding.is_some())
        );
        ok(StoreFixture::corrupt_fenced(&mut token, 31, fault));
        let files = bad.files();
        let failure = match bad.try_reopen(token) {
            Err(failure) => failure,
            Ok(_) => panic!("corrupt/unsigned/unbound recovery accepted"),
        };
        assert!(matches!(
            failure.error.status,
            StoreStatus::Unknown | StoreStatus::NotReady
        ));
        assert_eq!(
            bad.files(),
            files,
            "preflight precedes helper recovery mutations"
        );
        assert_eq!(
            ok(serde_json::to_vec(&failure.owner.fence_snapshot())),
            ok(serde_json::to_vec(&original_fence))
        );
        bad.emit(name,leg as u32+4,json!({"fault_index":leg,"retained_fence":original_fence,"no_replay_or_recovery_mutation":true}));
    }
}
