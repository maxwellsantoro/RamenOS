//! UI1.1c original eighteen: actual keyboard/application, concrete services and Store.
//! No model, backend callback, copied authorization verdict or app status setter.
#[path = "native_task_support/mod.rs"]
mod support;
use artifact_editor::NativeEditorError;
use artifact_store_schema::editor_save::*;
use desktop_service::dev::Status;
use desktop_service::editor_dev::{self as desktop, EditorMessage};
use kernel_api::cap::Handle;
use kernel_api::generated::{
    desktop_artifact_v1 as artifact, desktop_editor_session_v1 as session,
    desktop_focus_v1 as focus, desktop_surface_v1 as surface,
};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use store_service::editor_store::{self as store, StoreStatus};
use support::*;

#[test]
fn ui1_1_keyboard_edits_ascii_and_renders_owned_frame() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_keyboard_edits_ascii_and_renders_owned_frame",
    ));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_keyboard_edits_ascii_and_renders_owned_frame",
            "primary",
            0,
            recording::ScenarioScope::Composition,
        ),
        1,
    );
    let (mut app, endpoint) = f.app_with_endpoint(0, 101, 1);
    assert_eq!(ok(f.observe(&app, 1)).bytes, OLD);
    let old = f.composed(0, &mut app, 1);
    volatile::assert_golden(&old, false);
    f.replace(0, &mut app, NEW, 2);
    let o = ok(f.observe(&app, 2));
    assert_eq!((o.cursor, o.selection), (9, None));
    assert!(o.text_generation > 1);
    let new = f.composed(0, &mut app, 2);
    volatile::assert_golden(&new, true);
    assert!(new.sequence > old.sequence);
    assert_eq!(new.instance_id, old.instance_id);
    f.stroke(0, &mut app, 80, 3);
    assert_eq!(ok(f.observe(&app, 3)).cursor, 8);
    f.drivers[0].send(&f.h, 225, 1, 3);
    ok(app.step(3));
    f.stroke(0, &mut app, 80, 3);
    f.drivers[0].send(&f.h, 225, 2, 3);
    ok(app.step(3));
    assert_eq!(ok(f.observe(&app, 3)).selection, Some((7, 8)));
    let scroll=b"a\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\na\n";
    f.replace(0, &mut app, scroll, 3);
    assert!(ok(f.observe(&app, 3)).first_visible_line > 0);
    f.composed(0, &mut app, 3);
    f.replace(0, &mut app, NEW, 3);
    assert_eq!(ok(f.observe(&app, 3)).first_visible_line, 0);
    f.stroke(0, &mut app, 79, 3);
    f.chord(0, &mut app, 22, 4);
    f.until(&mut app, 4, |o| o.confirmed_revision == 2);
    // Receipt settlement alone is not the current protected frame's Saved join.
    assert!(!matches!(
        ok(f.observe(&app, 4)).phase,
        desktop::NativeEditorPhase::Saved
    ));
    assert!(matches!(
        f.dc.save_chrome_observation(&f.drivers[0].s.compositor),
        Err(Status::Stale)
    ));
    f.composed(0, &mut app, 4);
    assert!(matches!(
        ok(f.observe(&app, 4)).phase,
        desktop::NativeEditorPhase::Saved
    ));
    assert_eq!(f.counts(101), (1, 1, 1));
    let saved = object(&f, 101);
    app_chain_artifacts(
        &f,
        101,
        NEW,
        saved.operations[0].allocation.operation_id,
        &ok(f.observe(&app, 4)).actor,
        endpoint.handle,
    );
    f.stroke(0, &mut app, 5, 4);
    assert!(matches!(
        ok(f.observe(&app, 4)).phase,
        desktop::NativeEditorPhase::Unsaved
    ));
    let edited = f.composed(0, &mut app, 4);
    f.protected_golden(&edited, "UNSAVED", "");
    assert!(matches!(
        ok(f.dc.save_chrome_observation(&f.drivers[0].s.compositor)).current_document_phase,
        desktop::NativeEditorPhase::Unsaved
    ));
    drop((app, endpoint));
    f.reopen();
    let launch = f.launch(0, 101, 5);
    assert_eq!(selected(&mut f, &launch, 1000, 5), (2, NEW.to_vec()));
    drop(launch);
    f.finish();
}

#[test]
fn ui1_1_ascii_bounds_preserve_prior_draft() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_ascii_bounds_preserve_prior_draft",
    ));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_ascii_bounds_preserve_prior_draft",
            "primary",
            0,
            recording::ScenarioScope::Composition,
        ),
        1,
    );
    let (mut app, endpoint) = f.app_with_endpoint(0, 101, 1);
    let mut edge = vec![b'a'; 4094];
    edge.extend_from_slice(b"\t\n");
    f.replace(0, &mut app, &edge, 2);
    let prior_frame = f.composed(0, &mut app, 2);
    let before = ok(f.observe(&app, 2));
    assert_eq!(before.bytes.len(), 4096);
    // The 4097th legitimate ASCII key must not change text, cursor or selection.
    f.stroke(0, &mut app, 5, 2);
    let after = ok(f.observe(&app, 2));
    same_document(&before, &after);
    let initial = files(&f.path);
    for usage in [0, 0xffff, u32::MAX] {
        let q = f.drivers[0].request(usage, 1, 0);
        volatile::deny(&f.h, &f.drivers[0].s.input, q, 8, 2, Status::Invalid);
        same_document(&before, &ok(f.observe(&app, 2)));
    }
    let mut bad = f.drivers[0].request(4, 1, 0);
    bad.reserved = 1;
    assert!(matches!(
        bad.encode(f.drivers[0].s.input.handle),
        Err(Status::Invalid)
    ));
    // Exercise the handler's malformed-wire denial after checking typed encoding.
    bad.reserved = 0;
    let mut malformed = ok(bad.encode(f.drivers[0].s.input.handle));
    malformed.payload[44..48].copy_from_slice(&1_u32.to_le_bytes());
    let denied = f.h.dispatch(&f.drivers[0].s.input.peer, &malformed, 2);
    assert_eq!(&denied.payload[..8], &malformed.payload[24..32]);
    volatile::zero_reply(&denied, 8, Status::Invalid);
    same_document(&before, &ok(f.observe(&app, 2)));
    assert_eq!(files(&f.path), initial);
    let frame = f.composed(0, &mut app, 2);
    assert_eq!(frame.bgra.len(), 640 * 568 * 4);
    assert_eq!(
        &frame.bgra[48 * 2560..528 * 2560],
        &prior_frame.bgra[48 * 2560..528 * 2560]
    );
    f.save(0, &mut app, 3);
    app_chain_artifacts(
        &f,
        101,
        &edge,
        object(&f, 101).operations[0].allocation.operation_id,
        &ok(f.observe(&app, 3)).actor,
        endpoint.handle,
    );
    drop((app, endpoint));
    f.reopen();
    let p = f.launch(0, 101, 4);
    assert_eq!(selected(&mut f, &p, 1001, 4), (2, edge));
    drop(p);
    f.finish();
}

#[test]
fn ui1_1_focus_routes_reserved_keys_to_trusted_chrome() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_focus_routes_reserved_keys_to_trusted_chrome",
    ));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_focus_routes_reserved_keys_to_trusted_chrome",
            "primary",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        2,
    );
    let mut selector = f.selector(0, 1);
    let pin = ok(f
        .prepared()
        .pin_selection(&f.drivers[0].s.chrome.peer, 101, 30000));
    f.denied_at(
        "control_main_000",
        "prepare_save_preview",
        f.h.prepare_save_preview(&f.drivers[1].s.chrome.peer, &mut selector, &pin, 1),
        Status::Denied,
    );
    let own =
        ok(f.h.prepare_save_preview(&f.drivers[0].s.chrome.peer, &mut selector, &pin, 1));
    let p = ok(session::PrepareLaunchReply::decode(&own));
    assert_eq!(p.status, 0);
    let actual_preview = volatile::read(
        &f.h,
        &f.drivers[0].s.chrome,
        &volatile::desc(p.preview_shm, desktop::ObjectKind::Preview, p.preview_len),
        1,
    );
    f.record_grant("Preview", &own, &actual_preview);
    let no_effect = files(&f.path);
    for ep in [&f.drivers[0].s.input, &f.drivers[0].s.compositor] {
        let q = ok(session::ConfirmLaunch {
            request_id: 9,
            session_id: f.drivers[0].s.session_id,
            session_generation: f.drivers[0].s.session_generation,
            plan_id: p.plan_id,
            preview_revision: p.preview_revision,
        }
        .encode(ep.handle));
        volatile::zero_reply(&f.h.dispatch(&ep.peer, &q, 1), 44, Status::Unsupported);
    }
    let q = f.drivers[0].request(40, 1, 0);
    volatile::deny(
        &f.h,
        &f.drivers[0].s.compositor,
        q,
        8,
        1,
        Status::Unsupported,
    );
    // A held Enter cannot substitute for release+fresh approval.
    volatile::deny(
        &f.h,
        &f.drivers[0].s.input,
        f.drivers[0].request(40, 1, 0),
        8,
        1,
        Status::Invalid,
    );
    f.denied_at(
        "control_main_001",
        "take_save_preview_action",
        f.h.take_save_preview_action(&f.drivers[0].s.chrome.peer),
        Status::NotReady,
    );
    assert_eq!(files(&f.path), no_effect);
    let mut a = f.approval(0, 1);
    f.denied_at(
        "control_main_002",
        "confirm_save_preview",
        f.h.confirm_save_preview(&f.drivers[1].s.chrome.peer, &mut a, 1),
        Status::Denied,
    );
    let pending = Arc::new(ok(f.h.confirm_save_preview(
        &f.drivers[0].s.chrome.peer,
        &mut a,
        1,
    )));
    let pause = desktop_pause(&mut f, 0, desktop::NativeSavePausePoint::BeforeActivation);
    let prepared = f.prepared.as_ref().unwrap().clone();
    let pending_worker = pending.clone();
    let dc = f.dc.clone();
    let release = pause.clone();
    let caller = OwnedCaller::start(
        move || prepared.activate_save(&pending_worker),
        move || {
            let _ = dc.release_native_save(&release);
        },
    );
    assert!(ok(pause.wait_until_entered(1000)).entered);
    let inactive =
        ok(f.dc.probe_inactive_save(&f.drivers[0].s.chrome.peer, &pending, &pause, 1));
    f.record(|r| r.inactive_desktop("inactive_desktop", &inactive));
    for (q, r) in inactive
        .endpoint_requests
        .iter()
        .zip(&inactive.endpoint_replies)
    {
        assert_eq!(r.handle, Handle::INVALID);
        assert_eq!(&q.payload[..8], &r.payload[..8]);
        volatile::zero_reply(
            r,
            match r.msg_type {
                8 => 44,
                4 => 36,
                2 => 40,
                _ => panic!("closed actual inactive reply"),
            },
            Status::NotReady,
        );
    }
    f.denied_at(
        "control_main_003",
        "probe_inactive_save.origin_status",
        inactive.origin_status,
        Status::NotReady,
    );
    f.denied_at(
        "control_main_004",
        "probe_inactive_save.bootstrap_status",
        inactive.bootstrap_status,
        Status::NotReady,
    );
    f.denied_at(
        "control_main_005",
        "probe_inactive_save.grants_lease_status",
        inactive.grants_lease_status,
        Status::NotReady,
    );
    let store_inactive = ok(f.prepared().controller().probe_inactive_native_save(
        &f.drivers[0].s.chrome.peer,
        &pending,
        &pause,
    ));
    f.record(|r| r.inactive_store("inactive_store", &store_inactive));
    f.denied_at(
        "control_main_006",
        "probe_inactive_native_save.grant_status",
        store_inactive.grant_status,
        StoreStatus::NotReady,
    );
    assert_eq!(files(&f.path), no_effect);
    assert_eq!(f.counts(101), (0, 0, 0));
    ok(f.dc.release_native_save(&pause));
    let active = ok(caller.join(&f));
    assert!(ok(pause.wait_until_settled(1000)).settled);
    let mut holder = None;
    ok(f.h.take_save_launch(&f.drivers[0].s.chrome.peer, &mut holder, 1));
    let paired = ok(active.pair_launch(&mut holder));
    let boot = boot(&paired);
    let before_denials = files(&f.path);
    let bindings = paired.desktop().bindings();
    let self_status = bindings.self_status();
    let grants = volatile::read(
        &f.h,
        self_status,
        &volatile::desc(
            boot.grants_shm,
            desktop::ObjectKind::Grants,
            boot.grants_len,
        ),
        1,
    );
    f.record_grant("Active", paired.desktop().confirm_reply(), &grants);
    let issued = ok(
        artifact_store_schema::editor_native_save::EditorNativeSaveDataV0::decode_le_for_phase(
            &grants,
            artifact_store_schema::editor_native_save::EditorNativeSavePhaseV0::Active,
        ),
    );
    volatile::deny(
        &f.h,
        self_status,
        session::PrepareLaunch {
            request_id: 70,
            session_id: boot.session_id,
            session_generation: boot.session_generation,
            application_hash: volatile::APP,
            requested_rights: 63,
            reserved: 0,
        },
        36,
        1,
        Status::Denied,
    );
    volatile::deny(
        &f.h,
        self_status,
        session::ConfirmLaunch {
            request_id: 71,
            session_id: boot.session_id,
            session_generation: boot.session_generation,
            plan_id: p.plan_id,
            preview_revision: p.preview_revision,
        },
        44,
        1,
        Status::Denied,
    );
    volatile::deny(
        &f.h,
        self_status,
        session::PrepareRestart {
            request_id: 72,
            session_id: boot.session_id,
            session_generation: boot.session_generation,
            instance_id: boot.instance_id,
            instance_generation: boot.instance_generation,
        },
        36,
        1,
        Status::Denied,
    );
    volatile::deny(
        &f.h,
        bindings.surface(),
        f.drivers[0].request(4, 1, 0),
        8,
        1,
        Status::Denied,
    );
    volatile::deny(
        &f.h,
        bindings.surface(),
        focus::Assign {
            request_id: 73,
            session_id: boot.session_id,
            session_generation: boot.session_generation,
            instance_id: boot.instance_id,
            instance_generation: boot.instance_generation,
            surface_id: issued.records[2].resource_id,
            surface_generation: issued.records[2].resource_generation,
        },
        24,
        1,
        Status::Denied,
    );
    assert_eq!(files(&f.path), before_denials);
    assert_eq!(f.counts(101), (0, 0, 0));
    assert_eq!(selected(&mut f, &paired, 1002, 1), (1, OLD.to_vec()));
    assert_eq!(f.counts(101), (0, 0, 0));
    drop((paired, active, pending, a, pin, selector));
    f.finish();
}

#[test]
fn ui1_1_focus_epoch_reset_overflow_and_detach() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_focus_epoch_reset_overflow_and_detach",
    ));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_focus_epoch_reset_overflow_and_detach",
            "primary",
            0,
            recording::ScenarioScope::Composition,
        ),
        2,
    );
    let (mut app, endpoint, focus_endpoint) = f.app_with_controls(0, 101, 1);
    let before = ok(f.observe(&app, 1));
    f.drivers[0].send(&f.h, 224, 1, 2);
    f.drivers[0].send(&f.h, 4, 1, 2);
    f.drivers[0].stroke(&f.h, 41, 2);
    f.drivers[0].send(&f.h, 224, 2, 2);
    ok(app.step(2));
    assert_eq!(ok(f.observe(&app, 2)).bytes, before.bytes);
    let actor = before.actor.clone();
    let surface = volatile::latest::<surface::CreateReply>(&ok(f.dc.evidence()));
    let focus_reply: focus::AssignReply = volatile::call(
        &f.h,
        &f.drivers[0].s.chrome,
        focus::Assign {
            request_id: 24,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            instance_id: actor.instance_id,
            instance_generation: actor.instance_generation,
            surface_id: surface.surface_id,
            surface_generation: surface.surface_generation,
        },
        2,
    );
    assert_eq!(focus_reply.status, 0);
    // Release the genuinely held old-focus A; its suppressed release is not an edit.
    f.drivers[0].send(&f.h, 4, 2, 2);
    let overflow_sequence = f.drivers[0].next + 64;
    for _ in 0..32 {
        f.drivers[0].stroke(&f.h, 4, 2);
    }
    f.drivers[0].send(&f.h, 5, 1, 2);
    volatile::deny(
        &f.h,
        &f.drivers[0].s.input,
        f.drivers[0].request(4, 1, 0),
        8,
        2,
        Status::NotReady,
    );
    assert_eq!(f.drivers[0].next, overflow_sequence + 1);
    let reset: focus::PollKeysReply = volatile::call(
        &f.h,
        &focus_endpoint,
        focus::PollKeys {
            request_id: 26,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            instance_id: actor.instance_id,
            instance_generation: actor.instance_generation,
            focus_epoch: focus_reply.focus_epoch,
        },
        2,
    );
    assert_eq!(
        (
            reset.status,
            reset.sequence,
            reset.usage,
            reset.phase,
            reset.modifiers
        ),
        (0, overflow_sequence, 0, 3, 0)
    );
    same_document(&before, &ok(f.observe(&app, 2)));
    f.denied_at(
        "control_main_007",
        "take_save_preview_action",
        f.h.take_save_preview_action(&f.drivers[0].s.chrome.peer),
        Status::NotReady,
    );
    ok(f.dc.detach_keyboard(f.drivers[0].s.session_id, 3));
    let old_queue = f.drivers[0].queue;
    f.reattach_input(0, 4);
    assert!(f.drivers[0].queue > old_queue);
    f.denied_at(
        "control_main_008",
        "take_save_preview_action",
        f.h.take_save_preview_action(&f.drivers[0].s.chrome.peer),
        Status::NotReady,
    );
    let actor = before.actor;
    let r: focus::AssignReply = volatile::call(
        &f.h,
        &f.drivers[0].s.chrome,
        focus::Assign {
            request_id: 25,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            instance_id: actor.instance_id,
            instance_generation: actor.instance_generation,
            surface_id: volatile::latest::<surface::CreateReply>(&ok(f.dc.evidence())).surface_id,
            surface_generation: volatile::latest::<surface::CreateReply>(&ok(f.dc.evidence()))
                .surface_generation,
        },
        4,
    );
    assert_eq!(r.status, 0);
    f.stroke(0, &mut app, 5, 4);
    assert!(ok(f.observe(&app, 4)).bytes.ends_with(b"b"));
    assert_eq!(f.counts(101), (0, 0, 0));
    drop((app, endpoint, focus_endpoint));
    f.finish();
}

#[test]
fn ui1_1_surface_freeze_alias_quota_and_chrome_clip() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_surface_freeze_alias_quota_and_chrome_clip",
    ));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_surface_freeze_alias_quota_and_chrome_clip",
            "primary",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let raw = raw_editor(&mut f, 0, 101, 1);
    let obs = ok(f.h.render_native_editor(&raw.editor, 1));
    let before = ok(f.compose(0, 1));
    f.protected_golden(&before, "LOADED", "");
    let e = raw.launch.desktop().bindings().surface();
    let acquired: surface::AcquireReply = volatile::call(
        &f.h,
        e,
        surface::Acquire {
            request_id: 500,
            surface_id: obs.surface_id,
            surface_generation: obs.surface_generation,
            buffer_index: 0,
            reserved: 0,
        },
        2,
    );
    assert_eq!(acquired.status, 0);
    let d = volatile::desc(
        acquired.buffer_shm,
        desktop::ObjectKind::SurfaceBuffer,
        acquired.byte_len,
    );
    let writer = ok(f.h.write_lease(&e.peer, &d, acquired.mapping_generation, 2));
    let alias = writer.clone();
    let magenta = [255, 0, 255, 255].repeat(640 * 480);
    ok(writer.copy_from(0, &magenta, 2));
    let present = surface::Present {
        request_id: 501,
        surface_id: obs.surface_id,
        surface_generation: obs.surface_generation,
        mapping_generation: acquired.mapping_generation,
        sequence: before.sequence + 1,
        buffer_index: 0,
        reserved: 0,
    };
    let r: surface::PresentReply = volatile::call(&f.h, e, present, 2);
    assert_eq!(r.status, 0);
    f.denied_at(
        "surface_writer_retired",
        "SurfaceWriteLease.copy_from",
        writer.copy_from(0, &[0; 4], 2),
        Status::Stale,
    );
    f.denied_at(
        "surface_alias_retired",
        "SurfaceWriteLease.copy_from",
        alias.copy_from(0, &[0; 4], 2),
        Status::Stale,
    );
    let frozen = ok(f.h.read_lease(&f.drivers[0].s.compositor.peer, &d, 2));
    let mut b = vec![0; magenta.len()];
    ok(frozen.copy_into(0, &mut b, 2));
    assert_eq!(b, magenta);
    f.record(|r| r.app_frame_copy("surface_frozen_copy", &b));
    volatile::deny(&f.h, e, present, 16, 2, Status::Stale);
    let mut invalid = present;
    invalid.buffer_index = 2;
    volatile::deny(&f.h, e, invalid, 16, 2, Status::Invalid);
    let after = ok(f.compose(0, 2));
    assert_eq!(volatile::crop(&after), magenta);
    assert_eq!(protected(&before), protected(&after));
    f.protected_golden(&after, "LOADED", "");
    let mut sentinel = [0xa5; 4];
    f.denied_at(
        "surface_copy_consumed",
        "SurfaceReadLease.copy_into",
        frozen.copy_into(0, &mut sentinel, 2),
        Status::NotReady,
    );
    assert_eq!(sentinel, [0xa5; 4]);
    let fresh: surface::AcquireReply = volatile::call(
        &f.h,
        e,
        surface::Acquire {
            request_id: 502,
            surface_id: obs.surface_id,
            surface_generation: obs.surface_generation,
            buffer_index: 0,
            reserved: 0,
        },
        2,
    );
    assert_eq!(fresh.status, 0);
    assert!(fresh.mapping_generation > acquired.mapping_generation);
    let rendered = ok(f.h.render_native_editor(&raw.editor, 2));
    let next = ok(f.compose(0, 2));
    assert_eq!(
        (rendered.sequence, rendered.actor.instance_id),
        (next.sequence, next.instance_id)
    );
    assert!(next.sequence > after.sequence);
    f.unchanged();
    drop((raw, writer, alias, frozen));
    f.finish();
    // A genuine generic Present cannot inherit an older private Saved frame join.
    let mut saved = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_surface_freeze_alias_quota_and_chrome_clip",
            "saved_generic_present",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let raw = raw_editor(&mut saved, 0, 101, 1);
    raw_replace(&mut saved, 0, &raw, NEW, 2);
    let commit = attempt(&mut saved, 0, &raw, 510, 2);
    let reply = settle(&mut saved, &commit);
    join_attempt(&mut saved, &commit, &reply);
    assert_eq!(
        receipt(&reply, commit.operation).0.outcome,
        EditorReceiptOutcomeV0::Committed
    );
    let rendered = ok(saved.h.render_native_editor(&raw.editor, 2));
    let frame = ok(saved.compose(0, 2));
    saved_publication(
        &saved,
        0,
        &ok(saved.editor_observation(&raw.editor, 2)),
        &rendered,
        &frame,
    );
    let ep = raw.launch.desktop().bindings().surface();
    let acquired: surface::AcquireReply = volatile::call(
        &saved.h,
        ep,
        surface::Acquire {
            request_id: 512,
            surface_id: rendered.surface_id,
            surface_generation: rendered.surface_generation,
            buffer_index: 0,
            reserved: 0,
        },
        2,
    );
    assert_eq!(acquired.status, 0);
    let descriptor = volatile::desc(
        acquired.buffer_shm,
        desktop::ObjectKind::SurfaceBuffer,
        acquired.byte_len,
    );
    let writer = ok(saved
        .h
        .write_lease(&ep.peer, &descriptor, acquired.mapping_generation, 2));
    ok(writer.copy_from(0, &[255, 0, 255, 255].repeat(640 * 480), 2));
    let presented: surface::PresentReply = volatile::call(
        &saved.h,
        ep,
        surface::Present {
            request_id: 513,
            surface_id: rendered.surface_id,
            surface_generation: rendered.surface_generation,
            mapping_generation: acquired.mapping_generation,
            sequence: rendered.sequence + 1,
            buffer_index: 0,
            reserved: 0,
        },
        2,
    );
    assert_eq!(presented.status, 0);
    assert!(!matches!(
        ok(saved.editor_observation(&raw.editor, 2)).phase,
        desktop::NativeEditorPhase::Saved
    ));
    let unjoined = ok(saved.compose(0, 2));
    saved.protected_golden(&unjoined, "UNSAVED", "");
    assert!(!matches!(
        ok(saved
            .dc
            .save_chrome_observation(&saved.drivers[0].s.compositor))
        .label,
        desktop::NativeEditorPhase::Saved
    ));
    let fresh = ok(saved.h.render_native_editor(&raw.editor, 2));
    let current = ok(saved.compose(0, 2));
    saved_publication(
        &saved,
        0,
        &ok(saved.editor_observation(&raw.editor, 2)),
        &fresh,
        &current,
    );
    drop((writer, reply, commit, raw));
    saved.finish();
}

#[allow(clippy::manual_range_patterns)]
#[test]
fn ui1_1_wire_schema_descriptor_and_identity_fail_closed() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_wire_schema_descriptor_and_identity_fail_closed",
    ));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_wire_schema_descriptor_and_identity_fail_closed",
            "primary",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        2,
    );
    let p = f.launch(0, 101, 1);
    let own = selected_request(&p, 600);
    let before = files(&f.path);
    for variant in 0..7 {
        let mut q = own;
        match variant {
            0 => q.protocol = 352,
            1 => q.msg_type = 2,
            2 => q.msg_type = u32::MAX,
            3 => q.payload_len -= 1,
            4 => q.payload_len += 1,
            5 => q.payload[63] = 1,
            6 => q.handle = Handle::INVALID,
            _ => unreachable!(),
        };
        let result =
            f.h.begin_save_read(p.desktop().bindings().artifact_origin(), &q, 1);
        err(
            result,
            match variant {
                0 | 1 | 2 => Status::Unsupported,
                3 | 4 | 5 => Status::Invalid,
                6 => Status::Denied,
                _ => unreachable!(),
            },
        );
        assert_eq!(files(&f.path), before);
    }
    let wire = ok(desktop::encode_envelope_wire(&own));
    for offset in [14, 60, 84] {
        let mut bad = wire;
        bad[offset] ^= 0x80;
        let actual_decoder = desktop::decode_envelope_wire(&bad);
        assert!(actual_decoder.is_err());
        let actual_status = actual_decoder.err().unwrap();
        f.record(|r| {
            r.negative_wire(
                match offset {
                    14 => "wire_header_reserved",
                    60 => "wire_payload_reserved",
                    84 => "wire_tail_reserved",
                    _ => unreachable!(),
                },
                &bad,
                actual_status,
            )
        });
    }
    let entry =
        ok(f.h.begin_save_read(p.desktop().bindings().artifact_origin(), &own, 1));
    let reg = ok(f
        .host()
        .register_native_save(p.endpoint(), ok(entry.wait_once())));
    let reply = ok(f.host().execute_native_save(&reg));
    let lease = reply.selected().unwrap();
    let d = lease.descriptor();
    for variant in 0..5 {
        let mut forged = d.clone();
        match variant {
            0 => forged.handle.index += 1 << 16,
            1 => forged.handle.generation += 1 << 32,
            2 => forged.object_generation += 1,
            3 => forged.byte_len += 1,
            4 => forged.kind = store::SharedObjectKind::Receipt,
            _ => unreachable!(),
        };
        f.denied_at(
            "control_main_009",
            "selected_lease",
            f.host().selected_lease(&reg, &forged),
            StoreStatus::Denied,
        );
    }
    let mut sentinel = [0xa5; 4];
    f.denied_at(
        "control_main_010",
        "copy_into",
        lease.copy_into(u32::MAX, &mut sentinel),
        StoreStatus::Invalid,
    );
    assert_eq!(sentinel, [0xa5; 4]);
    let mut b = vec![0; d.byte_len as usize];
    ok(lease.copy_into(0, &mut b));
    assert_eq!(&b[64..], OLD);
    let foreign = f.launch(1, 102, 1);
    let bad = ok(f.h.begin_save_read(
        p.desktop().bindings().artifact_origin(),
        &selected_request(&p, 601),
        1,
    ));
    let denied = f
        .host()
        .register_native_save(foreign.endpoint(), ok(bad.wait_once()));
    assert!(matches!(denied,Err(ref e)if e.status()==StoreStatus::Denied));
    if let Err(ref failure) = denied {
        let joined_ids = failure
            .producers()
            .iter()
            .map(|id| join_core_observed(&mut f, id))
            .collect::<Vec<_>>();
        f.record(|r| r.registration_failure("foreign_registration", failure, &joined_ids));
    }
    assert_eq!(files(&f.path), before);
    for id in entry.producer_ids() {
        join_core(&mut f, id)
    }
    for id in reply.producers() {
        join_core(&mut f, id)
    }
    drop((reply, reg, entry, p, foreign));
    f.finish();
    for arm in 0..5 {
        let mut source_fixture = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_wire_schema_descriptor_and_identity_fail_closed",
                "malformed_source",
                arm as u32,
                recording::ScenarioScope::DirectNative,
            ),
            1,
        );
        let raw = raw_editor(&mut source_fixture, 0, 101, 1);
        raw_replace(&mut source_fixture, 0, &raw, NEW, 2);
        let staged = attempt(&mut source_fixture, 0, &raw, 650 + arm * 2, 2);
        let mut malformed = text_bytes(101, 1, NEW);
        match arm {
            0 => malformed[60] = 1,
            1 => malformed[64] = b'x',
            2 => {
                let mut body = NEW.to_vec();
                body[0] = 0x80;
                malformed = text_bytes(101, 1, &body);
            }
            3 => malformed = text_bytes(101, 1, b"note=bad\n"),
            4 => malformed[..4].copy_from_slice(&2u32.to_le_bytes()),
            _ => unreachable!(),
        }
        ok(ok(staged.source.write_lease()).copy_from(0, &malformed));
        let counts_before = source_fixture.counts(101);
        let rejected = settle(&mut source_fixture, &staged);
        source_fixture.record(|r| {
            r.negative_source(
                match arm {
                    0 => "source_reserved",
                    1 => "source_digest",
                    2 => "source_ascii",
                    3 => "source_snapshot",
                    4 => "source_schema",
                    _ => unreachable!(),
                },
                &staged.source.descriptor(),
                0,
                &malformed,
                StoreStatus::Ok,
                &staged.request,
                rejected.wire(),
                if arm == 3 {
                    StoreStatus::Denied
                } else {
                    StoreStatus::Invalid
                },
            )
        });
        commit_status(
            &rejected,
            if arm == 3 {
                StoreStatus::Denied
            } else {
                StoreStatus::Invalid
            },
            0,
        );
        assert_eq!(source_fixture.counts(101).1, counts_before.1);
        assert_eq!(source_fixture.counts(101).2, counts_before.2);
        assert_eq!(selected_blob(&source_fixture, 101), OLD);
        assert!(
            object(&source_fixture, 101)
                .operations
                .iter()
                .all(|r| r.permit.is_none())
        );
        assert!(
            !ok(source_fixture.prepared().controller().evidence())
                .io_events
                .iter()
                .any(|r| r.operation_id == staged.operation
                    && matches!(r.point, store::IoPoint::CandidateBlobWrite))
        );
        // Fresh explicit intent is required; the rejected original is never replayed.
        let valid = attempt(&mut source_fixture, 0, &raw, 670 + arm * 2, 3);
        let committed = settle(&mut source_fixture, &valid);
        assert_eq!(receipt(&committed, valid.operation).0.result_revision, 2);
        assert_ne!(valid.operation, staged.operation);
        assert_eq!(source_fixture.counts(101).1, 1);
        assert_eq!(source_fixture.counts(101).2, 1);
        drop((committed, valid, rejected, staged, raw));
        source_fixture.finish();
    }
}

#[test]
fn ui1_1_editor_grants_are_exact_and_preview_single_use() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_editor_grants_are_exact_and_preview_single_use",
    ));
    use artifact_store_schema::editor_native_save::{
        EditorNativeSaveDataV0, EditorNativeSavePhaseV0,
    };
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_editor_grants_are_exact_and_preview_single_use",
            "primary",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let (mut selector, pin, p) = f.prepare(0, 101, 1);
    let bytes = volatile::read(
        &f.h,
        &f.drivers[0].s.chrome,
        &volatile::desc(p.preview_shm, desktop::ObjectKind::Preview, p.preview_len),
        1,
    );
    let data = ok(EditorNativeSaveDataV0::decode_le_for_phase(
        &bytes,
        EditorNativeSavePhaseV0::Preview,
    ));
    assert_eq!(data.category_mask, 63);
    assert_eq!(data.selected_object_id, 101);
    assert_eq!(
        data.selected_content_hash,
        <sha2::Sha256 as sha2::Digest>::digest(OLD).as_slice()
    );
    assert_eq!(data.records.map(|r| r.rights), [1, 1, 15, 7]);
    assert!(data.records.iter().all(|r| r.handle == 0));
    f.denied_at(
        "control_main_011",
        "prepare_save_preview",
        f.h.prepare_save_preview(&f.drivers[0].s.chrome.peer, &mut selector, &pin, 1),
        Status::NotReady,
    );
    let mut approval = f.approval(0, 1);
    ok(f.dc.set_policy_revision(f.drivers[0].s.session_id, 2));
    f.denied_at(
        "control_main_012",
        "confirm_save_preview",
        f.h.confirm_save_preview(&f.drivers[0].s.chrome.peer, &mut approval, 1),
        Status::Stale,
    );
    f.unchanged();
    drop((approval, pin, selector));
    let pair = f.launch(0, 101, 2);
    let b = boot(&pair);
    let d = volatile::desc(b.grants_shm, desktop::ObjectKind::Grants, b.grants_len);
    let active_bytes = volatile::read(&f.h, pair.desktop().bindings().self_status(), &d, 2);
    f.record_grant("Active", pair.desktop().confirm_reply(), &active_bytes);
    let active = ok(EditorNativeSaveDataV0::decode_le_for_phase(
        &active_bytes,
        EditorNativeSavePhaseV0::Active,
    ));
    assert_eq!(active.records.map(|r| r.rights), [1, 1, 15, 7]);
    assert_eq!(active.records[3].handle, pair.endpoint().handle.pack());
    assert!(active.records.iter().all(|r| r.handle != 0));
    assert_eq!(
        (
            active.session_id,
            active.session_generation,
            active.instance_id,
            active.instance_generation
        ),
        (
            b.session_id,
            b.session_generation,
            b.instance_id,
            b.instance_generation
        )
    );
    assert_eq!(
        [
            active.records[0].handle,
            active.records[1].handle,
            active.records[2].handle
        ],
        [
            pair.desktop().bindings().self_status().handle.pack(),
            pair.desktop().bindings().focus_read().handle.pack(),
            pair.desktop().bindings().surface().handle.pack()
        ]
    );
    for r in &active.records[..2] {
        assert_eq!(
            (r.resource_id, r.resource_generation),
            (b.instance_id, b.instance_generation)
        );
    }
    let created: surface::CreateReply = volatile::call(
        &f.h,
        pair.desktop().bindings().surface(),
        surface::Create {
            request_id: 605,
            session_id: b.session_id,
            session_generation: b.session_generation,
            instance_id: b.instance_id,
            instance_generation: b.instance_generation,
            width: 640,
            height: 480,
            format: 1,
            reserved: 0,
        },
        2,
    );
    assert_eq!(created.status, 0);
    assert_eq!(
        (
            active.records[2].resource_id,
            active.records[2].resource_generation
        ),
        (created.surface_id, created.surface_generation)
    );
    let observed: session::GetStatusReply = volatile::call(
        &f.h,
        pair.desktop().bindings().self_status(),
        session::GetStatus {
            request_id: 606,
            session_id: b.session_id,
            session_generation: b.session_generation,
            instance_id: b.instance_id,
            instance_generation: b.instance_generation,
        },
        2,
    );
    assert_eq!(observed.status, 0);
    assert!(
        active.records[..3]
            .iter()
            .all(|r| r.service_epoch == observed.service_epoch)
    );
    let current = object(&f, 101);
    assert_eq!(
        (
            active.records[3].resource_id,
            active.records[3].resource_generation,
            active.records[3].service_epoch
        ),
        (
            current.selected.selected_object_id,
            current.selected.selected_generation,
            current.service_epoch
        )
    );

    assert_eq!(selected(&mut f, &pair, 602, 2), (1, OLD.to_vec()));
    drop(pair);
    f.finish();
    // Independent authentic application, manifest and preview-expiry invalidation.
    for invalidation in 0..3 {
        let mut stale = Fixture::recorded_ttl(
            &recording,
            scenario(
                "ui1_1_editor_grants_are_exact_and_preview_single_use",
                "approval_invalidation",
                invalidation,
                recording::ScenarioScope::DirectNative,
            ),
            1,
            5,
            600000,
        );
        let (selector, pin, _) = stale.prepare(0, 101, 1);
        let mut approval = stale.approval(0, 1);
        let before = files(&stale.path);
        if invalidation == 0 {
            ok(stale.dc.set_application_identity(
                stale.drivers[0].s.session_id,
                [7; 32],
                volatile::MANIFEST,
            ));
        }
        if invalidation == 1 {
            ok(stale.dc.set_application_identity(
                stale.drivers[0].s.session_id,
                volatile::APP,
                [8; 32],
            ));
        }
        let now = if invalidation == 2 { 7 } else { 1 };
        stale.denied_at(
            "approval_invalidated",
            "confirm_save_preview",
            stale
                .h
                .confirm_save_preview(&stale.drivers[0].s.chrome.peer, &mut approval, now),
            Status::Stale,
        );
        assert_eq!(files(&stale.path), before);
        assert_eq!(stale.counts(101), (0, 0, 0));
        assert_eq!(stale.producer_counts("producer_checkpoint").held, 0);
        drop((approval, pin, selector));
        ok(stale.dc.set_application_identity(
            stale.drivers[0].s.session_id,
            volatile::APP,
            volatile::MANIFEST,
        ));
        let current = stale.launch(0, 101, now + 1);
        assert_eq!(
            selected(&mut stale, &current, u64::from(610 + invalidation), now + 1),
            (1, OLD.to_vec())
        );
        drop(current);
        stale.finish();
    }
    // A pin issued later cannot cancel an already issued immutable Commit permit.
    let mut changed = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_editor_grants_are_exact_and_preview_single_use",
            "selection_invalidation",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let original = raw_editor(&mut changed, 0, 101, 1);
    raw_replace(&mut changed, 0, &original, NEW, 2);
    let commit = attempt(&mut changed, 0, &original, 620, 2);
    let pause = store_pause(
        &mut changed,
        &original,
        store::PausePoint::AfterCommitPermit,
    );
    let execution = ok(changed.host().start_native_save(&commit.commit));
    assert!(ok(pause.wait_until_entered(1000)).permit_issued);
    assert_eq!(object(&changed, 101).selected.revision, 1);
    let (selector, pin, preview) = changed.prepare(0, 101, 2);
    let mut approval = changed.approval(0, 2);
    assert_eq!(preview.status, 0);
    ok(changed.prepared().controller().release(&pause));
    let reply = pending_reply(&changed, &execution);
    assert!(ok(pause.wait_until_settled(1000)).settled);
    join_attempt(&mut changed, &commit, &reply);
    let (receipt, _) = receipt(&reply, commit.operation);
    assert_eq!(receipt.result_revision, 2);
    assert_eq!(object(&changed, 101).selected.revision, 2);
    changed.denied_at(
        "selection_invalidated",
        "confirm_save_preview",
        changed
            .h
            .confirm_save_preview(&changed.drivers[0].s.chrome.peer, &mut approval, 2),
        Status::Stale,
    );
    assert_eq!(changed.counts(101), (1, 1, 1));
    drop((approval, pin, selector, reply, execution, commit, original));
    let fresh = changed.launch(0, 101, 3);
    assert_eq!(selected(&mut changed, &fresh, 621, 3), (2, NEW.to_vec()));
    drop(fresh);
    changed.finish();
    // Separately stamped existing Read1 control: Save compilation never upgrades its actual grant.
    let mut read =
        read_control::Fixture::new("ui1_1_editor_grants_are_exact_and_preview_single_use", 1);
    let boot = read.rig.live[0].boot;
    let q = ok(artifact::AllocateSaveId {
        request_id: 604,
        session_id: boot.session_id,
        session_generation: boot.session_generation,
        expected_revision: 1,
    }
    .encode(read.endpoint(0).handle));
    let unsupported = read
        .rig
        .h
        .start_native_read(&read.rig.live[0].bindings.artifact.peer, &q, 1);
    let unsupported_status = *unsupported
        .as_ref()
        .err()
        .expect("actual Read1 mutation denial");
    assert_eq!(unsupported_status, Status::Unsupported);
    err(unsupported, Status::Unsupported);
    assert_eq!(read.host().native_read_producer_counts().held, 0);
    read_control::assert_unchanged(&read);
    let mut read_record = ok(recording.scenario(scenario(
        "ui1_1_editor_grants_are_exact_and_preview_single_use",
        "readonly_control",
        0,
        recording::ScenarioScope::NativeReadControl,
    )));
    ok(read_record.unit_control(
        "read_control_allocate_unsupported",
        "start_native_read",
        "Err",
        "DesktopStatus",
        unsupported_status as u32,
        Some(&q),
    ));
    let evidence = ok(read.native().controller().evidence());
    ok(read_record.store_snapshot(
        0,
        0,
        "final",
        "final_before_cleanup",
        &evidence,
        &read.store_path,
        None,
        None,
    ));
    ok(read_record.desktop(&ok(read.rig.c.evidence())));
    read.rig.release_all();
    let actual_joins = read.rig.join_all();
    let actual_counts = read.host().native_read_producer_counts();
    ok(read_record.producer_counts("final", &actual_counts));
    let root = read.root.take().unwrap();
    read.native.take();
    drop(read);
    ok(read_record.finish_native_read(root, actual_counts.held, &actual_joins));
}

#[test]
fn ui1_1_volatile_save_and_reopen_are_explicitly_labeled() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_volatile_save_and_reopen_are_explicitly_labeled",
    ));
    let mut volatile_fixture = volatile::Fixture::new(OLD);
    let mut volatile_record = ok(recording.scenario(scenario(
        "ui1_1_volatile_save_and_reopen_are_explicitly_labeled",
        "volatile_control",
        0,
        recording::ScenarioScope::VolatileControl,
    )));
    let mut live = volatile_fixture.launch(1);
    volatile_fixture
        .d
        .ctrl(&volatile_fixture.h, &mut live, 4, 2);
    volatile_fixture
        .d
        .text(&volatile_fixture.h, &mut live, NEW, 2);
    volatile_fixture
        .d
        .ctrl(&volatile_fixture.h, &mut live, 22, 2);
    let frame = volatile::render(&volatile_fixture.h, &volatile_fixture.d, &mut live, 2);
    volatile::assert_volatile_label(&frame);
    ok(volatile_record.frame(&frame, None, None, None));
    assert_eq!(
        volatile::selected(&volatile_fixture.h, &live, 2),
        (2, NEW.to_vec())
    );
    let volatile_evidence = ok(volatile_fixture.c.evidence());
    assert!(volatile_evidence.volatile_backend);
    let committed = volatile::latest::<artifact::CommitReply>(&volatile_evidence);
    assert_eq!(
        (committed.status, committed.revision, committed.receipt_len),
        (0, 2, 176)
    );
    let actual_receipt = volatile::receipt(
        &volatile_fixture.h,
        &live.bindings.artifact,
        &volatile_fixture.d.s,
        committed.operation_id,
        2,
    );
    ok(volatile_record.raw_bytes("volatile-receipt.bin", "receipt176", &actual_receipt));
    let decoded = ok(EditorSaveReceiptV0::decode_le(&actual_receipt));
    assert_eq!(
        (
            decoded.operation_id,
            decoded.result_revision,
            decoded.original_instance_id
        ),
        (committed.operation_id, 2, live.boot.instance_id)
    );
    let original = volatile_evidence
        .exchanges
        .iter()
        .find_map(|row| {
            let q = artifact::Commit::decode(&row.0).ok()?;
            (q.operation_id == committed.operation_id).then_some(q)
        })
        .unwrap();
    assert_eq!(
        (
            original.request_id,
            original.session_id,
            original.session_generation
        ),
        (
            committed.request_id,
            live.boot.session_id,
            live.boot.session_generation
        )
    );
    drop(live);
    let mut reopened_volatile = volatile_fixture.launch(3);
    assert_eq!(
        volatile::selected(&volatile_fixture.h, &reopened_volatile, 3),
        (2, NEW.to_vec())
    );
    let reopened_frame = volatile::render(
        &volatile_fixture.h,
        &volatile_fixture.d,
        &mut reopened_volatile,
        3,
    );
    volatile::assert_volatile_label(&reopened_frame);
    ok(volatile_record.frame(&reopened_frame, None, None, None));
    drop(reopened_volatile);
    let final_volatile = ok(volatile_fixture.c.evidence());
    ok(volatile_record.desktop(&final_volatile));
    drop(volatile_fixture);
    ok(volatile_record.finish_volatile(&final_volatile.ledger));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_volatile_save_and_reopen_are_explicitly_labeled",
            "primary",
            0,
            recording::ScenarioScope::Composition,
        ),
        1,
    );
    let (mut app, endpoint) = f.app_with_endpoint(0, 101, 1);
    f.replace(0, &mut app, NEW, 2);
    f.save(0, &mut app, 3);
    let frame = f.composed(0, &mut app, 3);
    f.protected_golden(&frame, "SAVED", "");
    let e = ok(f.prepared().controller().evidence());
    assert!(
        e.io_events
            .iter()
            .any(|v| matches!(v.point, store::IoPoint::JournalRename)
                && matches!(v.outcome, store::IoOutcome::Completed))
    );
    assert!(!ok(f.dc.evidence()).volatile_backend);
    app_chain_artifacts(
        &f,
        101,
        NEW,
        object(&f, 101).operations[0].allocation.operation_id,
        &ok(f.observe(&app, 3)).actor,
        endpoint.handle,
    );
    drop((app, endpoint));
    f.reopen();
    let p = f.launch(0, 101, 4);
    assert_eq!(selected(&mut f, &p, 603, 4), (2, NEW.to_vec()));
    drop(p);
    f.finish();
}

#[test]
fn ui1_1_revoke_paused_key_frame_and_save() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_revoke_paused_key_frame_and_save",
    ));
    for point in [
        desktop::NativeSavePausePoint::BeforeKeyDelivery,
        desktop::NativeSavePausePoint::BeforePresentFreeze,
        desktop::NativeSavePausePoint::BeforeFramePublish,
    ] {
        let mut f = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_revoke_paused_key_frame_and_save",
                "paused_retirement",
                desktop_pause_leg(point),
                recording::ScenarioScope::Composition,
            ),
            2,
        );
        let mut app = f.app(0, 101, 1);
        let old = ok(f.observe(&app, 1));
        let before = files(&f.path);
        let p = desktop_pause(&mut f, 0, point);
        let dc = f.dc.clone();
        let release = p.clone();
        if matches!(point, desktop::NativeSavePausePoint::BeforeKeyDelivery) {
            f.drivers[0].send(&f.h, 4, 1, 2);
        }
        let caller = OwnedCaller::start(
            move || {
                let r = if matches!(point, desktop::NativeSavePausePoint::BeforeKeyDelivery) {
                    app.step(2).map(|_| ())
                } else {
                    app.render(2).map(|_| ())
                };
                (app, r)
            },
            move || {
                let _ = dc.release_native_save(&release);
            },
        );
        let entered = ok(p.wait_until_entered(1000));
        assert!(entered.entered && !entered.settled);
        let published_before = ok(f.dc.evidence())
            .exchanges
            .iter()
            .filter(|(_, r)| {
                r.protocol == 833 && r.msg_type == 6 && volatile::u32_at(&r.payload, 16) == 0
            })
            .count();
        revoke(&f, 0, &old.actor, 3);
        ok(f.dc.release_native_save(&p));
        let (app, r) = caller.join(&f);
        assert!(matches!(r, Err(NativeEditorError::Desktop(Status::Stale))));
        assert!(ok(p.wait_until_settled(1000)).settled);
        let published_after = ok(f.dc.evidence())
            .exchanges
            .iter()
            .filter(|(_, r)| {
                r.protocol == 833 && r.msg_type == 6 && volatile::u32_at(&r.payload, 16) == 0
            })
            .count();
        assert_eq!(
            published_after, published_before,
            "no late successful actual Present after retirement"
        );
        assert_eq!(files(&f.path), before);
        assert_eq!(f.counts(101), (0, 0, 0));
        let retired_frame = ok(f.compose(0, 3));
        f.protected_golden(&retired_frame, "UNAVAILABLE", "");
        assert!(volatile::crop(&retired_frame).iter().all(|b| *b == 255));
        let mut other = f.app(1, 102, 3);
        f.stroke(1, &mut other, 4, 3);
        f.composed(1, &mut other, 3);
        drop((other, app));
        f.finish();
    }
    for expired in [false, true] {
        let mut f = Fixture::recorded_ttl(
            &recording,
            scenario(
                "ui1_1_revoke_paused_key_frame_and_save",
                "prepermit_retirement",
                u32::from(expired),
                recording::ScenarioScope::DirectNative,
            ),
            2,
            30000,
            20,
        );
        let raw = raw_editor(&mut f, 0, 101, 1);
        raw_replace(&mut f, 0, &raw, NEW, 2);
        let a = attempt(&mut f, 0, &raw, 700, 2);
        let writer = ok(a.source.write_lease());
        let p = store_pause(&mut f, &raw, store::PausePoint::BeforeCommitPermit);
        let execution = ok(f.host().start_native_save(&a.commit));
        let entered = ok(p.wait_until_entered(1000));
        assert!(entered.entered && !entered.permit_issued);
        if expired {
            ok(f.prepared().controller().advance_time(30));
            let _ = f.compose(0, 30);
        } else {
            revoke(&f, 0, &ok(f.editor_observation(&raw.editor, 2)).actor, 3);
        }
        let reply = pending_reply(&f, &execution);
        let r = ok(artifact::CommitReply::decode(reply.wire()));
        assert_eq!(r.status, StoreStatus::Stale as u32);
        assert_eq!(
            (r.operation_id, r.revision, r.receipt_shm, r.receipt_len),
            (0, 0, 0, 0)
        );
        ok(f.prepared().controller().release(&p));
        assert!(ok(p.wait_until_settled(1000)).settled);
        join_attempt(&mut f, &a, &reply);
        assert_eq!(f.counts(101), (1, 0, 0));
        assert_eq!(
            object(&f, 101).selected.content_hash,
            <sha2::Sha256 as sha2::Digest>::digest(OLD).as_slice()
        );
        assert_eq!(selected_blob(&f, 101), OLD);
        f.denied_at(
            "source_write_retired",
            "NativeSourceWrite.copy_from",
            writer.copy_from(64, NEW),
            StoreStatus::Stale,
        );
        drop((writer, reply, execution, a, raw));
        f.finish();
    }
}

#[test]
fn ui1_1_fault_restart_discards_unsaved_draft_and_old_grants() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_fault_restart_discards_unsaved_draft_and_old_grants",
    ));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_fault_restart_discards_unsaved_draft_and_old_grants",
            "primary",
            0,
            recording::ScenarioScope::Composition,
        ),
        1,
    );
    let mut app = f.app(0, 101, 1);
    f.replace(0, &mut app, NEW, 2);
    let old = ok(f.observe(&app, 2));
    ok(f.dc.fault_native_editor(old.actor.instance_id, 3));
    assert!(matches!(
        f.observe(&app, 3),
        Err(NativeEditorError::Desktop(Status::Stale))
    ));
    let unavailable = ok(f.compose(0, 3));
    assert!(volatile::crop(&unavailable).iter().all(|v| *v == 255));
    f.protected_golden(&unavailable, "UNAVAILABLE", "");
    assert_eq!(f.counts(101), (0, 0, 0));
    f.unchanged();
    drop(app);
    // Fresh explicit route: the recovery chord must release R before fresh Enter.
    f.drivers[0].send(&f.h, 224, 1, 4);
    f.drivers[0].send(&f.h, 21, 1, 4);
    f.drivers[0].send(&f.h, 224, 2, 4);
    f.drivers[0].send(&f.h, 40, 1, 4);
    f.denied_at(
        "control_main_013",
        "take_save_preview_action",
        f.h.take_save_preview_action(&f.drivers[0].s.chrome.peer),
        Status::NotReady,
    );
    f.drivers[0].send(&f.h, 40, 2, 4);
    f.drivers[0].send(&f.h, 21, 2, 4);
    let mut fresh = f.app(0, 101, 5);
    let o = ok(f.observe(&fresh, 5));
    assert_eq!(o.bytes, OLD);
    assert_ne!(o.actor.instance_id, old.actor.instance_id);
    assert!(o.actor.instance_generation > 0);
    let frame = f.composed(0, &mut fresh, 5);
    f.protected_golden(&frame, "LOADED", "");
    drop(fresh);
    f.finish();
}

#[test]
fn ui1_1_focus_compositor_restart_retires_old_epochs() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_focus_compositor_restart_retires_old_epochs",
    ));
    for service in [
        desktop::ServiceKind::Focus,
        desktop::ServiceKind::Compositor,
    ] {
        let mut f = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_focus_compositor_restart_retires_old_epochs",
                "primary",
                service_leg(service),
                recording::ScenarioScope::Composition,
            ),
            1,
        );
        let raw = raw_editor(&mut f, 0, 101, 1);
        let before = ok(f.h.render_native_editor(&raw.editor, 1));
        ok(f.compose(0, 1));
        let endpoint = raw.launch.desktop().bindings().surface().clone();
        ok(f.dc.restart_service(service, 2));
        f.denied_at(
            "control_main_014",
            "render_native_editor",
            f.h.render_native_editor(&raw.editor, 2),
            Status::Stale,
        );
        let denied = f.h.begin_save_read(
            raw.launch.desktop().bindings().artifact_origin(),
            &selected_request(&raw.launch, 800),
            2,
        );
        f.denied_at(
            "service_restart_read_retired",
            "begin_save_read",
            denied,
            Status::Stale,
        );
        volatile::deny(
            &f.h,
            &f.drivers[0].s.input,
            f.drivers[0].request(40, 1, 0),
            8,
            3,
            Status::Stale,
        );
        drop(raw);
        f.reattach_input(0, 3);
        let mut app = f.app(0, 101, 3);
        let fresh = ok(app.render(3));
        ok(f.compose(0, 3));
        assert_ne!(fresh.surface_id, before.surface_id);
        assert!(fresh.focus_epoch > 0);
        let bad = ok(surface::Acquire {
            request_id: 801,
            surface_id: before.surface_id,
            surface_generation: before.surface_generation,
            buffer_index: 0,
            reserved: 0,
        }
        .encode(endpoint.handle));
        volatile::zero_reply(&f.h.dispatch(&endpoint.peer, &bad, 3), 24, Status::Stale);
        assert_eq!(ok(f.observe(&app, 3)).bytes, OLD);
        drop(app);
        f.finish();
    }
}

#[test]
fn ui1_1_stalled_session_leaves_other_session_usable() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_stalled_session_leaves_other_session_usable",
    ));
    for point in [
        store::PausePoint::BeforeCommitPermit,
        store::PausePoint::AfterCommitPermit,
    ] {
        let mut f = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_stalled_session_leaves_other_session_usable",
                "primary",
                commit_pause_leg(point),
                recording::ScenarioScope::DirectNative,
            ),
            2,
        );
        let raw = raw_editor(&mut f, 0, 101, 1);
        raw_replace(&mut f, 0, &raw, NEW, 2);
        let a = attempt(&mut f, 0, &raw, 810, 2);
        let p = store_pause(&mut f, &raw, point);
        let execution = ok(f.host().start_native_save(&a.commit));
        let entered = ok(p.wait_until_entered(1000));
        assert_eq!(
            entered.permit_issued,
            matches!(point, store::PausePoint::AfterCommitPermit)
        );
        let begun = Instant::now();
        let mut b = f.app(1, 102, 3);
        f.stroke(1, &mut b, 4, 3);
        f.composed(1, &mut b, 3);
        assert!(begun.elapsed() < Duration::from_millis(1000));
        assert!(ok(f.observe(&b, 3)).bytes.ends_with(b"a"));
        assert_eq!(object(&f, 102).selected.revision, 1);
        let reply = pending_reply(&f, &execution);
        let r = ok(artifact::CommitReply::decode(reply.wire()));
        assert_eq!(
            r.status,
            if entered.permit_issued {
                StoreStatus::Unknown
            } else {
                StoreStatus::Timeout
            } as u32
        );
        assert_eq!(
            r.operation_id,
            if entered.permit_issued {
                a.operation
            } else {
                0
            }
        );
        assert!(f.producer_counts("producer_checkpoint").held > 0);
        let mut unfinished = 0;
        for id in reply.producers() {
            match f.host().join_native_save_finished(id) {
                Err(StoreStatus::NotReady) => {
                    unfinished += 1;
                    f.record(|r| {
                        r.unit_control(
                            "paused_join_not_ready",
                            "join_native_save_finished",
                            "Err",
                            "StoreStatus",
                            StoreStatus::NotReady as u32,
                            None,
                        )
                    });
                }
                Ok(proof) => {
                    assert!(matches!(proof.outcome(), desktop::JoinOutcome::Returned));
                    assert!(f.joined.insert(proof.producer_id()));
                    let actual = ok(f.h.native_join_observation(&proof));
                    f.record(|r| r.native_join(&actual));
                }
                Err(_) => panic!("own real producer join classification"),
            }
        }
        assert!(unfinished > 0);
        let old_io = ok(f.prepared().controller().evidence())
            .io_events
            .into_iter()
            .filter(|v| v.operation_id == a.operation)
            .map(|v| (v.sequence, v.writer_id))
            .collect::<Vec<_>>();
        if !entered.permit_issued {
            // Definitive prepermit closure leaves this SAME live instance reusable even while old dispatcher stays paused.
            let retry = attempt(&mut f, 0, &raw, 813, 3);
            let fresh = settle(&mut f, &retry);
            assert_ne!(retry.operation, a.operation);
            assert_eq!(receipt(&fresh, retry.operation).0.result_revision, 2);
            assert_eq!(f.counts(101), (2, 1, 1));
            drop((fresh, retry));
        }
        let before_release = f.counts(101);
        ok(f.prepared().controller().release(&p));
        assert!(ok(p.wait_until_settled(1000)).settled);
        for id in a.commit_entry.producer_ids() {
            join_core(&mut f, id)
        }
        f.join_all();
        if entered.permit_issued {
            assert_eq!(f.counts(101), (1, 1, 1));
            let r = observe_commit(&mut f, &raw, &a, 812, 3);
            assert_eq!(
                original_receipt(&r).0.outcome,
                EditorReceiptOutcomeV0::Committed
            );
        } else {
            assert_eq!(f.counts(101), before_release);
            no_late_io(&f, a.operation, &old_io);
        }
        drop((b, reply, execution, a, raw));
        f.finish();
    }
}

#[test]
fn ui1_1_store_commit_reopen_preserves_prior_blob() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_store_commit_reopen_preserves_prior_blob",
    ));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_store_commit_reopen_preserves_prior_blob",
            "primary",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let prior = selected_blob(&f, 101);
    assert_eq!(prior, OLD);
    let old_epoch = object(&f, 101).service_epoch;
    let (mut app, endpoint) = f.app_with_endpoint(0, 101, 1);
    let actor = ok(f.observe(&app, 1)).actor;
    f.replace(0, &mut app, NEW, 2);
    f.save(0, &mut app, 3);
    let frame = f.composed(0, &mut app, 3);
    f.protected_golden(&frame, "SAVED", "");
    assert_eq!(f.counts(101), (1, 1, 1));
    let committed = object(&f, 101);
    let record = &committed.operations[0];
    let receipt = record.receipt.as_ref().unwrap();
    assert_eq!(receipt.backend_service_epoch, old_epoch);
    assert_eq!(receipt.original_instance_id, actor.instance_id);
    assert_eq!(receipt.result_revision, 2);
    app_chain_artifacts(&f, 101, NEW, receipt.operation_id, &actor, endpoint.handle);
    let ev = ok(f.prepared().controller().evidence());
    for point in [
        store::IoPoint::CandidateBlobSync,
        store::IoPoint::CandidateReadback,
        store::IoPoint::JournalRename,
        store::IoPoint::JournalDirectorySync,
        store::IoPoint::JournalReadback,
    ] {
        assert!(
            ev.io_events
                .iter()
                .any(|e| e.operation_id == receipt.operation_id
                    && e.point == point
                    && e.outcome == store::IoOutcome::Completed)
        );
    }
    let old_blob = files(&f.path)
        .into_iter()
        .find(|(p, b)| p.extension().is_some_and(|v| v == "blob") && b == OLD)
        .unwrap();
    drop((app, endpoint));
    let token = fence(&mut f);
    let snap = token.fence_snapshot();
    assert_eq!(snap.original_tickets.len(), 2);
    assert!(
        snap.original_tickets
            .iter()
            .all(|v| v.completion == store::OriginalTicketCompletion::JoinedReturned)
    );
    let reopened = ok(store::StoreFixture::reopen_native_save(token));
    f.replace_prepared(reopened);
    let actual = object(&f, 101);
    assert_eq!(actual.selected.revision, 2);
    assert!(actual.service_epoch > old_epoch);
    assert_eq!(
        actual.reopen_witness.as_ref().unwrap().prior_service_epoch,
        old_epoch
    );
    assert_eq!(files(&f.path).get(&old_blob.0), Some(&prior));
    let p = f.launch(0, 101, 4);
    assert_eq!(selected(&mut f, &p, 1400, 4), (2, NEW.to_vec()));
    drop(p);
    f.finish();
}

#[test]
fn ui1_1_store_same_base_conflict_and_operation_reuse() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_store_same_base_conflict_and_operation_reuse",
    ));
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_store_same_base_conflict_and_operation_reuse",
            "primary",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let first = raw_editor(&mut f, 0, 101, 1);
    raw_replace(&mut f, 0, &first, NEW, 2);
    let a = attempt(&mut f, 0, &first, 1500, 2);
    let second = raw_editor(&mut f, 0, 101, 2);
    raw_replace(&mut f, 0, &second, b"note=other\n", 2);
    let b = attempt(&mut f, 0, &second, 1510, 2);
    assert_eq!(
        a.source.bound().binding().allocation.expected_revision,
        b.source.bound().binding().allocation.expected_revision
    );
    let pause = store_pause(&mut f, &first, store::PausePoint::BeforeCommitPermit);
    let running = ok(f.host().start_native_save(&a.commit));
    assert!(!ok(pause.wait_until_entered(1000)).permit_issued);
    f.denied_at(
        "control_main_015",
        "start_native_save",
        f.host().start_native_save(&a.commit),
        StoreStatus::NotReady,
    );
    let blocked = ok(f.host().execute_native_save(&b.commit));
    commit_status(&blocked, StoreStatus::NotReady, 0);
    for id in blocked.producers() {
        join_core(&mut f, id)
    }
    ok(f.prepared().controller().release(&pause));
    let winner = pending_reply(&f, &running);
    assert_eq!(
        receipt(&winner, a.operation).0.outcome,
        EditorReceiptOutcomeV0::Committed
    );
    join_attempt(&mut f, &a, &winner);
    let before = f.counts(101);
    let baseline = files(&f.path);
    // A fresh control refers to the already bound second original; it does not replay the claimed execution.
    let entry = ok(f.h.begin_save_commit(
        second.launch.desktop().bindings().artifact_origin(),
        b.source.bound(),
        &b.request,
        3,
    ));
    let registered = ok(f
        .host()
        .register_native_save(second.launch.endpoint(), ok(entry.wait_once())));
    let loser = ok(f.host().execute_native_save(&registered));
    let reply = ok(artifact::CommitReply::decode(loser.wire()));
    assert_eq!(
        (reply.status, reply.revision, reply.operation_id),
        (StoreStatus::Conflict as u32, 2, 0)
    );
    assert_eq!((reply.receipt_shm, reply.receipt_len), (0, 0));
    for id in entry.producer_ids() {
        join_core(&mut f, id)
    }
    for id in loser.producers() {
        join_core(&mut f, id)
    }
    assert_eq!(f.counts(101), (before.0 + 1, 1, 1));
    assert_eq!(files(&f.path), baseline);
    for index in 0..5 {
        let mut q = a.request;
        match index {
            0 => q.payload[24..32].copy_from_slice(&9u64.to_le_bytes()),
            1 => q.payload[32..40].copy_from_slice(&(a.operation + 1).to_le_bytes()),
            2 => q.payload[40..48]
                .copy_from_slice(&(a.source.descriptor().handle.pack() + 1).to_le_bytes()),
            3 => q.payload[48..52].copy_from_slice(&1u32.to_le_bytes()),
            4 => q.payload[8..16].copy_from_slice(&2u64.to_le_bytes()),
            _ => unreachable!(),
        };
        f.denied_at(
            "control_main_016",
            "begin_save_commit",
            f.h.begin_save_commit(
                first.launch.desktop().bindings().artifact_origin(),
                a.source.bound(),
                &q,
                3,
            ),
            Status::Denied,
        );
        assert_eq!(files(&f.path), baseline);
    }
    let duplicate = ok(f.h.begin_save_commit(
        first.launch.desktop().bindings().artifact_origin(),
        a.source.bound(),
        &a.request,
        3,
    ));
    let reg = ok(f
        .host()
        .register_native_save(first.launch.endpoint(), ok(duplicate.wait_once())));
    let r = ok(f.host().execute_native_save(&reg));
    assert_eq!(receipt(&r, a.operation).1, receipt(&winner, a.operation).1);
    for id in duplicate.producer_ids() {
        join_core(&mut f, id)
    }
    for id in r.producers() {
        join_core(&mut f, id)
    }
    assert_eq!(f.counts(101), (before.0 + 1, 1, 1));
    assert_eq!(files(&f.path), baseline);
    let mut wrong = ok(artifact::SaveStatus {
        request_id: 1520,
        session_id: boot(&first.launch).session_id,
        session_generation: boot(&first.launch).session_generation,
        operation_id: b.operation,
    }
    .encode(first.launch.endpoint().handle));
    let denied =
        f.h.begin_save_recovery(a.commit.original().observation_origin(), &wrong, 3);
    f.denied_at(
        "control_main_017",
        "held_rust_result",
        denied,
        Status::Denied,
    );
    wrong.msg_type = 1;
    f.denied_at(
        "control_main_018",
        "begin_save_recovery",
        f.h.begin_save_recovery(a.commit.original().observation_origin(), &wrong, 3),
        Status::Unsupported,
    );
    assert_eq!(
        original_receipt(&observe_commit(&mut f, &first, &a, 1521, 3))
            .0
            .operation_id,
        a.operation
    );
    drop((
        r, reg, duplicate, loser, registered, entry, blocked, winner, running, b, a, first, second,
    ));
    let fresh = raw_editor(&mut f, 0, 101, 4);
    raw_replace(&mut f, 0, &fresh, b"note=retry\n", 4);
    let retry = attempt(&mut f, 0, &fresh, 1530, 4);
    assert_eq!(
        retry.source.bound().binding().allocation.expected_revision,
        2
    );
    let r = settle(&mut f, &retry);
    assert_eq!(receipt(&r, retry.operation).0.result_revision, 3);
    drop((r, retry, fresh));
    f.finish();
}

#[test]
fn ui1_1_store_lost_reply_reconciles_without_mutation_replay() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_store_lost_reply_reconciles_without_mutation_replay",
    ));
    for before_journal in [true, false] {
        let mut f = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_store_lost_reply_reconciles_without_mutation_replay",
                "allocate_loss",
                u32::from(before_journal),
                recording::ScenarioScope::DirectNative,
            ),
            1,
        );
        let raw = raw_editor(&mut f, 0, 101, 1);
        raw_replace(&mut f, 0, &raw, NEW, 2);
        let allocation = allocate_start(&mut f, 0, &raw, 1600, 2);
        ok(f.prepared()
            .controller()
            .drop_next_native_reply(raw.launch.endpoint()));
        let pause = if before_journal {
            Some(store_pause(
                &mut f,
                &raw,
                store::PausePoint::BeforeAllocationJournal,
            ))
        } else {
            None
        };
        let running = ok(f.host().start_native_save(&allocation.registered));
        if let Some(p) = &pause {
            assert!(!ok(p.wait_until_entered(1000)).permit_issued);
        }
        let reply = pending_reply(&f, &running);
        let r = ok(artifact::AllocateSaveIdReply::decode(reply.wire()));
        assert_eq!(r.operation_id, 0);
        assert_eq!(r.status, StoreStatus::Disconnected as u32);
        let rendered = ok(f.h.render_native_editor(&raw.editor, 2));
        let frame = ok(f.compose(0, 2));
        assert_eq!(rendered.sequence, frame.sequence);
        unknown(&f, 0, None);
        f.denied_at(
            "control_main_019",
            "save_chrome_record",
            f.dc.save_chrome_record(&f.drivers[0].s.compositor),
            Status::NotReady,
        );
        f.protected_golden(&frame, "UNSAVED", "ALLOCATION UNKNOWN");
        if let Some(p) = &pause {
            ok(f.prepared().controller().release(p));
            assert!(ok(p.wait_until_settled(1000)).settled);
        }
        for id in allocation.entry.producer_ids() {
            join_core(&mut f, id)
        }
        for id in reply.producers() {
            join_core(&mut f, id)
        }
        // Recover only the genuine original Allocate9/10. It never emits Commit5.
        let mut observed = observe_allocate(&mut f, &allocation, 1601, 3);
        let wire = ok(artifact::ObserveAllocationReply::decode(observed.wire()));
        assert_eq!(wire.request_id, 1601);
        assert_eq!(wire.reserved, 0);
        if before_journal {
            assert!(matches!(
                observed.observation(),
                store::NativeOriginalObservation::NoAllocation
            ));
            assert!(observed.allocation().is_none());
            assert_eq!(wire.operation_id, 0);
        } else {
            assert!(matches!(
                observed.observation(),
                store::NativeOriginalObservation::AllocationRetained {
                    commit_dispatched: false,
                    ..
                }
            ));
            assert!(wire.operation_id > 0);
            let retained_allocation = observed.allocation().unwrap().observation();
            assert_eq!(retained_allocation.operation_id, wire.operation_id);
            let deadline = allocation.intent.deadline();
            while Instant::now() <= deadline {
                thread::sleep(Duration::from_millis(1));
            }
            if let Some(actual) = observed.take_allocation() {
                f.denied_at(
                    "control_main_020",
                    "source_for_intent",
                    f.host()
                        .source_for_intent(raw.launch.endpoint(), &actual, &allocation.intent),
                    StoreStatus::Stale,
                );
            }
            let retained = object(&f, 101)
                .operations
                .into_iter()
                .find(|r| r.allocation.operation_id == wire.operation_id)
                .unwrap();
            assert_eq!(retained.allocation, retained_allocation);
            assert_eq!(retained.state, EditorOperationStateV0::Noncommit);
            assert_eq!(retained.closure, EditorClosureV0::Deadline);
            assert!(
                retained.binding.is_none()
                    && retained.permit.is_none()
                    && retained.receipt.is_none()
            );
            assert_eq!(f.counts(101), (0, 0, 0));
        }
        assert!(
            ok(f.prepared().controller().evidence())
                .exchanges
                .iter()
                .all(|x| ok(desktop::decode_envelope_wire(&x.request_wire)).msg_type != 5)
        );
        drop((observed, reply, running, allocation, raw));
        f.finish();
    }
    for arm in 0..4 {
        let mut f = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_store_lost_reply_reconciles_without_mutation_replay",
                "commit_loss",
                arm,
                recording::ScenarioScope::DirectNative,
            ),
            2,
        );
        let raw = raw_editor(&mut f, 0, 101, 1);
        raw_replace(&mut f, 0, &raw, NEW, 2);
        let a = attempt(&mut f, 0, &raw, 1620, 2);
        ok(f.prepared()
            .controller()
            .drop_next_native_reply(raw.launch.endpoint()));
        let point = if arm == 0 {
            store::PausePoint::BeforeCommitPermit
        } else {
            store::PausePoint::AfterCommitPermit
        };
        let pause = store_pause(&mut f, &raw, point);
        let execution = ok(f.host().start_native_save(&a.commit));
        let entered = ok(pause.wait_until_entered(1000));
        assert_eq!(entered.permit_issued, arm != 0);
        if arm == 2 {
            revoke(&f, 0, &a.intent.snapshot().view().actor, 3);
        }
        if arm == 3 {
            ok(f.prepared().controller().advance_time(600002));
        }
        let reply = pending_reply(&f, &execution);
        commit_status(&reply, StoreStatus::Disconnected, 0);
        let before = f.counts(101);
        let rows = ok(f.prepared().controller().evidence()).exchanges.len();
        if arm != 1 {
            if arm == 0 {
                let rendered = ok(f.h.render_native_editor(&raw.editor, 3));
                let composed = ok(f.compose(0, 3));
                assert_eq!(rendered.sequence, composed.sequence);
            } else {
                // Retired attachments publish blank Unavailable without a live render.
                ok(f.compose(0, 3));
            }
            unknown(&f, 0, Some(a.operation));
        }
        if arm == 1 {
            raw_stroke(&mut f, 0, &raw, 4, 3);
            let frame = ok(f.h.render_native_editor(&raw.editor, 3));
            let composed = ok(f.compose(0, 3));
            assert_eq!(frame.sequence, composed.sequence);
            f.protected_golden(&composed, "UNSAVED", "SAVE UNKNOWN");
            unknown(&f, 0, Some(a.operation));
            assert!(
                ok(f.editor_observation(&raw.editor, 3))
                    .bytes
                    .ends_with(b"a")
            );
            raw_chord(&mut f, 0, &raw, 22, 3);
            f.denied_at(
                "control_main_021",
                "take_save_intent",
                f.h.take_save_intent(&raw.editor),
                Status::NotReady,
            );
            assert_eq!(f.counts(101), before);
        }
        let pending = if arm == 0 {
            // No receipt is inferred from timeout while the unpermitted producer is still held.
            None
        } else {
            let pending = observe_commit(&mut f, &raw, &a, 1622, 3);
            assert!(matches!(
                pending.observation(),
                store::NativeOriginalObservation::CommitPending { .. }
            ));
            assert_eq!(
                ok(artifact::SaveStatusReply::decode(pending.wire())).operation_id,
                a.operation
            );
            Some(pending)
        };
        assert_eq!(f.counts(101), before);
        assert!(ok(f.prepared().controller().evidence()).exchanges.len() >= rows);
        let mut b = f.app(1, 102, 3);
        f.stroke(1, &mut b, 4, 3);
        f.composed(1, &mut b, 3);
        ok(f.prepared().controller().release(&pause));
        assert!(ok(pause.wait_until_settled(1000)).settled);
        join_attempt(&mut f, &a, &reply);
        let resolved = observe_commit(&mut f, &raw, &a, 1623, 3);
        let record = original_receipt(&resolved).0;
        assert_eq!(record.operation_id, a.operation);
        assert_eq!(
            record.backend_service_epoch,
            object(&f, 101)
                .operations
                .iter()
                .find(|r| r.allocation.operation_id == a.operation)
                .unwrap()
                .submitted_service_epoch
                .unwrap()
        );
        if arm == 0 {
            assert_eq!(record.outcome, EditorReceiptOutcomeV0::DefinitiveNoncommit);
            assert_eq!(f.counts(101), (1, 0, 0));
        } else {
            assert_eq!(record.outcome, EditorReceiptOutcomeV0::Committed);
            assert_eq!(f.counts(101), (1, 1, 1));
        }
        assert_eq!(
            ok(f.prepared().controller().evidence())
                .exchanges
                .iter()
                .filter(|r| ok(desktop::decode_envelope_wire(&r.request_wire)).msg_type == 5)
                .count(),
            1
        );
        if arm == 1 {
            let frame = ok(f.h.render_native_editor(&raw.editor, 3));
            let composed = ok(f.compose(0, 3));
            assert_eq!(frame.sequence, composed.sequence);
            f.protected_golden(&composed, "UNSAVED", "");
        }
        drop((resolved, pending, b, reply, execution, a, raw));
        f.finish();
    }
    let mut f = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_store_lost_reply_reconciles_without_mutation_replay",
            "app_commit_loss",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let raw = raw_editor(&mut f, 0, 101, 1);
    raw_replace(&mut f, 0, &raw, NEW, 2);
    let a = attempt(&mut f, 0, &raw, 1640, 2);
    let pause = store_pause(&mut f, &raw, store::PausePoint::BeforeSelectionRename);
    let execution = ok(f.host().start_native_save(&a.commit));
    assert!(ok(pause.wait_until_entered(1000)).permit_issued);
    f.store_snapshot("permitted_pre_selection", "source_checkpoint", None, None);
    ok(f.prepared()
        .controller()
        .fail_next_io(raw.launch.endpoint(), store::IoPoint::JournalDirectorySync));
    ok(f.prepared().controller().release(&pause));
    let reply = pending_reply(&f, &execution);
    assert!(ok(pause.wait_until_settled(1000)).settled);
    join_attempt(&mut f, &a, &reply);
    let r = ok(artifact::CommitReply::decode(reply.wire()));
    assert_eq!(r.status, StoreStatus::Unknown as u32);
    assert_eq!(r.operation_id, a.operation);
    assert!(
        ok(f.prepared().controller().evidence())
            .io_events
            .iter()
            .any(|e| e.operation_id == a.operation
                && e.writer_id > 0
                && e.point == store::IoPoint::JournalDirectorySync
                && e.outcome == store::IoOutcome::SimulatedFailure)
    );
    // A joined permit does not turn a failed installed-journal read into a receipt.
    let before_readback = files(&f.path);
    let captures_before = capture_signature(&ok(f.prepared().controller().native_save_artifacts()));
    ok(f.prepared()
        .controller()
        .fail_next_io(raw.launch.endpoint(), store::IoPoint::JournalReadback));
    let unresolved = observe_commit(&mut f, &raw, &a, 1641, 3);
    assert!(matches!(
        unresolved.observation(),
        store::NativeOriginalObservation::CommitPending { .. }
    ));
    let unresolved_wire = ok(artifact::SaveStatusReply::decode(unresolved.wire()));
    assert_eq!(unresolved_wire.status, StoreStatus::Unknown as u32);
    assert_eq!(unresolved_wire.operation_id, a.operation);
    assert_eq!(
        (unresolved_wire.receipt_shm, unresolved_wire.receipt_len),
        (0, 0)
    );
    assert_eq!(object(&f, 101).selected.revision, 1);
    assert_eq!(f.counts(101), (1, 1, 1));
    assert_eq!(files(&f.path), before_readback);
    assert_eq!(
        capture_signature(&ok(f.prepared().controller().native_save_artifacts())),
        captures_before
    );
    assert!(
        ok(f.prepared().controller().evidence())
            .io_events
            .iter()
            .any(|e| e.operation_id == a.operation
                && e.point == store::IoPoint::JournalReadback
                && e.outcome == store::IoOutcome::SimulatedFailure)
    );
    drop(unresolved);
    let recovered = observe_commit(&mut f, &raw, &a, 1642, 3);
    assert_eq!(
        original_receipt(&recovered).0.outcome,
        EditorReceiptOutcomeV0::Committed
    );
    assert_eq!(object(&f, 101).selected.revision, 2);
    assert_eq!(f.counts(101), (1, 1, 1));
    drop((recovered, reply, execution, pause, a, raw));
    f.finish();
}

#[test]
fn ui1_1_store_recovery_validates_atomic_selection_and_receipt() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_store_recovery_validates_atomic_selection_and_receipt",
    ));
    for (index, point) in [
        store::PausePoint::BeforeCandidateWrite,
        store::PausePoint::AfterCandidatePublication,
        store::PausePoint::AfterSelectionRename,
        store::PausePoint::AfterAcknowledgement,
    ]
    .into_iter()
    .enumerate()
    {
        let mut f = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_store_recovery_validates_atomic_selection_and_receipt",
                "crash_phase",
                index as u32,
                recording::ScenarioScope::DirectNative,
            ),
            1,
        );
        let raw = raw_editor(&mut f, 0, 101, 1);
        raw_replace(&mut f, 0, &raw, NEW, 2);
        let a = attempt(&mut f, 0, &raw, 1700, 2);
        let old_epoch = object(&f, 101).service_epoch;
        let recovery = ok(f
            .prepared()
            .controller()
            .recovery_for_original(&f.drivers[0].s.chrome.peer, a.commit.original()));
        let predecessor = if index == 2 {
            Some(store_pause(
                &mut f,
                &raw,
                store::PausePoint::BeforeSelectionRename,
            ))
        } else {
            None
        };
        let pause = store_pause(&mut f, &raw, point);
        if predecessor.is_some() {
            let baseline = ok(f.prepared().controller().evidence());
            let files_before = files(&f.path);
            let held = f.producer_counts("producer_checkpoint");
            for duplicate in [
                store::PausePoint::BeforeSelectionRename,
                store::PausePoint::AfterSelectionRename,
                store::PausePoint::BeforeCandidateWrite,
            ] {
                let error = match f
                    .prepared()
                    .controller()
                    .pause_native_save(raw.launch.endpoint(), duplicate)
                {
                    Err(error) => error,
                    Ok(_) => panic!("native pause pair admitted duplicate or third member"),
                };
                assert_eq!(error.status, StoreStatus::NotReady);
                f.record(|r| {
                    r.unit_control(
                        "pause_pair_denial",
                        "pause_native_save",
                        "Err",
                        "StoreStatus",
                        error.status as u32,
                        None,
                    )
                });
                let after = ok(f.prepared().controller().evidence());
                assert_eq!(
                    serde_json::to_vec(&after.barriers).unwrap(),
                    serde_json::to_vec(&baseline.barriers).unwrap()
                );
                assert_eq!(after.io_events.len(), baseline.io_events.len());
                assert_eq!(
                    recording::dto_store_ledger(&after.ledger),
                    recording::dto_store_ledger(&baseline.ledger)
                );
                assert_eq!(files(&f.path), files_before);
                let after = f.producer_counts("producer_checkpoint");
                assert_eq!(
                    (after.held, after.io, after.joining, after.joined_total),
                    (held.held, held.io, held.joining, held.joined_total)
                );
            }
        }
        let execution = ok(f.host().start_native_save(&a.commit));
        let predecessor_identity = predecessor.as_ref().map(|before| {
            let actual = ok(before.wait_until_entered(1000));
            assert!(actual.permit_issued);
            assert_eq!(actual.operation_id, a.operation);
            f.store_snapshot("permitted_pre_selection", "source_checkpoint", None, None);
            ok(f.prepared().controller().release(before));
            (actual.request_id, actual.operation_id)
        });
        let entered = ok(pause.wait_until_entered(1000));
        assert!(entered.permit_issued);
        if let Some(actual) = predecessor_identity {
            assert_eq!((entered.request_id, entered.operation_id), actual);
        }
        let failure = f.prepared().controller().quiesce_native_save(1);
        assert!(matches!(failure,Err(ref e)if e.status==StoreStatus::NotReady));
        assert_eq!(object(&f, 101).service_epoch, old_epoch);
        ok(f.prepared().controller().stop_at_phase(&pause));
        assert!(ok(pause.wait_until_settled(1000)).settled);
        let reply = pending_reply(&f, &execution);
        join_attempt(&mut f, &a, &reply);
        if let Some(before) = &predecessor {
            assert!(ok(before.wait_until_settled(1000)).settled);
            assert!(ok(pause.wait_until_settled(1000)).settled);
        }
        let token = fence(&mut f);
        let first = token.fence_snapshot();
        let before = files(&f.path);
        for _ in 0..2 {
            assert!(
                matches!(f.prepared().controller().quiesce_native_save(1),Err(ref e)if e.status==StoreStatus::NotReady)
            );
            assert_eq!(files(&f.path), before);
            assert_eq!(token.fence_snapshot().base.fence_id, first.base.fence_id);
        }
        let reopened = ok(store::StoreFixture::reopen_native_save(token));
        f.replace_prepared(reopened);
        let expected = if index < 2 { OLD } else { NEW };
        let revision = if index < 2 { 1 } else { 2 };
        let p = f.launch(0, 101, 3);
        assert_eq!(selected(&mut f, &p, 1702, 3), (revision, expected.to_vec()));
        let observed = recover_commit(&mut f, &raw, &a, &recovery, 1703, 3);
        let receipt = original_receipt(&observed).0;
        assert_eq!(
            receipt.outcome,
            if index < 2 {
                EditorReceiptOutcomeV0::DefinitiveNoncommit
            } else {
                EditorReceiptOutcomeV0::Committed
            }
        );
        assert_eq!(receipt.backend_service_epoch, old_epoch);
        assert_eq!(f.counts(101).0, 1);
        assert!(object(&f, 101).service_epoch > old_epoch);
        drop((observed, p, reply, execution, predecessor, pause, a, raw));
        f.finish();
    }
    for fault in [
        store::StorageFault::MissingJournal,
        store::StorageFault::TruncatedJournal,
        store::StorageFault::UnknownJournalVersion,
        store::StorageFault::ExtraJournalField,
        store::StorageFault::DuplicateJournalField,
        store::StorageFault::WrongJournalBinding,
        store::StorageFault::BrokenJournalChain,
        store::StorageFault::EarlierAllocatedOnlyJournal,
        store::StorageFault::CorruptBlob,
        store::StorageFault::UnsignedManifest,
        store::StorageFault::WrongSigningKey,
        store::StorageFault::UnboundPublicationIntent,
        store::StorageFault::UnsignedPublicationIntent,
        store::StorageFault::SymlinkEntry,
    ] {
        let mut f = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_store_recovery_validates_atomic_selection_and_receipt",
                "storage_fault",
                storage_fault_leg(fault),
                recording::ScenarioScope::DirectNative,
            ),
            1,
        );
        let raw = raw_editor(&mut f, 0, 101, 1);
        raw_replace(&mut f, 0, &raw, NEW, 2);
        let a = attempt(&mut f, 0, &raw, 1720, 2);
        if matches!(fault, store::StorageFault::EarlierAllocatedOnlyJournal) {
            let p = store_pause(&mut f, &raw, store::PausePoint::BeforeCommitPermit);
            let ex = ok(f.host().start_native_save(&a.commit));
            assert!(!ok(p.wait_until_entered(1000)).permit_issued);
            let r = pending_reply(&f, &ex);
            commit_status(&r, StoreStatus::Timeout, 0);
            ok(f.prepared().controller().release(&p));
            assert!(ok(p.wait_until_settled(1000)).settled);
            join_attempt(&mut f, &a, &r);
            assert!(object(&f, 101).operations[0].binding.is_some());
            drop((r, ex));
        } else {
            let r = settle(&mut f, &a);
            assert_eq!(
                receipt(&r, a.operation).0.outcome,
                EditorReceiptOutcomeV0::Committed
            );
            drop(r);
        }
        let mut token = fence(&mut f);
        let original = token.fence_snapshot();
        ok(store::StoreFixture::corrupt_native_fenced(
            &mut token, 101, fault,
        ));
        f.record(|r| r.storage_fault_applied(101, fault));
        f.store_snapshot(
            "after_storage_fault",
            "after_storage_fault",
            Some(&token.fence_snapshot()),
            None,
        );
        let failure = match store::StoreFixture::reopen_native_save(token) {
            Err(e) => e,
            Ok(_) => panic!("corrupt native namespace reopened"),
        };
        assert_eq!(failure.error.status, StoreStatus::Unknown);
        assert_eq!(
            failure.owner.fence_snapshot().base.fence_id,
            original.base.fence_id
        );
        assert_eq!(
            failure.owner.fence_snapshot().base.namespace_id,
            original.base.namespace_id
        );
        assert_eq!(failure.owner.fence_snapshot().original_tickets.len(), 2);
        assert_eq!(f.producer_counts("producer_checkpoint").held, 0);
        drop((a, raw));
        // The token remains exclusive and all writers are actually joined. The trusted symlink is never followed.
        f.finish_reopen_failure(failure);
    }
}

#[test]
fn ui1_1_unknown_save_survives_editor_recovery() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_unknown_save_survives_editor_recovery",
    ));
    for focus_fault in [false, true] {
        let mut f = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_unknown_save_survives_editor_recovery",
                "direct_unknown",
                u32::from(focus_fault),
                recording::ScenarioScope::DirectNative,
            ),
            3,
        );
        let raw = raw_editor(&mut f, 0, 101, 1);
        raw_replace(&mut f, 0, &raw, NEW, 2);
        let a = attempt(&mut f, 0, &raw, 1800, 2);
        let writer = ok(a.source.write_lease());
        let pause = store_pause(&mut f, &raw, store::PausePoint::AfterCommitPermit);
        let running = ok(f.host().start_native_save(&a.commit));
        assert!(ok(pause.wait_until_entered(1000)).permit_issued);
        let actor = a.intent.snapshot().view().actor;
        if focus_fault {
            ok(f.dc.restart_service(desktop::ServiceKind::Focus, 3));
        } else {
            ok(f.dc.fault_native_editor(actor.instance_id, 3));
        }
        let reply = pending_reply(&f, &running);
        commit_status(&reply, StoreStatus::Unknown, a.operation);
        ok(f.compose(0, 3));
        unknown(&f, 0, Some(a.operation));
        f.denied_at(
            "control_main_022",
            "render_native_editor",
            f.h.render_native_editor(&raw.editor, 3),
            Status::Stale,
        );
        f.denied_at(
            "source_write_retired",
            "NativeSourceWrite.copy_from",
            writer.copy_from(64, NEW),
            StoreStatus::Stale,
        );
        f.denied_at(
            "control_main_023",
            "begin_save_read",
            f.h.begin_save_read(
                raw.launch.desktop().bindings().artifact_origin(),
                &selected_request(&raw.launch, 1802),
                3,
            ),
            Status::Stale,
        );

        let mut wrong = ok(artifact::SaveStatus {
            request_id: 1803,
            session_id: actor.session_id,
            session_generation: actor.session_generation,
            operation_id: a.operation + 1,
        }
        .encode(raw.launch.endpoint().handle));
        f.denied_at(
            "control_main_024",
            "begin_save_recovery",
            f.h.begin_save_recovery(a.commit.original().observation_origin(), &wrong, 3),
            Status::Denied,
        );
        wrong.msg_type = 3;
        f.denied_at(
            "control_main_025",
            "begin_save_recovery",
            f.h.begin_save_recovery(a.commit.original().observation_origin(), &wrong, 3),
            Status::Unsupported,
        );
        // Fresh unrelated C in A's genuine session does not erase A's retained original banner.
        if focus_fault {
            f.reattach_input(0, 3);
        }
        let mut c = f.app(0, 103, 3);
        assert_eq!(ok(f.observe(&c, 3)).bytes, C);
        let frame = f.composed(0, &mut c, 3);
        f.protected_golden(&frame, "LOADED", "SAVE UNKNOWN");
        unknown(&f, 0, Some(a.operation));
        let recovery = ok(f
            .prepared()
            .controller()
            .recovery_for_original(&f.drivers[0].s.chrome.peer, a.commit.original()));
        assert!(
            matches!(f.prepared().controller().quiesce_native_save(1),Err(ref e)if e.status==StoreStatus::NotReady)
        );
        ok(f.prepared().controller().release(&pause));
        assert!(ok(pause.wait_until_settled(1000)).settled);
        join_attempt(&mut f, &a, &reply);
        let resolved = recover_commit(&mut f, &raw, &a, &recovery, 1804, 4);
        let record = original_receipt(&resolved).0;
        assert_eq!(record.outcome, EditorReceiptOutcomeV0::Committed);
        assert_eq!(record.original_instance_id, actor.instance_id);
        assert_eq!(f.counts(101), (1, 1, 1));
        assert!(matches!(
            f.observe(&c, 4),
            Err(NativeEditorError::Desktop(Status::Stale))
        ));
        drop((c, resolved, writer, reply, running, a, raw));
        f.reopen();
        let mut fresh = f.app(0, 101, 5);
        assert_eq!(ok(f.observe(&fresh, 5)).bytes, NEW);
        assert_ne!(
            ok(f.observe(&fresh, 5)).actor.instance_id,
            actor.instance_id
        );
        f.stroke(0, &mut fresh, 4, 5);
        let frame = f.composed(0, &mut fresh, 5);
        f.protected_golden(&frame, "UNSAVED", "");
        assert_eq!(f.counts(101), (1, 1, 1));
        drop(fresh);
        f.finish();
    }
    // Corresponding application-chain arms: private original owners stay inside the concrete app.
    for focus_fault in [false, true] {
        let mut f = Fixture::recorded(
            &recording,
            scenario(
                "ui1_1_unknown_save_survives_editor_recovery",
                "app_unknown",
                u32::from(focus_fault),
                recording::ScenarioScope::Composition,
            ),
            3,
        );
        let (mut app, endpoint) = f.app_with_endpoint(0, 101, 1);
        f.replace(0, &mut app, NEW, 2);
        let actor = ok(f.observe(&app, 2)).actor;
        let pause = Arc::new(ok(f
            .prepared()
            .controller()
            .pause_native_save(&endpoint, store::PausePoint::AfterCommitPermit)));
        f.store_pauses
            .push((f.prepared.as_ref().unwrap().clone(), pause.clone()));
        f.chord(0, &mut app, 22, 2);
        let end = Instant::now() + Duration::from_millis(1000);
        loop {
            match pause.wait_until_entered(1) {
                Ok(v) => {
                    assert!(v.entered);
                    break;
                }
                Err(e) => assert_eq!(e.status, StoreStatus::Timeout),
            }
            ok(app.step(2));
            assert!(Instant::now() < end);
            thread::yield_now();
        }
        assert!(ok(pause.wait_until_entered(1000)).permit_issued);
        f.until(&mut app, 2, |o| {
            matches!(o.phase, desktop::NativeEditorPhase::Unknown)
        });
        let original = object(&f, 101).operations[0].allocation.operation_id;
        if focus_fault {
            ok(f.dc.restart_service(desktop::ServiceKind::Focus, 3));
        } else {
            ok(f.dc.fault_native_editor(actor.instance_id, 3));
        }
        ok(f.compose(0, 3));
        unknown(&f, 0, Some(original));
        assert!(matches!(
            f.observe(&app, 3),
            Err(NativeEditorError::Desktop(Status::Stale))
        ));
        if focus_fault {
            f.reattach_input(0, 3);
        }
        let mut other = f.app(0, 103, 3);
        let frame = f.composed(0, &mut other, 3);
        f.protected_golden(&frame, "LOADED", "SAVE UNKNOWN");
        assert!(matches!(f.quiesce_recorded(1),Err(ref e)if e.status==StoreStatus::NotReady));
        ok(f.prepared().controller().release(&pause));
        assert!(ok(pause.wait_until_settled(1000)).settled);
        f.join_all();
        ok(app.reconcile(4));
        ok(f.compose(0, 4));
        no_unknown(&f, 0);
        assert_eq!(f.counts(101), (1, 1, 1));
        app_chain_artifacts(&f, 101, NEW, original, &actor, endpoint.handle);
        assert!(matches!(
            f.observe(&other, 4),
            Err(NativeEditorError::Desktop(Status::Stale))
        ));
        drop((other, app, endpoint));
        f.reopen();
        let mut fresh = f.app(0, 101, 5);
        assert_eq!(ok(f.observe(&fresh, 5)).bytes, NEW);
        assert_ne!(
            ok(f.observe(&fresh, 5)).actor.instance_id,
            actor.instance_id
        );
        f.stroke(0, &mut fresh, 4, 5);
        let frame = f.composed(0, &mut fresh, 5);
        f.protected_golden(&frame, "UNSAVED", "");
        assert_eq!(f.counts(101), (1, 1, 1));
        drop(fresh);
        f.finish();
    }
}

#[test]
fn ui1_1_clock_capacity_and_counter_limits_fail_closed() {
    let recording = ok(recording::CaseRecorder::open(
        "ui1_1_clock_capacity_and_counter_limits_fail_closed",
    ));
    // Clock highwater is shared, but actual request Instant is never caller-selected.
    let mut clock = Fixture::recorded_ttl(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "clock",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        2,
        30000,
        10,
    );
    clock.denied_at(
        "session_capacity",
        "register_store_session",
        clock
            .dc
            .register_store_session(3, volatile::APP, volatile::MANIFEST, 103, 1, 1),
        Status::Exhausted,
    );
    let a = clock.launch(0, 101, 1);
    let b = clock.launch(1, 102, 1);
    assert_eq!(ok(clock.prepared().controller().advance_time(12)), 12);
    assert_eq!(ok(clock.prepared().controller().advance_time(2)), 12);
    clock.denied_at(
        "control_main_026",
        "begin_save_read",
        clock.h.begin_save_read(
            a.desktop().bindings().artifact_origin(),
            &selected_request(&a, 1300),
            2,
        ),
        Status::Stale,
    );
    drop((a, b));
    let fresh = clock.launch(1, 102, 13);
    assert_eq!(selected(&mut clock, &fresh, 1301, 13), (1, B.to_vec()));
    drop(fresh);
    clock.finish();
    // 48 actual first Allocate+Commit executions, 96 immutable tickets; data aliases released each iteration.
    let mut history = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "history",
            0,
            recording::ScenarioScope::Composition,
        ),
        3,
    );
    for (session, object_id) in [(0, 101), (1, 102), (0, 103)] {
        let mut app = history.app(session, object_id, 1);
        for revision in 2..=17 {
            history.replace(
                session,
                &mut app,
                if revision % 2 == 0 { NEW } else { OLD },
                2,
            );
            history.chord(session, &mut app, 22, 2);
            history.until(&mut app, 2, |o| o.confirmed_revision == revision);
            history.composed(session, &mut app, 2);
            assert!(matches!(
                ok(history.observe(&app, 2)).phase,
                desktop::NativeEditorPhase::Saved
            ));
        }
        assert_eq!(history.counts(object_id), (16, 16, 16));
        drop(app);
        history.join_all();
    }
    let ev = ok(history.prepared().controller().evidence());
    assert_eq!(
        ev.objects.iter().map(|o| o.operations.len()).sum::<usize>(),
        48
    );
    assert_eq!(
        ev.exchanges
            .iter()
            .filter(|r| ok(desktop::decode_envelope_wire(&r.request_wire)).msg_type == 5)
            .count(),
        48
    );
    assert_eq!(ev.ledger.active_io_workers, 0);
    assert!(ev.io_events.len() > 1024 && ev.io_events.len() <= 65536);
    let captures = ok(history.prepared().controller().native_save_artifacts());
    assert!(captures.len() <= 256);
    let token = fence(&mut history);
    assert_eq!(token.fence_snapshot().original_tickets.len(), 96);
    history.replace_prepared(ok(store::StoreFixture::reopen_native_save(token)));
    for (session, id) in [(0, 101), (1, 102), (0, 103)] {
        let p = history.launch(session, id, 3);
        assert_eq!(selected(&mut history, &p, 1302 + id, 3).0, 17);
        drop(p);
    }
    history.finish();
    // Independent16-slot denial:32 completed tickets leave room for the genuine17th Allocate.
    let mut slots = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "slots",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let raw = raw_editor(&mut slots, 0, 101, 1);
    let mut last = None;
    for revision in 2..=17 {
        raw_replace(
            &mut slots,
            0,
            &raw,
            if revision % 2 == 0 { NEW } else { OLD },
            2,
        );
        let a = attempt(&mut slots, 0, &raw, 3300 + revision, 2);
        let r = settle(&mut slots, &a);
        let (actual, _) = receipt(&r, a.operation);
        assert_eq!(actual.result_revision, revision);
        last = Some((a, r));
    }
    let before = object(&slots, 101);
    let before_files = files(&slots.path);
    raw_replace(&mut slots, 0, &raw, NEW, 2);
    raw_chord(&mut slots, 0, &raw, 22, 2);
    let intent = ok(slots.h.take_save_intent(&raw.editor));
    let actor = intent.snapshot().view().actor;
    let q = canonical(ok(artifact::AllocateSaveId {
        request_id: 3390,
        session_id: actor.session_id,
        session_generation: actor.session_generation,
        expected_revision: 17,
    }
    .encode(raw.launch.endpoint().handle)));
    let entry = ok(slots.h.begin_save_allocate(
        raw.launch.desktop().bindings().artifact_origin(),
        &intent,
        &q,
        2,
    ));
    let registered = ok(slots
        .host()
        .register_native_save(raw.launch.endpoint(), ok(entry.wait_once())));
    let denied = ok(slots.host().execute_native_save(&registered));
    allocation_status(&denied, StoreStatus::Exhausted);
    for id in entry.producer_ids() {
        join_core(&mut slots, id);
    }
    for id in denied.producers() {
        join_core(&mut slots, id);
    }
    assert_eq!(object(&slots, 101).operations.len(), 16);
    assert_eq!(object(&slots, 101).selected, before.selected);
    assert_eq!(files(&slots.path), before_files);
    assert_eq!(slots.counts(101).1, before.permit_count);
    assert_eq!(slots.counts(101).2, before.transition_count);
    assert_eq!(
        ok(slots.prepared().controller().evidence())
            .exchanges
            .iter()
            .filter(|r| ok(desktop::decode_envelope_wire(&r.request_wire)).msg_type == 5)
            .count(),
        16
    );
    assert_eq!(
        selected(&mut slots, &raw.launch, 3391, 2),
        (17, OLD.to_vec())
    );
    let recovered = observe_commit(&mut slots, &raw, &last.as_ref().unwrap().0, 3392, 2);
    assert_eq!(original_receipt(&recovered).0.result_revision, 17);
    drop((recovered, denied, registered, entry, intent, last, raw));
    slots.finish();
    // Pin16 has no launched contexts/data/producers that can mask its own retention ceiling.
    let mut pins = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "pins",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let mut held_pins = Vec::new();
    for _ in 0..16 {
        held_pins.push(ok(pins.prepared().pin_selection(
            &pins.drivers[0].s.chrome.peer,
            101,
            30000,
        )));
    }
    let before = files(&pins.path);
    pins.denied_at(
        "control_main_027",
        "pin_selection",
        pins.prepared()
            .pin_selection(&pins.drivers[0].s.chrome.peer, 101, 30000),
        StoreStatus::Exhausted,
    );
    assert_eq!(pins.producer_counts("producer_checkpoint").held, 0);
    assert_eq!(files(&pins.path), before);
    drop(held_pins);
    let fresh = pins.launch(0, 101, 2);
    assert_eq!(selected(&mut pins, &fresh, 1390, 2), (1, OLD.to_vec()));
    drop(fresh);
    pins.finish();
    // Sixteen live documents/contexts/four-grant sets are a coupled admission ceiling.
    let mut contexts = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "contexts",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let mut live = Vec::new();
    for _ in 0..16 {
        live.push(contexts.app(0, 101, 1));
    }
    let (selector, pin, _) = contexts.prepare(0, 101, 2);
    let mut approval = contexts.approval(0, 2);
    contexts.denied_at(
        "context_confirm_exhausted",
        "confirm_save_preview",
        contexts
            .h
            .confirm_save_preview(&contexts.drivers[0].s.chrome.peer, &mut approval, 2),
        Status::Exhausted,
    );
    assert_eq!(object(&contexts, 101).selected.revision, 1);
    assert_eq!(contexts.counts(101), (0, 0, 0));
    assert_eq!(contexts.producer_counts("producer_checkpoint").held, 0);
    assert_eq!(selected_blob(&contexts, 101), OLD);
    drop((selector, pin, approval, live));
    contexts.finish();
    // Query64 is reached with no Store data leases and with each real entry pair joined.
    let mut queries = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "queries",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let p = queries.launch(0, 101, 1);
    let mut held = Vec::new();
    let mut ids = Vec::new();
    for i in 0..64 {
        let entry = ok(queries.h.begin_save_read(
            p.desktop().bindings().artifact_origin(),
            &selected_request(&p, 1400 + i),
            1,
        ));
        ids.push(entry.origin_id());
        let call = ok(entry.wait_once());
        for producer in entry.producer_ids() {
            queries.join(producer)
        }
        held.push(call);
    }
    assert!(ids.windows(2).all(|w| w[0] < w[1]));
    queries.denied_at(
        "control_main_028",
        "begin_save_read",
        queries.h.begin_save_read(
            p.desktop().bindings().artifact_origin(),
            &selected_request(&p, 1464),
            1,
        ),
        Status::Exhausted,
    );
    assert_eq!(queries.producer_counts("producer_checkpoint").held, 0);
    drop(held);
    let fresh = ok(queries.h.begin_save_read(
        p.desktop().bindings().artifact_origin(),
        &selected_request(&p, 1465),
        1,
    ));
    assert!(fresh.origin_id() > ids[63]);
    let reg = ok(queries
        .host()
        .register_native_save(p.endpoint(), ok(fresh.wait_once())));
    let r = ok(queries.host().execute_native_save(&reg));
    assert_eq!(ok(artifact::ReadSelectedReply::decode(r.wire())).status, 0);
    for id in fresh.producer_ids() {
        join_core(&mut queries, id)
    }
    for id in r.producers() {
        join_core(&mut queries, id)
    }
    drop((r, reg, fresh, p));
    queries.finish();
    // Thirty-two genuinely paused dispatcher/supervisor pairs, not caller threads.
    let mut workers = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "workers",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let p = workers.launch(0, 101, 1);
    let mut pending = Vec::new();
    for i in 0..32 {
        let pause = desktop_pause(
            &mut workers,
            0,
            desktop::NativeSavePausePoint::BeforeEntryAdmission,
        );
        let entry = ok(workers.h.begin_save_read(
            p.desktop().bindings().artifact_origin(),
            &selected_request(&p, 1500 + i),
            1,
        ));
        assert!(ok(pause.wait_until_entered(1000)).entered);
        pending.push((pause, entry));
    }
    assert_eq!(workers.producer_counts("producer_checkpoint").held, 64);
    workers.denied_at(
        "control_main_029",
        "begin_save_read",
        workers.h.begin_save_read(
            p.desktop().bindings().artifact_origin(),
            &selected_request(&p, 1532),
            1,
        ),
        Status::Exhausted,
    );
    let before = workers.producer_counts("producer_checkpoint").held;
    workers.denied_at(
        "entry_join_pending",
        "join_native_finished",
        workers
            .h
            .join_native_finished(pending[0].1.producer_ids()[0]),
        Status::NotReady,
    );
    assert_eq!(workers.producer_counts("producer_checkpoint").held, before);
    for (pause, entry) in pending {
        workers.denied_at(
            "entry_wait_timeout",
            "NativeSaveEntry.wait_once",
            entry.wait_once(),
            Status::Timeout,
        );
        ok(workers.dc.release_native_save(&pause));
        assert!(ok(pause.wait_until_settled(1000)).settled);
        for id in entry.producer_ids() {
            workers.join(id)
        }
    }
    assert_eq!(workers.joined.len(), 64);
    assert_eq!(workers.producer_counts("producer_checkpoint").held, 0);
    assert_eq!(selected(&mut workers, &p, 1533, 2), (1, OLD.to_vec()));
    drop(p);
    workers.finish();
    // Data32 is independent from the retained query64 cap.
    let mut data = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "data",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let p = data.launch(0, 101, 1);
    let mut leases = Vec::new();
    for i in 0..32 {
        let e = ok(data.h.begin_save_read(
            p.desktop().bindings().artifact_origin(),
            &selected_request(&p, 1600 + i),
            1,
        ));
        let reg = ok(data
            .host()
            .register_native_save(p.endpoint(), ok(e.wait_once())));
        let r = ok(data.host().execute_native_save(&reg));
        assert!(r.selected().is_some());
        for id in e.producer_ids() {
            join_core(&mut data, id)
        }
        for id in r.producers() {
            join_core(&mut data, id)
        }
        leases.push((reg, r));
    }
    assert_eq!(
        ok(data.prepared().controller().evidence())
            .ledger
            .shared_objects,
        32
    );
    let e = ok(data.h.begin_save_read(
        p.desktop().bindings().artifact_origin(),
        &selected_request(&p, 1632),
        1,
    ));
    let reg = ok(data
        .host()
        .register_native_save(p.endpoint(), ok(e.wait_once())));
    let r = ok(data.host().execute_native_save(&reg));
    read_reply_status(&r, StoreStatus::Exhausted);
    for id in e.producer_ids() {
        join_core(&mut data, id)
    }
    for id in r.producers() {
        join_core(&mut data, id)
    }
    let high = leases
        .last()
        .unwrap()
        .1
        .selected()
        .unwrap()
        .descriptor()
        .handle
        .generation;
    drop((leases, r, reg, e));
    let e = ok(data.h.begin_save_read(
        p.desktop().bindings().artifact_origin(),
        &selected_request(&p, 1633),
        1,
    ));
    let reg = ok(data
        .host()
        .register_native_save(p.endpoint(), ok(e.wait_once())));
    let r = ok(data.host().execute_native_save(&reg));
    assert!(r.selected().unwrap().descriptor().handle.generation > high);
    for id in e.producer_ids() {
        join_core(&mut data, id)
    }
    for id in r.producers() {
        join_core(&mut data, id)
    }
    drop((r, reg, e, p));
    data.finish();
    // IO2 is contained within shared64: actual A+B permitted IO remain held while C reads.
    let mut io = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "io",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        3,
    );
    let a = raw_editor(&mut io, 0, 101, 1);
    raw_replace(&mut io, 0, &a, NEW, 2);
    let aa = attempt(&mut io, 0, &a, 1700, 2);
    let b = raw_editor(&mut io, 1, 102, 1);
    raw_replace(&mut io, 1, &b, NEW, 2);
    let bb = attempt(&mut io, 1, &b, 1710, 2);
    let pa = store_pause(&mut io, &a, store::PausePoint::AfterCommitPermit);
    let ea = ok(io.host().start_native_save(&aa.commit));
    assert!(ok(pa.wait_until_entered(1000)).permit_issued);
    let pb = store_pause(&mut io, &b, store::PausePoint::AfterCommitPermit);
    let eb = ok(io.host().start_native_save(&bb.commit));
    assert!(ok(pb.wait_until_entered(1000)).permit_issued);
    assert_eq!(
        ok(io.prepared().controller().evidence())
            .ledger
            .active_io_workers,
        2
    );
    io.store_snapshot("io_two_writers_held", "source_checkpoint", None, None);
    revoke(&io, 0, &aa.intent.snapshot().view().actor, 3);
    let c = raw_editor(&mut io, 0, 103, 3);
    assert_eq!(ok(io.editor_observation(&c.editor, 3)).bytes, C);
    let cc = allocate_start(&mut io, 0, &c, 1720, 3);
    let r = ok(io.host().execute_native_save(&cc.registered));
    allocation_status(&r, StoreStatus::Exhausted);
    assert_eq!(object(&io, 103).operations.len(), 0);
    assert_eq!(
        ok(io.prepared().controller().evidence())
            .ledger
            .active_io_workers,
        2
    );
    io.store_snapshot(
        "io_third_allocation_denied",
        "source_checkpoint",
        None,
        None,
    );
    for id in cc.entry.producer_ids() {
        join_core(&mut io, id)
    }
    for id in r.producers() {
        join_core(&mut io, id)
    }
    ok(io.prepared().controller().release(&pa));
    ok(io.prepared().controller().release(&pb));
    assert!(ok(pa.wait_until_settled(1000)).settled && ok(pb.wait_until_settled(1000)).settled);
    let ra = pending_reply(&io, &ea);
    let rb = pending_reply(&io, &eb);
    join_attempt(&mut io, &aa, &ra);
    join_attempt(&mut io, &bb, &rb);
    assert!(io.producer_counts("producer_checkpoint").held <= 64);
    drop((r, cc, c, ra, rb, ea, eb, aa, bb, a, b));
    io.finish();
    // Capture256 is independent of query/data quotas: repeat full copies of ONE real lease.
    let mut captures = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "captures",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let raw = raw_editor(&mut captures, 0, 101, 1);
    raw_replace(&mut captures, 0, &raw, NEW, 2);
    let staged = attempt(&mut captures, 0, &raw, 1800, 2);
    let entry = ok(captures.h.begin_save_read(
        raw.launch.desktop().bindings().artifact_origin(),
        &selected_request(&raw.launch, 1802),
        2,
    ));
    let registered = ok(captures
        .host()
        .register_native_save(raw.launch.endpoint(), ok(entry.wait_once())));
    let read = ok(captures.host().execute_native_save(&registered));
    let lease = read.selected().unwrap();
    let size = lease.descriptor().byte_len as usize;
    assert!((64..=4160).contains(&size));
    let initial = ok(captures.prepared().controller().native_save_artifacts());
    let prior = capture_signature(&initial);
    assert!(initial.len() < 256);
    for _ in initial.len()..256 {
        let mut bytes = vec![0xa5; size];
        ok(lease.copy_into(0, &mut bytes));
        let header = ok(EditorTextHeaderV0::decode_le(&bytes[..64]));
        ok(header.validate_bytes(&bytes[64..]));
        assert_eq!(&bytes[64..], OLD);
    }
    let full = ok(captures.prepared().controller().native_save_artifacts());
    assert_eq!(full.len(), 256);
    assert!(full.windows(2).all(|r| r[0].capture_id < r[1].capture_id));
    assert_eq!(&capture_signature(&full)[..prior.len()], prior.as_slice());
    for row in &full[prior.len()..] {
        assert!(matches!(
            row.kind,
            store::NativeSaveArtifactKind::SelectedTextCopy
        ));
        assert_eq!(&row.bytes[64..], OLD);
        assert_eq!(
            row.capture_request,
            ok(desktop::encode_envelope_wire(&selected_request(
                &raw.launch,
                1802
            )))
        );
    }
    let full = capture_signature(&full);
    let files_before = files(&captures.path);
    let counts_before = captures.counts(101);
    let mut denied = vec![0x93; size];
    captures.denied_at(
        "capture_full_copy_exhausted",
        "NativeSelectedRead.copy_into",
        lease.copy_into(0, &mut denied),
        StoreStatus::Exhausted,
    );
    assert_eq!(denied, vec![0x93; size]);
    assert_eq!(
        capture_signature(&ok(captures
            .prepared()
            .controller()
            .native_save_artifacts())),
        full
    );
    assert_eq!(files(&captures.path), files_before);
    assert_eq!(captures.counts(101), counts_before);
    let io_before = ok(captures.prepared().controller().evidence())
        .io_events
        .into_iter()
        .filter(|r| r.operation_id == staged.operation)
        .map(|r| (r.sequence, r.writer_id))
        .collect::<Vec<_>>();
    captures.store_snapshot(
        "capture_capacity_before_commit",
        "source_checkpoint",
        None,
        None,
    );
    let before_selection = store_pause(
        &mut captures,
        &raw,
        store::PausePoint::BeforeSelectionRename,
    );
    let after_selection = store_pause(&mut captures, &raw, store::PausePoint::AfterSelectionRename);
    assert!(
        Instant::now() < staged.intent.deadline(),
        "capture boundary must be tested inside the REAL original intent lifetime"
    );
    let capture_execution = ok(captures.host().start_native_save(&staged.commit));
    let denied_commit = pending_reply(&captures, &capture_execution);
    join_attempt(&mut captures, &staged, &denied_commit);
    commit_status(&denied_commit, StoreStatus::Exhausted, 0);
    for member in [&before_selection, &after_selection] {
        let actual = ok(member.wait_until_settled(1000));
        assert!(actual.settled);
        assert!(
            !actual.entered,
            "rejected source must cancel unentered same-original pair"
        );
    }
    assert_eq!(
        captures.counts(101),
        (counts_before.0 + 1, counts_before.1, counts_before.2)
    );
    assert_eq!(object(&captures, 101).selected.revision, 1);
    let row = object(&captures, 101)
        .operations
        .into_iter()
        .find(|r| r.allocation.operation_id == staged.operation)
        .unwrap();
    assert!(row.permit.is_none());
    assert_eq!(
        capture_signature(&ok(captures
            .prepared()
            .controller()
            .native_save_artifacts())),
        full
    );
    no_late_io(&captures, staged.operation, &io_before);
    captures.store_snapshot(
        "capture_capacity_after_commit",
        "source_checkpoint",
        None,
        None,
    );
    for id in entry.producer_ids() {
        join_core(&mut captures, id)
    }
    for id in read.producers() {
        join_core(&mut captures, id)
    }
    let commit_rows = ok(captures.prepared().controller().evidence())
        .exchanges
        .into_iter()
        .filter(|r| ok(desktop::decode_envelope_wire(&r.request_wire)).msg_type == 5)
        .collect::<Vec<_>>();
    assert_eq!(commit_rows.len(), 1);
    assert_eq!(
        commit_rows[0].request_wire,
        ok(desktop::encode_envelope_wire(&staged.request))
    );
    drop((
        denied_commit,
        capture_execution,
        before_selection,
        after_selection,
        read,
        registered,
        entry,
        staged,
        raw,
    ));
    captures.finish();
    // Save-specific total frame history128 and literal profile; no claim release replenishes history.
    let mut frames = Fixture::recorded(
        &recording,
        scenario(
            "ui1_1_clock_capacity_and_counter_limits_fail_closed",
            "frames",
            0,
            recording::ScenarioScope::DirectNative,
        ),
        1,
    );
    let raw = raw_editor(&mut frames, 0, 101, 1);
    let limits = ok(frames.h.native_save_profile());
    assert_eq!(
        (
            limits.selected_object_limit,
            limits.trace_capacity,
            limits.composed_history_capacity,
            limits.pending_frame_capacity
        ),
        (3, 131072, 128, 16)
    );
    for _ in 1..128 {
        ok(frames.h.render_native_editor(&raw.editor, 1));
        ok(frames.compose(0, 1));
    }
    frames.denied_at(
        "control_main_030",
        "render_native_editor",
        frames.h.render_native_editor(&raw.editor, 1),
        Status::Exhausted,
    );
    assert_eq!(object(&frames, 101).selected.revision, 1);
    drop(raw);
    frames.finish();
    native_max_controls(&recording);
    store_max_controls(&recording);
}
