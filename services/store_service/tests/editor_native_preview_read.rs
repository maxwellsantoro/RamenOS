//! Gate-first NPR Read1 precursor. Actual Store selection, typed keyboard tickets,
//! inactive admission, paired activation, original queries and owned real joins.
//! No Save, mutation permit, recovery/reopen, IO2 execution or target/device claim.
#![cfg(feature = "editor_native_preview_v0_dev")]
mod editor_native_preview_read_support;
use artifact_store_schema::editor_preview::EditorPreviewPhaseV0;
use desktop_service::dev::Status;
use desktop_service::editor_dev::{self as desktop, EditorMessage};
use editor_native_preview_read_support::*;
use kernel_api::generated::{desktop_artifact_v1 as artifact, desktop_editor_session_v1 as editor};
use std::sync::Arc;
use std::time::{Duration, Instant};
use store_service::editor_store::{self as store, StoreStatus};

#[test]
fn native_preview_current_selection() {
    let mut f = Fixture::new(2);
    let mut selector = f.selector(0, 1);
    let pin = f.pin(0, 30_000);
    let before = counts(&f.h);
    let bytes_before = files(&f.path);
    // The actual other owner cannot consume A's opaque holder/pin.
    denied(
        f.h.prepare_store_preview(&f.drivers[1].s.chrome.peer, &mut selector, &pin, 1),
        Status::Denied,
    );
    assert_eq!(counts(&f.h), before);
    assert_eq!(files(&f.path), bytes_before);
    let prepared = f.prepare(0, &mut selector, &pin, 1);
    let preview = f.preview_bytes(0, &prepared, 1);
    let metadata = data(&preview, 0, EditorPreviewPhaseV0::Preview);
    assert_eq!(
        (metadata.session_id, metadata.session_generation),
        (f.drivers[0].s.session_id, f.drivers[0].s.session_generation)
    );
    assert_eq!(prepared.granted_rights, 63);
    // Actual preview render has no genuine actor yet; it cannot fabricate 248.
    let preview_frame = success(f.h.compose_next(&f.drivers[0].s.compositor, 1));
    frame(&preview_frame, false, false);
    denied(
        f.c.preview_chrome_observation(&f.drivers[0].s.compositor),
        Status::NotReady,
    );
    denied(
        f.c.preview_chrome_observation(&f.drivers[1].s.chrome),
        Status::Denied,
    );
    let preview_descriptor = ui::desc(prepared.preview_shm, desktop::ObjectKind::Preview, 464);
    let immutable = success(f.h.read_lease(&f.drivers[0].s.chrome.peer, &preview_descriptor, 1));
    denied(
        f.h.write_lease(&f.drivers[0].s.chrome.peer, &preview_descriptor, 0, 1),
        Status::Denied,
    );
    denied(
        f.h.read_lease(&f.drivers[1].s.chrome.peer, &preview_descriptor, 1),
        Status::Denied,
    );
    let mut approval = f.approval(0, &prepared, 1);
    let pending = f.pending(0, &mut approval, 1);
    let active = success(f.prepared().activate_read(&pending));
    let mut holder = Some(success(f.h.take_store_launch(
        &f.drivers[0].s.chrome.peer,
        &pending,
        active.stamp(),
        1,
    )));
    let a = success(active.pair_launch(&mut holder));
    let d = active_data(&f, 0, &a, 1);
    assert_eq!(d.selected_content_hash, metadata.selected_content_hash);
    assert_eq!(d.selected_revision, metadata.selected_revision);
    let pixels = present(&f, 0, &a, 1, 1);
    frame(&pixels, false, true);
    let original = chrome(&f, 0, &pixels, &d, false);
    let b = f.launch(1, 2);
    f.positive(1, &b, 10, 2);
    let entry = success(f.h.begin_preview_read(
        a.desktop().bindings().artifact_origin(),
        &read_request(&a, 11),
        2,
    ));
    let call = success(entry.wait());
    let registered = success(f.host().register_preview_read(&a.endpoint().peer, call));
    let reply = success(f.host().execute_preview_read(&registered));
    selected(&reply, 101, A, 11);
    let selected_lease = reply.selected().unwrap();
    let descriptor = selected_lease.descriptor();
    let files_before = files(&f.path);
    success(f.prepared().controller().fault_preview_object(101));
    // Fault is a real irreversible fixture object lifecycle transition; not a
    // mutation/epoch setter. Previously returned bytes stay historical only.
    assert_eq!(
        success(
            artifact_store_schema::editor_preview::EditorPreviewChromeV0::decode_le(
                &original.bytes
            )
        )
        .state,
        artifact_store_schema::editor_preview::EditorPreviewChromeStateV0::NoSave
    );
    denied(
        f.c.preview_chrome_observation(&f.drivers[0].s.compositor),
        Status::Stale,
    );
    let mut sentinel = vec![0xa5; descriptor.byte_len as usize];
    denied(
        selected_lease.copy_into(0, &mut sentinel),
        StoreStatus::Stale,
    );
    assert!(sentinel.iter().all(|b| *b == 0xa5));
    denied(
        f.h.begin_preview_read(
            a.desktop().bindings().artifact_origin(),
            &read_request(&a, 12),
            2,
        ),
        Status::Stale,
    );
    let mut preview_out = vec![0xa5; 464];
    denied(immutable.copy_into(0, &mut preview_out, 2), Status::Stale);
    assert_eq!(preview_out, vec![0xa5; 464]);
    assert_eq!(files(&f.path), files_before);
    let terminal = success(f.h.compose_next(&f.drivers[0].s.compositor, 2));
    assert_eq!(
        (
            terminal.session_id,
            terminal.instance_id,
            terminal.focus_epoch,
            terminal.sequence
        ),
        (d.session_id, d.instance_id, 0, 0)
    );
    frame(&terminal, true, true);
    let unavailable = chrome(&f, 0, &terminal, &d, true);
    assert!(unavailable.status_version > original.status_version);
    denied(
        f.h.compose_next(&f.drivers[0].s.compositor, 2),
        Status::NotReady,
    );
    f.positive(1, &b, 13, 2);
    frame(&present(&f, 1, &b, 1, 2), false, true);
    for id in entry.producer_ids() {
        f.join(id);
    }
    for id in reply.producer_ids() {
        f.join(id);
    }
    drop((
        reply, registered, entry, a, b, active, pending, approval, selector, pin, immutable,
    ));
    f.finish();
}

#[test]
fn native_preview_fresh_approval_activation() {
    let mut f = Fixture::new(2);
    let mut selector = f.selector(0, 1);
    let pin = f.pin(0, 30_000);
    let preview = f.prepare(0, &mut selector, &pin, 1);
    let original_request = *selector.request();
    let bypass_before = counts(&f.h);
    status_reply(
        &f.h.dispatch(&f.drivers[0].s.chrome.peer, &canonical(original_request), 1),
        &original_request,
        2,
        36,
        Status::Unsupported as u32,
    );
    assert_eq!(counts(&f.h), bypass_before);
    // No fresh accepted release/press => no second approval event.
    denied(
        f.h.take_preview_action(&f.drivers[0].s.chrome.peer),
        Status::NotReady,
    );
    let duplicate = f.drivers[0].request(40, 1, 0);
    let duplicate_wire = canonical(success(duplicate.encode(f.drivers[0].s.input.handle)));
    let bad = f.h.dispatch(&f.drivers[0].s.input.peer, &duplicate_wire, 1);
    assert_eq!(ui::u32_at(&bad.payload, 8), Status::Invalid as u32);
    assert_eq!(ui::u64_at(&bad.payload, 0), duplicate.sequence);
    ui::zero_reply(&bad, 8, Status::Invalid);
    let mut approval = f.approval(0, &preview, 1);
    let aq = canonical(*approval.request());
    assert!(approval.view().input_sequence > selector.view().input_sequence);
    assert_eq!(aq.protocol, 352);
    assert_eq!(aq.msg_type, 5);
    let bypass_before = counts(&f.h);
    status_reply(
        &f.h.dispatch(&f.drivers[0].s.chrome.peer, &aq, 1),
        &aq,
        6,
        44,
        Status::Unsupported as u32,
    );
    assert_eq!(counts(&f.h), bypass_before);
    let before = counts(&f.h);
    denied(
        f.h.confirm_store_preview(&f.drivers[1].s.chrome.peer, &mut approval, 1),
        Status::Denied,
    );
    assert_eq!(counts(&f.h), before);
    let pending = Arc::new(f.pending(0, &mut approval, 1));
    let pause = f.preview_pause(0, desktop::PreviewPausePoint::BeforeActivation);
    let owner = f.store.as_ref().unwrap().clone();
    let p = pending.clone();
    let caller = OwnedCaller::new(
        std::thread::spawn(move || owner.activate_read(&p)),
        f.c.clone(),
        pause.clone(),
    );
    let stage = success(pause.wait_until_entered(500));
    assert!(stage.entered && !stage.settled);
    assert_eq!(stage.point, desktop::PreviewPausePoint::BeforeActivation);
    let before = counts(&f.h);
    assert_eq!(before[2], 1);
    assert_eq!(before[3], 0);
    let shared_before = f.h.native_producer_counts();
    let files_before = files(&f.path);
    denied(
        f.c.probe_inactive_preview(&f.drivers[1].s.chrome.peer, &pending, &pause, 1),
        Status::Denied,
    );
    denied(
        f.prepared().controller().probe_inactive_preview_read(
            &f.drivers[1].s.chrome.peer,
            &pending,
            &pause,
        ),
        StoreStatus::Denied,
    );
    let observed =
        success(f.c.probe_inactive_preview(&f.drivers[0].s.chrome.peer, &pending, &pause, 1));
    for (j, (q, r)) in observed
        .endpoint_requests
        .iter()
        .zip(observed.endpoint_replies.iter())
        .enumerate()
    {
        let (protocol, msg, reply, offset) = [(352, 7, 8, 44), (832, 3, 4, 36), (833, 1, 2, 40)][j];
        assert_eq!((q.protocol, q.msg_type), (protocol, msg));
        assert_eq!(ui::u64_at(&q.payload, 0), approval.view().input_sequence);
        assert_ne!(q.handle, 0);
        status_reply(r, q, reply, offset, Status::NotReady as u32);
    }
    denied(observed.origin_status, Status::NotReady);
    denied(observed.bootstrap_status, Status::NotReady);
    denied(observed.grants_lease_status, Status::NotReady);
    let store_probe = success(f.prepared().controller().probe_inactive_preview_read(
        &f.drivers[0].s.chrome.peer,
        &pending,
        &pause,
    ));
    denied(store_probe.grant_status, StoreStatus::NotReady);
    assert_eq!(counts(&f.h), before);
    assert_eq!(f.h.native_producer_counts(), shared_before);
    assert_eq!(files(&f.path), files_before);
    success(f.c.release_store_preview(&pause));
    let active = success(caller.join());
    assert!(success(pause.wait_until_settled(500)).settled);
    denied(
        f.c.probe_inactive_preview(&f.drivers[0].s.chrome.peer, &pending, &pause, 1),
        Status::NotReady,
    );
    denied(
        f.h.take_store_launch(&f.drivers[1].s.chrome.peer, &pending, active.stamp(), 1)
            .map_err(|e| e.status()),
        Status::Denied,
    );
    let mut holder = Some(success(f.h.take_store_launch(
        &f.drivers[0].s.chrome.peer,
        &pending,
        active.stamp(),
        1,
    )));
    let launch = holder.as_ref().unwrap();
    let cr = success(editor::ConfirmLaunchReply::decode(&canonical(
        *launch.confirm_reply(),
    )));
    assert_eq!(cr.request_id, approval.view().input_sequence);
    assert_ne!(cr.request_id, ui::u64_at(&original_request.payload, 0));
    let b = success(editor::InstanceBootstrap::decode(&canonical(
        *launch.bindings().bootstrap(),
    )));
    assert_eq!(launch.bindings().bootstrap().msg_type, 17);
    denied(f.c.editor_bindings(b.instance_id), Status::Denied);
    let paired = success(active.pair_launch(&mut holder));
    let data = active_data(&f, 0, &paired, 1);
    assert_eq!(data.records[3].handle, paired.endpoint().handle.pack());
    assert_eq!(counts(&f.h)[3], 1);
    let read = read_request(&paired, 30);
    let foreign_entry =
        success(f.h.begin_preview_read(paired.desktop().bindings().artifact_origin(), &read, 1));
    let foreign_call = success(foreign_entry.wait());
    let foreign = Fixture::new(1);
    denied(
        foreign
            .host()
            .register_preview_read(&paired.endpoint().peer, foreign_call),
        StoreStatus::Denied,
    );
    for id in foreign_entry.producer_ids() {
        f.join(id);
    }
    drop(foreign_entry);
    foreign.finish();
    let mut wrong = read;
    wrong.handle = f.drivers[0].s.chrome.handle;
    denied(
        f.h.begin_preview_read(paired.desktop().bindings().artifact_origin(), &wrong, 1),
        Status::Denied,
    );
    let mut bad = read;
    bad.payload[40] = 1;
    denied(
        f.h.begin_preview_read(paired.desktop().bindings().artifact_origin(), &bad, 1),
        Status::Invalid,
    );
    for q in [
        success(
            artifact::AllocateSaveId {
                request_id: 31,
                session_id: b.session_id,
                session_generation: b.session_generation,
                expected_revision: 1,
            }
            .encode(paired.endpoint().handle),
        ),
        success(
            artifact::Commit {
                request_id: 32,
                session_id: b.session_id,
                session_generation: b.session_generation,
                expected_revision: 1,
                operation_id: 1,
                source_shm: 0,
                byte_len: 0,
                reserved: 0,
            }
            .encode(paired.endpoint().handle),
        ),
        success(
            artifact::SaveStatus {
                request_id: 33,
                session_id: b.session_id,
                session_generation: b.session_generation,
                operation_id: 1,
            }
            .encode(paired.endpoint().handle),
        ),
    ] {
        denied(
            f.h.begin_preview_read(
                paired.desktop().bindings().artifact_origin(),
                &canonical(q),
                1,
            ),
            Status::Unsupported,
        );
    }
    f.positive(0, &paired, 34, 1);
    frame(&present(&f, 0, &paired, 1, 1), false, true);
    f.drivers[0].send(&f.h, 40, 2, 1);
    // Actual old query expires; deadline closure alone preserves the instance.
    let pause_index = f.read_pause(0);
    let entry = success(f.h.begin_preview_read(
        paired.desktop().bindings().artifact_origin(),
        &read_request(&paired, 35),
        1,
    ));
    let barrier = success(f.read_pauses[pause_index].wait_until_entered(500));
    assert!(barrier.entered);
    denied(entry.wait(), Status::Timeout);
    assert_eq!(f.h.native_producer_counts().held, 2);
    success(f.c.release_store_read(&f.read_pauses[pause_index]));
    assert!(success(f.read_pauses[pause_index].wait_until_settled(500)).settled);
    for id in entry.producer_ids() {
        f.join(id);
    }
    f.positive(0, &paired, 36, 2);
    // Store's supported BeforeReadReply pause is reached by the original
    // actual query; delegation cannot create another1000ms lifetime.
    let store_pause = f.store_pause(&paired);
    let q = read_request(&paired, 39);
    let store_entry =
        success(f.h.begin_preview_read(paired.desktop().bindings().artifact_origin(), &q, 2));
    let call = success(store_entry.wait());
    let original_deadline = call.deadline();
    let registered = Arc::new(success(
        f.host()
            .register_preview_read(&paired.endpoint().peer, call),
    ));
    let owner = f.store.as_ref().unwrap().clone();
    let registered_for_thread = registered.clone();
    let host = f.host();
    let store_caller = OwnedStoreCaller::new(
        std::thread::spawn(move || host.execute_preview_read(&registered_for_thread)),
        owner,
        store_pause.clone(),
    );
    let entered = success(store_pause.wait_until_entered(500));
    assert!(entered.entered);
    let timeout = success(store_caller.join());
    assert!(Instant::now() >= original_deadline);
    status_reply(&timeout.wire(), &q, 2, 36, StoreStatus::Timeout as u32);
    assert!(timeout.selected().is_none());
    assert_eq!(timeout.producer_ids().len(), 2);
    assert_eq!(f.h.native_producer_counts().held, 4);
    success(f.prepared().controller().release(&store_pause));
    assert!(success(store_pause.wait_until_settled(500)).settled);
    for id in store_entry.producer_ids() {
        f.join(id);
    }
    for id in timeout.producer_ids() {
        f.join(id);
    }
    f.positive(0, &paired, 40, 2);
    drop((registered, timeout, store_entry, store_pause));
    // Expired setup/taken holder Drop does not revoke a delivered Active row.
    drop((selector, approval, pin));
    success(f.h.maintenance(1002));
    f.positive(0, &paired, 37, 1002);
    success(f.h.maintenance(data.expires_at_ms));
    denied(
        f.h.begin_preview_read(
            paired.desktop().bindings().artifact_origin(),
            &read_request(&paired, 38),
            data.expires_at_ms,
        ),
        Status::Stale,
    );
    drop((paired, active, pending, entry, pause));
    f.finish();
}

#[test]
fn native_preview_pin_cancel_expiry_race() {
    // Each leg reaches the real staged inactive barrier before the actual
    // lifecycle transition. The external caller is retained and joined.
    for cause in 0..6 {
        let mut f = Fixture::new(2);
        let mut selector = f.selector(0, 1);
        let pin = f.pin(0, if cause == 4 { 10 } else { 30_000 });
        let preview = f.prepare(0, &mut selector, &pin, 1);
        let mut approval = f.approval(0, &preview, 1);
        let pending = Arc::new(f.pending(0, &mut approval, 1));
        let pause = f.preview_pause(0, desktop::PreviewPausePoint::BeforeActivation);
        let owner = f.store.as_ref().unwrap().clone();
        let p = pending.clone();
        let caller = OwnedCaller::new(
            std::thread::spawn(move || owner.activate_read(&p)),
            f.c.clone(),
            pause.clone(),
        );
        let entered = success(pause.wait_until_entered(500));
        assert!(entered.entered && !entered.settled);
        assert_eq!(counts(&f.h)[2], 1);
        let before = files(&f.path);
        match cause {
            0 => cancel(&f, 0, preview.plan_id, 1, Status::Ok),
            1 => success(f.c.set_policy_revision(f.drivers[0].s.session_id, 2)),
            2 => f.drivers[0].send(&f.h, 0, 3, 1),
            3 => success(f.c.detach_keyboard(f.drivers[0].s.session_id, 1)),
            4 => success(f.h.maintenance(11)),
            5 => success(f.prepared().controller().fault_preview_object(101)),
            _ => unreachable!(),
        }
        success(f.c.release_store_preview(&pause));
        let error = match caller.join() {
            Err(e) => e,
            Ok(_) => panic!("retired staged attempt activated"),
        };
        assert_eq!(error.status(), StoreStatus::Stale);
        let o = error.observation();
        assert_eq!(o.status, StoreStatus::Stale);
        assert!(o.pending_retained && o.inactive_store_grant_retained && !o.active);
        assert!(success(pause.wait_until_settled(500)).settled);
        assert_eq!(counts(&f.h)[3], 0);
        assert_eq!(files(&f.path), before);
        // Unrelated B remains responsive through actual Core/Grant/Read copy.
        let b = f.launch(1, if cause == 4 { 12 } else { 2 });
        f.positive(1, &b, 50, if cause == 4 { 12 } else { 2 });
        if cause == 0 {
            f.drivers[0].send(&f.h, 40, 2, 2);
            let a = f.launch(0, 2);
            f.positive(0, &a, 51, 2);
            drop(a);
        }
        drop((b, error, pending, approval, selector, pin, pause));
        f.finish();
    }
    // Consumed Selector deadline does not become the fresh Approval deadline.
    {
        let mut f = Fixture::new(1);
        let mut t = f.selector(0, 1);
        let pin = f.pin(0, 30_000);
        let preview = f.prepare(0, &mut t, &pin, 1);
        success(f.h.maintenance(1002));
        let mut approval = f.approval(0, &preview, 1002);
        let pending = f.pending(0, &mut approval, 1002);
        let activation = success(f.prepared().activate_read(&pending));
        let mut holder = Some(success(f.h.take_store_launch(
            &f.drivers[0].s.chrome.peer,
            &pending,
            activation.stamp(),
            1002,
        )));
        let live = success(activation.pair_launch(&mut holder));
        let d = active_data(&f, 0, &live, 1002);
        assert_eq!(
            d.expires_at_ms,
            1002 + 30_000,
            "instance TTL anchored at actual confirming event"
        );
        cancel(&f, 0, preview.plan_id, 1002, Status::NotReady);
        f.positive(0, &live, 52, 1002);
        let reply = success(
            f.host().execute_preview_read(&success(
                f.host().register_preview_read(
                    &live.endpoint().peer,
                    success(
                        success(f.h.begin_preview_read(
                            live.desktop().bindings().artifact_origin(),
                            &read_request(&live, 53),
                            1002,
                        ))
                        .wait(),
                    ),
                ),
            )),
        );
        let retained = reply.selected().unwrap();
        let desc = retained.descriptor();
        retire(&f, 0, &live, 1002);
        let mut sentinel = vec![0xa5; desc.byte_len as usize];
        denied(retained.copy_into(0, &mut sentinel), StoreStatus::Stale);
        assert!(sentinel.iter().all(|b| *b == 0xa5));
        denied(
            f.h.begin_preview_read(
                live.desktop().bindings().artifact_origin(),
                &read_request(&live, 54),
                1002,
            ),
            Status::Stale,
        );
        drop((reply, live, activation, pending, approval, t, pin));
        f.finish();
    }
    // A taken ticket's original real Instant deadline is not reset by take.
    {
        let mut f = Fixture::new(1);
        let mut t = f.selector(0, 1);
        let pin = f.pin(0, 30_000);
        let v = t.view();
        std::thread::sleep(
            v.deadline.saturating_duration_since(Instant::now()) + Duration::from_millis(2),
        );
        denied(
            f.h.prepare_store_preview(&f.drivers[0].s.chrome.peer, &mut t, &pin, 1),
            Status::Stale,
        );
        assert_eq!(counts(&f.h)[3], 0);
        f.drivers[0].send(&f.h, 40, 2, 1);
        drop((t, pin));
        let live = f.launch(0, 2);
        f.positive(0, &live, 55, 2);
        drop(live);
        f.finish();
    }
    // Drop closes only that unconsumed action: B's holder remains usable.
    {
        let mut f = Fixture::new(2);
        let a = f.selector(0, 1);
        let mut b = f.selector(1, 1);
        let pin = f.pin(1, 30_000);
        drop(a);
        let p = f.prepare(1, &mut b, &pin, 1);
        let mut approve = f.approval(1, &p, 1);
        let pending = f.pending(1, &mut approve, 1);
        let active = success(f.prepared().activate_read(&pending));
        let mut holder = Some(success(f.h.take_store_launch(
            &f.drivers[1].s.chrome.peer,
            &pending,
            active.stamp(),
            1,
        )));
        let live = success(active.pair_launch(&mut holder));
        f.positive(1, &live, 56, 1);
        drop((live, active, pending, approve, b, pin));
        f.finish();
    }
    // A paused renderer's old NoSave snapshot must not publish after fault.
    {
        let mut f = Fixture::new(2);
        let a = Arc::new(f.launch(0, 1));
        let d = active_data(&f, 0, &a, 1);
        let b = f.launch(1, 1);
        freeze(&f, 0, &a, 1, 1);
        let pause = f.preview_pause(0, desktop::PreviewPausePoint::BeforeFramePublish);
        let h = f.h.clone();
        let compositor = f.drivers[0].s.compositor.clone();
        let caller = OwnedCaller::new(
            std::thread::spawn(move || h.compose_next(&compositor, 1)),
            f.c.clone(),
            pause.clone(),
        );
        let entered = success(pause.wait_until_entered(500));
        assert!(entered.entered && !entered.settled);
        assert_eq!(
            entered.point,
            desktop::PreviewPausePoint::BeforeFramePublish
        );
        success(f.prepared().controller().fault_preview_object(101));
        success(f.c.release_store_preview(&pause));
        denied(caller.join(), Status::Stale);
        assert!(success(pause.wait_until_settled(500)).settled);
        let terminal = success(f.h.compose_next(&f.drivers[0].s.compositor, 1));
        assert_eq!(
            (
                terminal.session_id,
                terminal.instance_id,
                terminal.sequence,
                terminal.focus_epoch
            ),
            (d.session_id, d.instance_id, 0, 0)
        );
        frame(&terminal, true, true);
        chrome(&f, 0, &terminal, &d, true);
        denied(
            f.h.compose_next(&f.drivers[0].s.compositor, 1),
            Status::NotReady,
        );
        frame(&present(&f, 1, &b, 1, 1), false, true);
        f.positive(1, &b, 57, 1);
        drop((a, b, pause));
        f.finish();
    }
}

#[test]
fn native_preview_init_exposure_failure() {
    // Preflight identity failure must not initialize a fake ready Store.
    for mismatch in 0..2 {
        let (h, c, witness, enrollment) = success(desktop::HostDesktop::new_store_preview_read(
            ui::config(30_000, 30_000),
        ));
        let session = success(c.register_store_session(1, ui::APP, ui::MANIFEST, 101, 1, 1));
        let root = temporary();
        let path = root.path().join("owner");
        let mut p = profile(1);
        if mismatch == 0 {
            p.initial_objects[0].owner_id = 99;
        } else {
            p.initial_objects[0].object_id = 999;
        }
        let error = match store::StoreFixture::prepare_preview_read(&path, p, witness, enrollment) {
            Err(e) => e,
            Ok(_) => panic!("profile mismatch exposed Store"),
        };
        assert_eq!(error.status(), StoreStatus::Denied);
        let o = error.observation();
        assert_eq!(o.stage, store::PreviewInitStage::Validate);
        assert!(!o.panicked);
        assert_eq!(o.root, store::ReadRootObservation::NotAttempted);
        assert_eq!(h.native_producer_counts().held, 0);
        assert_eq!(
            std::fs::symlink_metadata(&path).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
        drop((error, session, c, h));
        let p = root.path().to_owned();
        success(root.close());
        assert_eq!(
            std::fs::symlink_metadata(p).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
    }
    {
        let (h, c, witness, enrollment) = success(desktop::HostDesktop::new_store_preview_read(
            ui::config(30_000, 30_000),
        ));
        let session = success(c.register_store_session(1, ui::APP, ui::MANIFEST, 101, 1, 1));
        let root = temporary();
        let path = root.path().join("owner");
        success(std::fs::create_dir(&path));
        success(std::fs::write(
            path.join("sentinel"),
            b"preexisting-owned-fixture",
        ));
        let before = files(&path);
        let error =
            match store::StoreFixture::prepare_preview_read(&path, profile(1), witness, enrollment)
            {
                Err(e) => e,
                Ok(_) => panic!("preexisting root reused"),
            };
        assert_eq!(error.status(), StoreStatus::Internal);
        let o = error.observation();
        assert_eq!(o.stage, store::PreviewInitStage::CreateRoot);
        assert!(o.core_retained && o.issuer_retained && !o.panicked);
        assert_eq!(o.root, store::ReadRootObservation::AttemptUncertain);
        assert_eq!(o.root_path.as_deref(), Some(path.as_path()));
        assert_eq!(files(&path), before);
        assert_eq!(h.native_producer_counts().held, 0);
        assert_eq!(counts(&h)[3], 0);
        drop((error, session, c, h));
        assert_eq!(
            files(&path),
            before,
            "dropping retained init owner cannot delete preexisting data"
        );
        success(root.close());
    }
    // Foreign mutable holders and activation/launch pairing leave the same
    // rightful carriers intact. No reconstructed view or numeric stamp is used.
    {
        let mut f = Fixture::new(2);
        let mut foreign = Fixture::new(1);
        let mut t = f.selector(0, 1);
        let pin = f.pin(0, 30_000);
        let v = t.view();
        let before = counts(&f.h);
        denied(
            foreign
                .h
                .prepare_store_preview(&foreign.drivers[0].s.chrome.peer, &mut t, &pin, 1),
            Status::Denied,
        );
        assert_eq!(t.view().ticket_id, v.ticket_id);
        assert_eq!(counts(&f.h), before);
        let other_pin = foreign.pin(0, 30_000);
        denied(
            f.h.prepare_store_preview(&f.drivers[0].s.chrome.peer, &mut t, &other_pin, 1),
            Status::Denied,
        );
        assert_eq!(t.view().ticket_id, v.ticket_id);
        assert_eq!(counts(&f.h), before);
        drop(other_pin);
        let p = f.prepare(0, &mut t, &pin, 1);
        let mut approval = f.approval(0, &p, 1);
        let pending = f.pending(0, &mut approval, 1);
        let e = match foreign.prepared().activate_read(&pending) {
            Err(e) => e,
            Ok(_) => panic!("foreign Core activated"),
        };
        assert_eq!(e.status(), StoreStatus::Denied);
        let o = e.observation();
        assert!(!o.pending_retained && !o.inactive_store_grant_retained && !o.active);
        let active = success(f.prepared().activate_read(&pending));
        let mut ft = foreign.selector(0, 1);
        let fp = foreign.pin(0, 30_000);
        let fr = foreign.prepare(0, &mut ft, &fp, 1);
        let mut fa = foreign.approval(0, &fr, 1);
        let fpend = foreign.pending(0, &mut fa, 1);
        let factive = success(foreign.prepared().activate_read(&fpend));
        let before = counts(&f.h);
        denied(
            f.h.take_store_launch(&f.drivers[0].s.chrome.peer, &pending, factive.stamp(), 1)
                .map_err(|e| e.status()),
            Status::Denied,
        );
        assert_eq!(counts(&f.h), before);
        let e = match f.h.take_store_launch(
            &foreign.drivers[0].s.chrome.peer,
            &pending,
            active.stamp(),
            1,
        ) {
            Err(e) => e,
            Ok(_) => panic!("foreign Chrome delivery"),
        };
        assert_eq!(e.status(), Status::Denied);
        let o = e.observation();
        assert!(!o.authorized_attempt && !o.active && !o.retired);
        assert_eq!(counts(&f.h), before);
        let mut held = Some(success(f.h.take_store_launch(
            &f.drivers[0].s.chrome.peer,
            &pending,
            active.stamp(),
            1,
        )));
        let original_wire = success(desktop::encode_envelope_wire(
            held.as_ref().unwrap().confirm_reply(),
        ));
        let e = match factive.pair_launch(&mut held) {
            Err(e) => e,
            Ok(_) => panic!("foreign Core paired"),
        };
        assert_eq!(e.status(), StoreStatus::Denied);
        assert!(held.is_some());
        assert_eq!(
            success(desktop::encode_envelope_wire(
                held.as_ref().unwrap().confirm_reply()
            )),
            original_wire
        );
        assert_eq!(counts(&f.h), before);
        let live = success(active.pair_launch(&mut held));
        assert!(held.is_none());
        f.positive(0, &live, 70, 1);
        match active.pair_launch(&mut held) {
            Err(e) => assert_eq!(e.status(), StoreStatus::NotReady),
            Ok(_) => panic!("launch reused"),
        }
        // Pair is checked after successful delivery against instance lifetime,
        // not consumed setup deadline. Existing Active remains readable.
        success(f.h.maintenance(1002));
        f.positive(0, &live, 71, 1002);
        let mut foreignheld = Some(success(foreign.h.take_store_launch(
            &foreign.drivers[0].s.chrome.peer,
            &fpend,
            factive.stamp(),
            1,
        )));
        let flive = success(factive.pair_launch(&mut foreignheld));
        foreign.positive(0, &flive, 72, 1);
        drop((
            live, active, pending, approval, t, pin, flive, factive, fpend, fa, ft, fp, e,
        ));
        f.finish();
        foreign.finish();
    }
    // Own failure after Active is actual retirement, never a rollback to absent.
    {
        let mut f = Fixture::new(2);
        let mut t = f.selector(0, 1);
        let pin = f.pin(0, 10);
        let preview = f.prepare(0, &mut t, &pin, 1);
        let mut approval = f.approval(0, &preview, 1);
        let pending = f.pending(0, &mut approval, 1);
        let active = success(f.prepared().activate_read(&pending));
        assert_eq!(counts(&f.h)[3], 1);
        success(f.h.maintenance(11));
        let e =
            match f
                .h
                .take_store_launch(&f.drivers[0].s.chrome.peer, &pending, active.stamp(), 11)
            {
                Err(e) => e,
                Ok(_) => panic!("expired setup delivered"),
            };
        assert_eq!(e.status(), Status::Stale);
        let o = e.observation();
        assert!(o.authorized_attempt && o.active && o.retired);
        assert_eq!(counts(&f.h)[3], 0);
        assert!(counts(&f.h)[4] >= 1);
        let b = f.launch(1, 12);
        f.positive(1, &b, 73, 12);
        drop((e, b, active, pending, approval, t, pin));
        f.finish();
    }
}

#[test]
fn native_preview_caps_versions_actual_joins() {
    // Two actual selected objects, one live pin per object, retained alias cap.
    {
        let mut f = Fixture::new(2);
        let a = f.pin(0, 30_000);
        let b = f.pin(1, 30_000);
        assert_eq!(counts(&f.h)[0], 2);
        denied(
            f.prepared()
                .pin_selection(&f.drivers[0].s.chrome.peer, 30_000),
            StoreStatus::NotReady,
        );
        drop((a, b));
        assert_eq!(counts(&f.h)[0], 0);
        let mut pins = Vec::new();
        let mut tickets = Vec::new();
        let mut last_ticket = 0;
        for _ in 0..16 {
            let mut t = f.selector(0, 1);
            assert!(t.view().ticket_id > last_ticket);
            last_ticket = t.view().ticket_id;
            let pin = f.pin(0, 30_000);
            let p = f.prepare(0, &mut t, &pin, 1);
            cancel(&f, 0, p.plan_id, 1, Status::Ok);
            f.drivers[0].send(&f.h, 40, 2, 1);
            pins.push(pin);
            tickets.push(t);
        }
        assert_eq!(counts(&f.h)[1], 16);
        assert_eq!(counts(&f.h)[6], 16);
        assert_eq!(counts(&f.h)[0], 0);
        let before = counts(&f.h);
        denied(
            f.prepared()
                .pin_selection(&f.drivers[0].s.chrome.peer, 30_000),
            StoreStatus::Exhausted,
        );
        assert_eq!(counts(&f.h), before);
        f.drivers[0].launcher(&f.h, 1);
        let q = f.drivers[0].request(40, 1, 0);
        let wire = canonical(success(q.encode(f.drivers[0].s.input.handle)));
        let r = f.h.dispatch(&f.drivers[0].s.input.peer, &wire, 1);
        ui::zero_reply(&r, 8, Status::Exhausted);
        assert_eq!(ui::u64_at(&r.payload, 0), q.sequence);
        assert_eq!(counts(&f.h), before);
        drop((pins, tickets));
        let pin = f.pin(0, 30_000);
        drop(pin);
        let t = f.selector(1, 2);
        assert!(t.view().ticket_id > last_ticket);
        drop(t);
        f.finish();
    }
    // Historical view-lease charges are distinct from descriptor/pin identity.
    {
        let mut f = Fixture::new(1);
        let mut t = f.selector(0, 1);
        let pin = f.pin(0, 30_000);
        let p = f.prepare(0, &mut t, &pin, 1);
        let descriptor = ui::desc(p.preview_shm, desktop::ObjectKind::Preview, 464);
        let mut leases = Vec::new();
        for _ in 0..16 {
            leases.push(success(f.h.read_lease(
                &f.drivers[0].s.chrome.peer,
                &descriptor,
                1,
            )));
        }
        assert_eq!(counts(&f.h)[5], 16);
        denied(
            f.h.read_lease(&f.drivers[0].s.chrome.peer, &descriptor, 1),
            Status::Exhausted,
        );
        let mut bytes = vec![0; 464];
        success(leases[0].copy_into(0, &mut bytes, 1));
        data(&bytes, 0, EditorPreviewPhaseV0::Preview);
        drop(leases);
        let lease = success(f.h.read_lease(&f.drivers[0].s.chrome.peer, &descriptor, 1));
        drop((lease, t, pin));
        f.finish();
    }
    // Sixteen retained actual instance/read contexts are separate from three
    // registered Desktop endpoints per instance, the fourth is a Store Grant.
    {
        let mut f = Fixture::new(1);
        let mut live = Vec::new();
        let mut last = 0;
        for n in 0..16 {
            let instance = f.launch(0, 1);
            let b = boot(&instance);
            assert!(b.instance_id > last);
            last = b.instance_id;
            assert_eq!(counts(&f.h)[8], n + 1);
            f.positive(0, &instance, 100 + n as u64, 1);
            retire(&f, 0, &instance, 1);
            live.push(instance);
        }
        assert_eq!(counts(&f.h)[8], 16);
        let mut t = f.selector(0, 1);
        let pin = f.pin(0, 30_000);
        let p = f.prepare(0, &mut t, &pin, 1);
        let mut a = f.approval(0, &p, 1);
        let before = counts(&f.h);
        denied(
            f.h.confirm_store_preview(&f.drivers[0].s.chrome.peer, &mut a, 1),
            Status::Exhausted,
        );
        assert_eq!(counts(&f.h)[2], before[2]);
        assert_eq!(
            counts(&f.h)[8],
            before[8],
            "failed17th Confirm cannot admit a private context"
        );
        assert_eq!(
            counts(&f.h)[9],
            before[9],
            "failed17th Confirm cannot admit a query row"
        );
        assert_eq!(f.h.native_producer_counts().held, 0);
        drop((live, t, pin, a));
        f.finish();
    }
    // Each checked overflow seed is exercised by its real next transition,
    // keeping unrelated actual live B reads and charged joins usable.
    for counter in [
        desktop::NativePreviewCounter::Pin,
        desktop::NativePreviewCounter::AttachmentVersion,
        desktop::NativePreviewCounter::StatusVersion,
        desktop::NativePreviewCounter::UiTicket,
        desktop::NativePreviewCounter::ReadContext,
    ] {
        let mut f = Fixture::new(2);
        let b = f.launch(1, 1);
        f.positive(1, &b, 200, 1);
        if counter == desktop::NativePreviewCounter::UiTicket {
            success(f.c.seed_preview_counter(counter));
            f.drivers[0].launcher(&f.h, 1);
            let q = f.drivers[0].request(40, 1, 0);
            let wire = canonical(success(q.encode(f.drivers[0].s.input.handle)));
            let before = counts(&f.h);
            let r = f.h.dispatch(&f.drivers[0].s.input.peer, &wire, 1);
            ui::zero_reply(&r, 8, Status::Exhausted);
            assert_eq!(ui::u64_at(&r.payload, 0), q.sequence);
            assert_eq!(counts(&f.h), before);
        } else {
            let mut t = f.selector(0, 1);
            if counter == desktop::NativePreviewCounter::Pin {
                success(f.c.seed_preview_counter(counter));
                let before = counts(&f.h);
                denied(
                    f.prepared()
                        .pin_selection(&f.drivers[0].s.chrome.peer, 30_000),
                    StoreStatus::Exhausted,
                );
                assert_eq!(counts(&f.h), before);
            } else {
                let pin = f.pin(0, 30_000);
                let p = f.prepare(0, &mut t, &pin, 1);
                let mut a = f.approval(0, &p, 1);
                success(f.c.seed_preview_counter(counter));
                let before = counts(&f.h);
                denied(
                    f.h.confirm_store_preview(&f.drivers[0].s.chrome.peer, &mut a, 1),
                    Status::Exhausted,
                );
                assert_eq!(counts(&f.h)[2], before[2]);
                assert_eq!(counts(&f.h)[3], 1);
                assert_eq!(
                    counts(&f.h)[8],
                    before[8],
                    "MAXfailed Confirm cannot admit a private context"
                );
                assert_eq!(
                    counts(&f.h)[9],
                    before[9],
                    "MAXfailed Confirm cannot admit a query row"
                );
                drop((pin, a));
            }
            drop(t);
        }
        f.positive(1, &b, 201, 1);
        assert_eq!(f.h.native_producer_counts().io, 0);
        drop(b);
        f.finish();
    }
    // Status version exhaustion cannot undo a genuine object fault or publish a
    // fabricated current248 record. B still has its unrelated Core object.
    {
        let mut f = Fixture::new(2);
        let a = f.launch(0, 1);
        let d = active_data(&f, 0, &a, 1);
        let b = f.launch(1, 1);
        let pixels = present(&f, 0, &a, 1, 1);
        let old = chrome(&f, 0, &pixels, &d, false);
        // This actual A read is admitted and copied before the MAX seed. Keep
        // its real lease and both installed producer pairs through retirement.
        let entry = success(f.h.begin_preview_read(
            a.desktop().bindings().artifact_origin(),
            &read_request(&a, 202),
            1,
        ));
        let call = success(entry.wait());
        let registered = success(f.host().register_preview_read(&a.endpoint().peer, call));
        let reply = success(f.host().execute_preview_read(&registered));
        selected(&reply, 101, A, 202);
        let lease = reply.selected().expect("actual pre-fault selected lease");
        let descriptor = lease.descriptor();
        assert_eq!(reply.producer_ids().len(), 2);
        assert_eq!(f.h.native_producer_counts().held, 4);
        success(f.c.seed_preview_counter(desktop::NativePreviewCounter::StatusVersion));
        match f.prepared().controller().fault_preview_object(101) {
            Err(e) => {
                assert_eq!(e.status, StoreStatus::Exhausted);
                assert!(matches!(e.reason, store::FixtureErrorKind::Exhausted));
            }
            Ok(_) => panic!("unrepresentable status version claimed success"),
        }
        let mut sentinel = vec![0xa5; descriptor.byte_len as usize];
        denied(
            lease.copy_checked(&descriptor, 0, &mut sentinel),
            StoreStatus::Stale,
        );
        assert!(sentinel.iter().all(|byte| *byte == 0xa5));
        assert_eq!(
            f.h.native_producer_counts().held,
            4,
            "real fault cannot silently free unjoined handles"
        );
        denied(
            f.h.begin_preview_read(
                a.desktop().bindings().artifact_origin(),
                &read_request(&a, 202),
                1,
            ),
            Status::Stale,
        );
        denied(
            f.c.preview_chrome_observation(&f.drivers[0].s.compositor),
            Status::Stale,
        );
        denied(
            f.h.compose_next(&f.drivers[0].s.compositor, 1),
            Status::Exhausted,
        );
        assert!(old.status_version > 0);
        for id in entry.producer_ids() {
            f.join(id);
        }
        for id in reply.producer_ids() {
            f.join(id);
        }
        assert_eq!(f.h.native_producer_counts().held, 0);
        f.positive(1, &b, 203, 1);
        drop((reply, registered, entry, a, b));
        f.finish();
    }
    // Actual32 paused Desktop pairs charge the same64 roster. An unfinished
    // dispatcher cannot be joined, and Timeout/Drop cannot free a held handle.
    {
        let mut f = Fixture::new(1);
        let live = f.launch(0, 1);
        let mut entries = Vec::new();
        let mut pauses = Vec::new();
        let mut actual_ids = std::collections::BTreeSet::new();
        let started = Instant::now();
        for n in 0..32 {
            let index = f.read_pause(0);
            let entry = success(f.h.begin_preview_read(
                live.desktop().bindings().artifact_origin(),
                &read_request(&live, 300 + n),
                1,
            ));
            let v = success(f.read_pauses[index].wait_until_entered(500));
            assert!(v.entered && !v.settled);
            assert!(actual_ids.insert(v.dispatcher) && actual_ids.insert(v.supervisor));
            denied(
                f.h.join_native_finished(entry.producer_ids()[0]),
                Status::NotReady,
            );
            assert_eq!(f.h.native_producer_counts().held, 2 * (n + 1) as u32);
            pauses.push(index);
            entries.push(entry);
            assert!(started.elapsed() < Duration::from_secs(5));
        }
        let before = counts(&f.h);
        let p = f.h.native_producer_counts();
        assert_eq!((p.held, p.io, p.joining), (64, 0, 0));
        denied(
            f.h.begin_preview_read(
                live.desktop().bindings().artifact_origin(),
                &read_request(&live, 333),
                1,
            ),
            Status::Exhausted,
        );
        assert_eq!(counts(&f.h), before);
        assert_eq!(f.h.native_producer_counts(), p);
        for index in &pauses {
            success(f.c.release_store_read(&f.read_pauses[*index]));
        }
        for (entry, index) in entries.iter().zip(&pauses) {
            match entry.wait() {
                Ok(call) => {
                    assert_eq!(
                        call.deadline().duration_since(call.entered()),
                        Duration::from_millis(1000)
                    );
                    drop(call);
                }
                Err(e) => assert_eq!(e, Status::Timeout),
            }
            assert!(success(f.read_pauses[*index].wait_until_settled(500)).settled);
            for id in entry.producer_ids() {
                f.join(id);
            }
        }
        assert_eq!(f.joined, actual_ids);
        assert_eq!(f.h.native_producer_counts().held, 0);
        assert_eq!(f.h.native_producer_counts().io, 0);
        let old = actual_ids.clone();
        f.positive(0, &live, 334, 2);
        assert_eq!(f.joined.len(), 68);
        assert!(old.is_subset(&f.joined));
        drop((entries, live));
        f.finish();
    }
    // Independent query cap: retain64 actual owners while their two installed
    // producers are genuinely joined between begins. No fake cap seed/worker.
    {
        let mut f = Fixture::new(1);
        let live = f.launch(0, 1);
        let mut entries = Vec::new();
        let mut calls = Vec::new();
        let started = Instant::now();
        let mut last_origin = 0;
        for n in 0..64 {
            assert_eq!(
                (
                    f.h.native_producer_counts().held,
                    f.h.native_producer_counts().io,
                    f.h.native_producer_counts().joining
                ),
                (0, 0, 0)
            );
            let entry = success(f.h.begin_preview_read(
                live.desktop().bindings().artifact_origin(),
                &read_request(&live, 400 + n),
                1,
            ));
            assert!(entry.origin_id() > last_origin);
            last_origin = entry.origin_id();
            let call = success(entry.wait());
            assert_eq!(
                call.deadline().duration_since(call.entered()),
                Duration::from_millis(1000)
            );
            for id in entry.producer_ids() {
                f.join(id);
            }
            entries.push(entry);
            calls.push(call);
            assert_eq!(counts(&f.h)[9], n as u32 + 1);
            assert!(started.elapsed() < Duration::from_secs(15));
        }
        assert_eq!(f.joined.len(), 128);
        let before = counts(&f.h);
        let p = f.h.native_producer_counts();
        assert_eq!(p.held, 0);
        denied(
            f.h.begin_preview_read(
                live.desktop().bindings().artifact_origin(),
                &read_request(&live, 464),
                1,
            ),
            Status::Exhausted,
        );
        assert_eq!(counts(&f.h), before);
        assert_eq!(f.h.native_producer_counts(), p);
        f.unchanged();
        drop((entries, calls));
        let entry = success(f.h.begin_preview_read(
            live.desktop().bindings().artifact_origin(),
            &read_request(&live, 465),
            1,
        ));
        assert!(entry.origin_id() > last_origin);
        assert_eq!(counts(&f.h)[9], 1);
        let call = success(entry.wait());
        let registered = success(f.host().register_preview_read(&live.endpoint().peer, call));
        let reply = success(f.host().execute_preview_read(&registered));
        selected(&reply, 101, A, 465);
        for id in entry.producer_ids() {
            f.join(id);
        }
        for id in reply.producer_ids() {
            f.join(id);
        }
        assert_eq!(f.joined.len(), 132);
        drop((entry, registered, reply, live));
        f.finish();
    }
}
