//! Supplemental review regressions; the original eighteen task assertions stay fixed.
#![cfg(feature = "host_native_save_v0_dev")]
#[allow(dead_code)]
#[path = "native_task_support/mod.rs"]
mod support;
use desktop_service::{
    dev::Status,
    editor_dev::{self as desktop, EditorMessage},
};
use kernel_api::generated::{
    desktop_artifact_v1 as artifact, desktop_editor_session_v1 as session,
    desktop_focus_v1 as focus, desktop_surface_v1 as surface,
};
use std::{thread, time::Duration};
use support::*;

fn fixture(name: &'static str, n: usize) -> Fixture {
    let recording = ok(recording::CaseRecorder::open(name));
    let mut spec = scenario(
        "ui1_1_clock_capacity_and_counter_limits_fail_closed",
        "repair",
        0,
        recording::ScenarioScope::DirectNative,
    );
    spec.case = name;
    spec.requirements = &["supplemental_desktop_reliability"];
    Fixture::recorded(&recording, spec, n)
}
fn restore(f: &Fixture, raw: &RawEditor, request_id: u64) {
    let boot = boot(&raw.launch);
    let bytes = volatile::read(
        &f.h,
        raw.launch.desktop().bindings().self_status(),
        &volatile::desc(
            boot.grants_shm,
            desktop::ObjectKind::Grants,
            boot.grants_len,
        ),
        2,
    );
    let grants =
        artifact_store_schema::editor_native_save::EditorNativeSaveDataV0::decode_le_for_phase(
            &bytes,
            artifact_store_schema::editor_native_save::EditorNativeSavePhaseV0::Active,
        )
        .unwrap();
    let r: focus::AssignReply = volatile::call(
        &f.h,
        &f.drivers[0].s.chrome,
        focus::Assign {
            request_id,
            session_id: boot.session_id,
            session_generation: boot.session_generation,
            instance_id: boot.instance_id,
            instance_generation: boot.instance_generation,
            surface_id: grants.records[2].resource_id,
            surface_generation: grants.records[2].resource_generation,
        },
        2,
    );
    assert_eq!(r.status, 0);
}
fn acquired(f: &Fixture, raw: &RawEditor) -> (desktop::ObjectDescriptor, u64) {
    let evidence = ok(f.dc.evidence());
    let r = evidence
        .exchanges
        .iter()
        .rev()
        .find_map(|(q, r)| {
            (q.protocol == 833 && q.msg_type == 3)
                .then(|| surface::AcquireReply::decode(r).unwrap())
        })
        .unwrap();
    let _ = raw;
    (
        volatile::desc(r.buffer_shm, desktop::ObjectKind::SurfaceBuffer, r.byte_len),
        r.mapping_generation,
    )
}

#[test]
fn reliability_initial_approved_launch_has_focus() {
    let mut f = fixture("reliability_initial_approved_launch_has_focus", 1);
    let launch = f.launch(0, 101, 1);
    let b = boot(&launch);
    let r: session::ObserveFocusReply = volatile::call(
        &f.h,
        launch.desktop().bindings().self_status(),
        session::ObserveFocus {
            request_id: 700,
            session_id: b.session_id,
            session_generation: b.session_generation,
            instance_id: b.instance_id,
            instance_generation: b.instance_generation,
        },
        1,
    );
    assert_eq!(
        r.status, 0,
        "approved launch must receive focus before first redraw"
    );
    assert_ne!(r.focus_epoch, 0);
    drop(launch);
    f.finish();
}

#[test]
fn reliability_stale_render_releases_acquisition() {
    let mut f = fixture("reliability_stale_render_releases_acquisition", 1);
    let raw = raw_editor(&mut f, 0, 101, 1);
    let pause = desktop_pause(
        &mut f,
        0,
        desktop::NativeSavePausePoint::BeforePresentFreeze,
    );
    let h = f.h.clone();
    let editor = &raw.editor;
    thread::scope(|scope| {
        let worker = scope.spawn(move || h.render_native_editor(editor, 2));
        assert!(ok(pause.wait_until_entered(1000)).entered);
        f.drivers[0].launcher(&f.h, 2);
        ok(f.dc.release_native_save(&pause));
        assert_eq!(worker.join().unwrap(), Err(Status::Stale));
        assert!(ok(pause.wait_until_settled(1000)).settled);
    });
    restore(&f, &raw, 702);
    let fresh = ok(f.h.render_native_editor(&raw.editor, 2));
    let composed = ok(f.h.compose_native_save(&f.drivers[0].s.compositor, 2));
    assert_eq!(
        (fresh.actor.instance_id, fresh.sequence),
        (composed.instance_id, composed.sequence)
    );
    drop(raw);
    f.finish();
}

#[test]
fn reliability_background_redraw_preserves_launcher_focus() {
    let mut f = fixture("reliability_background_redraw_preserves_launcher_focus", 1);
    let raw = raw_editor(&mut f, 0, 101, 1);
    let before = ok(f.h.editor_observation(&raw.editor, 1));
    f.drivers[0].launcher(&f.h, 2);
    assert_eq!(
        f.h.render_native_editor(&raw.editor, 2),
        Err(Status::NotReady)
    );
    f.drivers[0].send(&f.h, 4, 1, 2);
    let step = ok(f.h.step_native_editor(&raw.editor, 2));
    assert_eq!(step.consumed_sequence, None);
    assert_eq!(
        ok(f.h.editor_observation(&raw.editor, 2)).bytes,
        before.bytes
    );
    f.drivers[0].send(&f.h, 4, 2, 2);
    restore(&f, &raw, 703);
    let frame = ok(f.h.render_native_editor(&raw.editor, 2));
    assert_eq!(
        ok(f.h.compose_native_save(&f.drivers[0].s.compositor, 2)).sequence,
        frame.sequence
    );
    drop(raw);
    f.finish();
}

#[test]
fn reliability_abandoned_acquisition_retires_only_its_generation() {
    let mut f = fixture(
        "reliability_abandoned_acquisition_retires_only_its_generation",
        1,
    );
    let raw = raw_editor(&mut f, 0, 101, 1);
    let pause = desktop_pause(
        &mut f,
        0,
        desktop::NativeSavePausePoint::BeforePresentFreeze,
    );
    let h = f.h.clone();
    let editor = &raw.editor;
    let (alias, cloned_alias, other_alias, mapping) = thread::scope(|scope| {
        let worker = scope.spawn(move || h.render_native_editor(editor, 2));
        assert!(ok(pause.wait_until_entered(1000)).entered);
        let (descriptor, mapping) = acquired(&f, &raw);
        let alias = ok(f.h.write_lease(
            &raw.launch.desktop().bindings().surface().peer,
            &descriptor,
            mapping,
            2,
        ));
        let cloned_alias = alias.clone();
        // A separate buffer acquisition is held across exact-generation cleanup.
        let initial = boot(&raw.launch);
        let bytes = volatile::read(
            &f.h,
            raw.launch.desktop().bindings().self_status(),
            &volatile::desc(
                initial.grants_shm,
                desktop::ObjectKind::Grants,
                initial.grants_len,
            ),
            2,
        );
        let grants =
            artifact_store_schema::editor_native_save::EditorNativeSaveDataV0::decode_le_for_phase(
                &bytes,
                artifact_store_schema::editor_native_save::EditorNativeSavePhaseV0::Active,
            )
            .unwrap();
        let other: surface::AcquireReply = volatile::call(
            &f.h,
            raw.launch.desktop().bindings().surface(),
            surface::Acquire {
                request_id: 705,
                surface_id: grants.records[2].resource_id,
                surface_generation: grants.records[2].resource_generation,
                buffer_index: 0,
                reserved: 0,
            },
            2,
        );
        assert_eq!(other.status, 0);
        assert_ne!(other.buffer_shm, descriptor.handle.pack());
        let other_descriptor = volatile::desc(
            other.buffer_shm,
            desktop::ObjectKind::SurfaceBuffer,
            other.byte_len,
        );
        let other_alias = ok(f.h.write_lease(
            &raw.launch.desktop().bindings().surface().peer,
            &other_descriptor,
            other.mapping_generation,
            2,
        ));
        f.drivers[0].launcher(&f.h, 2);
        ok(f.dc.release_native_save(&pause));
        assert_eq!(worker.join().unwrap(), Err(Status::Stale));
        (alias, cloned_alias, other_alias, mapping)
    });
    assert_eq!(alias.copy_from(0, &[0], 2), Err(Status::Stale));
    assert_eq!(cloned_alias.copy_from(0, &[0], 2), Err(Status::Stale));
    assert_eq!(other_alias.copy_from(0, &[7], 2), Ok(()));
    restore(&f, &raw, 704);
    let fresh = ok(f.h.render_native_editor(&raw.editor, 2));
    assert_ne!(fresh.mapping_generation, mapping);
    assert_eq!(alias.copy_from(0, &[0], 2), Err(Status::Stale));
    assert_eq!(
        ok(f.h.compose_native_save(&f.drivers[0].s.compositor, 2)).sequence,
        fresh.sequence
    );
    drop((alias, cloned_alias, other_alias, raw));
    f.finish();
}

#[test]
fn reliability_expired_key_creates_no_save() {
    let mut f = fixture("reliability_expired_key_creates_no_save", 1);
    let mut app = f.app(0, 101, 1);
    f.drivers[0].send(&f.h, 224, 1, 2);
    ok(app.step(2));
    f.drivers[0].send(&f.h, 22, 1, 2);
    thread::sleep(Duration::from_millis(1050));
    assert_eq!(
        app.step(2),
        Err(artifact_editor::NativeEditorError::Desktop(Status::Timeout))
    );
    assert_ne!(
        ok(app.observe(2)).phase,
        desktop::NativeEditorPhase::AllocationPending
    );
    assert_eq!(f.counts(101), (0, 0, 0));
    f.drivers[0].send(&f.h, 22, 2, 2);
    ok(app.step(2));
    f.drivers[0].send(&f.h, 224, 2, 2);
    ok(app.step(2));
    f.save(0, &mut app, 2);
    assert_eq!(ok(app.observe(2)).bytes, OLD);
    assert_eq!(f.counts(101), (1, 1, 1));
    drop(app);
    f.finish();
}

#[test]
fn reliability_expired_intent_closes_before_allocation() {
    let mut f = fixture("reliability_expired_intent_closes_before_allocation", 2);
    let raw = raw_editor(&mut f, 0, 101, 1);
    let foreign = raw_editor(&mut f, 1, 102, 1);
    raw_chord(&mut f, 0, &raw, 22, 2);
    let intent = ok(f.h.take_save_intent(&raw.editor));
    let v = intent.snapshot().view();
    let q = ok(artifact::AllocateSaveId {
        request_id: 710,
        session_id: v.actor.session_id,
        session_generation: v.actor.session_generation,
        expected_revision: v.expected_revision,
    }
    .encode(raw.launch.endpoint().handle));
    thread::sleep(Duration::from_millis(1050));
    assert!(matches!(
        f.h.begin_save_allocate(
            foreign.launch.desktop().bindings().artifact_origin(),
            &intent,
            &q,
            2
        ),
        Err(Status::Denied)
    ));
    assert_eq!(
        ok(f.h.editor_observation(&raw.editor, 2)).phase,
        desktop::NativeEditorPhase::AllocationPending
    );
    assert!(matches!(
        f.h.begin_save_allocate(
            raw.launch.desktop().bindings().artifact_origin(),
            &intent,
            &q,
            2
        ),
        Err(Status::Timeout)
    ));
    assert_eq!(
        ok(f.h.editor_observation(&raw.editor, 2)).phase,
        desktop::NativeEditorPhase::Unsaved
    );
    assert_eq!(f.counts(101), (0, 0, 0));
    let a = attempt(&mut f, 0, &raw, 712, 2);
    let reply = ok(f.host().execute_native_save(&a.commit));
    receipt(&reply, a.operation);
    assert_eq!(ok(f.h.editor_observation(&raw.editor, 2)).bytes, OLD);
    assert_eq!(f.counts(101), (1, 1, 1));
    let saved = ok(f.h.editor_observation(&raw.editor, 2));
    assert!(matches!(
        f.h.begin_save_allocate(
            raw.launch.desktop().bindings().artifact_origin(),
            &intent,
            &q,
            2
        ),
        Err(Status::Stale)
    ));
    assert_eq!(ok(f.h.editor_observation(&raw.editor, 2)), saved);
    assert_eq!(f.counts(101), (1, 1, 1));
    drop((reply, a, intent, foreign, raw));
    f.finish();
}

#[test]
fn reliability_stale_frame_publication_releases_frozen_frame() {
    let mut f = fixture(
        "reliability_stale_frame_publication_releases_frozen_frame",
        1,
    );
    let raw = raw_editor(&mut f, 0, 101, 1);
    let pause = desktop_pause(&mut f, 0, desktop::NativeSavePausePoint::BeforeFramePublish);
    let h = f.h.clone();
    thread::scope(|scope| {
        let editor = &raw.editor;
        let worker = scope.spawn(move || h.render_native_editor(editor, 2));
        assert!(ok(pause.wait_until_entered(1000)).entered);
        f.drivers[0].launcher(&f.h, 2);
        ok(f.dc.release_native_save(&pause));
        assert_eq!(worker.join().unwrap(), Err(Status::Stale));
        assert!(ok(pause.wait_until_settled(1000)).settled);
    });
    restore(&f, &raw, 720);
    let fresh = ok(f.h.render_native_editor(&raw.editor, 2));
    assert_eq!(
        ok(f.h.compose_native_save(&f.drivers[0].s.compositor, 2)).sequence,
        fresh.sequence
    );
    drop(raw);
    f.finish();
}
