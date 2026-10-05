//! Gate-first UI1.1a assertions: actual typed host consumer, volatile backend.
//! No editor process, Store IO, native IPC, device or target claim.
#![cfg(feature = "desktop_v0_dev")]
#[path = "support/editor_host_assertions.rs"]
mod support;
use desktop_service::dev::Status;
use desktop_service::editor_dev::*;
use kernel_api::generated::{
    desktop_artifact_v1 as artifact, desktop_editor_session_v1 as editor,
    desktop_focus_v1 as focus, desktop_surface_v1 as surface,
};
use sha2::Digest;
use std::thread;
use std::time::{Duration, Instant};
use support::*;

#[test]
fn ui1_1_keyboard_edits_ascii_and_renders_owned_frame() {
    let name = "ui1_1_keyboard_edits_ascii_and_renders_owned_frame";
    let mut f = Fixture::new(b"note=old\n");
    let mut l = f.launch(1);
    let mut r = Record::new(name);
    let old = render(&f.h, &f.d, &mut l, 1);
    assert_golden(&old, false);
    assert_volatile_label(&old);
    r.frame(old);
    f.d.ctrl(&f.h, &mut l, 4, 2);
    assert_eq!(l.client.snapshot().selection, Some((0, 9)));
    f.d.text(&f.h, &mut l, b"note=new\n", 2);
    assert_eq!(l.client.snapshot().bytes, b"note=new\n");
    let new = render(&f.h, &f.d, &mut l, 2);
    assert_golden(&new, true);
    r.frame(new);
    f.d.key(&f.h, &mut l, 80, 3);
    assert_eq!(l.client.snapshot().cursor, 8);
    f.d.key(&f.h, &mut l, 74, 3);
    assert_eq!(l.client.snapshot().cursor, 0);
    f.d.event(&f.h, &mut l, 225, 1, 3);
    f.d.key(&f.h, &mut l, 77, 3);
    f.d.event(&f.h, &mut l, 225, 2, 3);
    assert_eq!(l.client.snapshot().selection, Some((0, 8)));
    assert_eq!(selected(&f.h, &l, 3), (1, b"note=old\n".to_vec()));
    r.note(
        "edit",
        &format!(
            "{{\"before_hash\":\"{}\",\"after_hash\":\"{}\",\"cursor\":8,\"selection\":[0,8]}}",
            hash(b"note=old\n"),
            hash(b"note=new\n")
        ),
    );
    r.finish(&f.c, 0);
    let mut scroll = Fixture::new(&vec![b'\n'; 4096]);
    let mut s = scroll.launch(1);
    assert_eq!(s.client.snapshot().bytes.len(), 4096);
    assert!(s.client.snapshot().first_visible_line > 0);
    let mut rr = Record::new(name);
    rr.frame(render(&scroll.h, &scroll.d, &mut s, 1));
    rr.note(
        "scroll",
        &format!(
            "{{\"byte_len\":4096,\"first_visible_line\":{}}}",
            s.client.snapshot().first_visible_line
        ),
    );
    rr.finish(&scroll.c, 1);
}

#[test]
fn ui1_1_ascii_bounds_preserve_prior_draft() {
    let name = "ui1_1_ascii_bounds_preserve_prior_draft";
    let mut f = Fixture::new(&vec![b'a'; 4095]);
    let mut l = f.launch(1);
    let mut r = Record::new(name);
    f.d.key(&f.h, &mut l, 4, 2);
    assert_eq!(l.client.snapshot().bytes.len(), 4096);
    let before = l.client.snapshot();
    f.d.send(&f.h, 4, 1, 2);
    match l.client.step(2) {
        Ok(o) => assert!(!o.draft_changed),
        Err(Status::Exhausted) => {}
        _ => panic!("overflow was accepted"),
    }
    f.d.event(&f.h, &mut l, 4, 2, 2);
    assert_draft_equal(&before, &l.client.snapshot());
    for (usage, phase, mods, status) in [
        (50, 1, 0, Status::Unsupported),
        (4, 1, 4, Status::Invalid),
        (4, 2, 0, Status::Invalid),
    ] {
        let ledger = f.c.evidence().unwrap().ledger;
        deny(
            &f.h,
            &f.d.s.input,
            f.d.request(usage, phase, mods),
            8,
            2,
            status,
        );
        assert_draft_equal(&before, &l.client.snapshot());
        assert_ledger_equal(&ledger, &f.c.evidence().unwrap().ledger);
    }
    assert!(matches!(
        f.h.draft_source(&l.bindings.artifact.peer, 1, &[0xff], 2),
        Err(Status::Invalid)
    ));
    assert!(matches!(
        f.h.draft_source(&l.bindings.artifact.peer, 1, &vec![b'a'; 4097], 2),
        Err(Status::Exhausted)
    ));
    f.d.ctrl(&f.h, &mut l, 4, 3);
    f.d.text(&f.h, &mut l, b"A\t!\n", 3);
    assert_eq!(l.client.snapshot().bytes, b"A\t!\n");
    r.frame(render(&f.h, &f.d, &mut l, 3));
    r.note("ascii_bounds","{\"accepted_edge\":4096,\"rejected_edge\":4097,\"prior_preserved\":true,\"lf_tab_shift_witness\":true}");
    r.finish(&f.c, 0);
}

#[test]
fn ui1_1_focus_routes_reserved_keys_to_trusted_chrome() {
    let name = "ui1_1_focus_routes_reserved_keys_to_trusted_chrome";
    let mut f = Fixture::new(b"note=old\n");
    let mut l = f.launch(1);
    let mut b = Driver::register(&f.h, &f.c, 2, b"other\n", 1);
    let mut bl = b.launch(&f.h, &f.c, 1);
    let before = l.client.snapshot();
    let ledger = f.c.evidence().unwrap().ledger;
    let prepare = editor::PrepareLaunch {
        request_id: 20,
        session_id: f.d.s.session_id,
        session_generation: f.d.s.session_generation,
        application_hash: APP,
        requested_rights: 63,
        reserved: 0,
    };
    deny(
        &f.h,
        &l.bindings.self_status,
        prepare,
        36,
        2,
        Status::Denied,
    );
    deny(
        &f.h,
        &l.bindings.self_status,
        editor::ConfirmLaunch {
            request_id: 21,
            session_id: f.d.s.session_id,
            session_generation: f.d.s.session_generation,
            plan_id: 1,
            preview_revision: 1,
        },
        44,
        2,
        Status::Denied,
    );
    deny(
        &f.h,
        &l.bindings.self_status,
        editor::PrepareRestart {
            request_id: 22,
            session_id: f.d.s.session_id,
            session_generation: f.d.s.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        36,
        2,
        Status::Denied,
    );
    deny(
        &f.h,
        &l.bindings.surface,
        focus::Assign {
            request_id: 23,
            session_id: f.d.s.session_id,
            session_generation: f.d.s.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
            surface_id: l.surface.surface_id,
            surface_generation: l.surface.surface_generation,
        },
        24,
        2,
        Status::Denied,
    );
    deny(
        &f.h,
        &l.bindings.focus_read,
        f.d.request(40, 1, 0),
        8,
        2,
        Status::Denied,
    );
    deny(
        &f.h,
        &l.bindings.surface,
        surface::Consume {
            request_id: 24,
            surface_id: l.surface.surface_id,
            surface_generation: l.surface.surface_generation,
            sequence: 1,
        },
        16,
        2,
        Status::Denied,
    );
    deny(
        &f.h,
        &l.bindings.self_status,
        status_request(&bl, &b.s),
        44,
        2,
        Status::Denied,
    );
    deny(
        &f.h,
        &l.bindings.self_status,
        editor::CancelPreview {
            request_id: 50,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            plan_id: 1,
        },
        8,
        2,
        Status::Denied,
    );
    deny(
        &f.h,
        &l.bindings.self_status,
        editor::CloseInstance {
            request_id: 51,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        8,
        2,
        Status::Denied,
    );
    deny(
        &f.h,
        &l.bindings.self_status,
        editor::RevokeInstance {
            request_id: 52,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        8,
        2,
        Status::Denied,
    );
    let mut foreign_input = f.d.request(4, 1, 0);
    foreign_input.session_id = b.s.session_id;
    foreign_input.session_generation = b.s.session_generation;
    foreign_input.queue_generation = b.queue;
    deny(&f.h, &f.d.s.input, foreign_input, 8, 2, Status::Denied);
    let mut spoof = prepare.encode(f.d.s.chrome.handle).unwrap();
    spoof.handle = f.d.s.chrome.handle;
    zero_reply(
        &f.h.dispatch(&l.bindings.self_status.peer, &spoof, 2),
        36,
        Status::Denied,
    );
    assert_ledger_equal(&ledger, &f.c.evidence().unwrap().ledger);
    assert_draft_equal(&before, &l.client.snapshot());
    f.d.key(&f.h, &mut l, 4, 3);
    b.key(&f.h, &mut bl, 5, 3);
    assert!(l.client.snapshot().bytes.ends_with(b"a"));
    assert!(bl.client.snapshot().bytes.ends_with(b"b"));
    assert_eq!(selected(&f.h, &bl, 3).1, b"other\n");
    let mut r = Record::new(name);
    r.frame(render(&f.h, &f.d, &mut l, 3));
    r.note("trusted_route","{\"generated_input_focus_chrome\":true,\"privileged_denials\":10,\"foreign_valid_a_endpoint_denied\":true,\"foreign_input_denied\":true,\"both_consumers_usable\":true}");
    r.finish(&f.c, 0);
}

#[test]
fn ui1_1_focus_epoch_reset_overflow_and_detach() {
    let name = "ui1_1_focus_epoch_reset_overflow_and_detach";
    let mut f = Fixture::new(b"note=old\n");
    let mut l = f.launch(1);
    let epoch = observe(&f.h, &l, 1).focus_epoch;
    let before = l.client.snapshot();
    f.d.send(&f.h, 40, 1, 2); // held Enter queued for old focus; no client poll
    let assigned: focus::AssignReply = call(
        &f.h,
        &f.d.s.chrome,
        focus::Assign {
            request_id: 25,
            session_id: f.d.s.session_id,
            session_generation: f.d.s.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
            surface_id: l.surface.surface_id,
            surface_generation: l.surface.surface_generation,
        },
        2,
    );
    assert_eq!(assigned.status, 0);
    assert!(assigned.focus_epoch > epoch);
    deny(
        &f.h,
        &f.d.s.input,
        f.d.request(40, 1, 0),
        8,
        2,
        Status::Invalid,
    );
    f.d.send(&f.h, 40, 2, 2); // legitimate release accepted, suppressed in new focus
    let empty: focus::PollKeysReply = call(
        &f.h,
        &l.bindings.focus_read,
        poll_request(&l, assigned.focus_epoch),
        2,
    );
    assert_eq!(empty.status, Status::NotReady as u32);
    assert_eq!(
        (
            empty.sequence,
            empty.usage,
            empty.phase,
            empty.modifiers,
            empty.focus_epoch
        ),
        (0, 0, 0, 0, 0)
    );
    assert_draft_equal(&before, &l.client.snapshot());
    for _ in 0..32 {
        f.d.send(&f.h, 4, 1, 3);
        f.d.send(&f.h, 4, 2, 3);
    }
    let overflow_sequence = f.d.next;
    f.d.send(&f.h, 5, 1, 3);
    let after = f.d.next;
    assert_eq!(after, overflow_sequence + 1);
    deny(
        &f.h,
        &f.d.s.input,
        f.d.request(4, 1, 0),
        8,
        3,
        Status::NotReady,
    );
    let reset: focus::PollKeysReply = call(
        &f.h,
        &l.bindings.focus_read,
        poll_request(&l, assigned.focus_epoch),
        3,
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
    assert_draft_equal(&before, &l.client.snapshot());
    deny(
        &f.h,
        &f.d.s.input,
        f.d.request(5, 2, 0),
        8,
        3,
        Status::Invalid,
    );
    let old = f.d.s.input.clone();
    let stale = f.d.request(4, 1, 0);
    f.c.detach_keyboard(f.d.s.session_id, 4).unwrap();
    deny(&f.h, &old, stale, 8, 4, Status::Stale);
    let (input, device) = f.c.reattach_keyboard(f.d.s.session_id, 4).unwrap();
    assert_ne!(input.handle.pack(), old.handle.pack());
    f.d.s.input = input;
    f.d.s.device_generation = device;
    f.d.next = 1;
    f.d.mods = 0;
    f.d.attach(&f.h, 4);
    f.d.key(&f.h, &mut l, 4, 4);
    assert!(l.client.snapshot().bytes.ends_with(b"a"));
    close(&f.h, &f.d.s, &l.boot, 5);
    let preview = f.d.preview(&f.h, &f.c, 5);
    assert_eq!(preview.status, 0);
    let reset_sequence = f.d.next;
    f.d.send(&f.h, 0, 3, 5);
    let after_reset = f.c.evidence().unwrap();
    // No launcher chord repairs the route: Reset itself must leave it usable.
    f.d.stroke(&f.h, 40, 5);
    let selected_preview = latest::<editor::PrepareLaunchReply>(&f.c.evidence().unwrap());
    assert_eq!(selected_preview.status, 0);
    assert_ne!(selected_preview.plan_id, preview.plan_id);
    assert_eq!(
        f.c.evidence().unwrap().ledger.instances,
        after_reset.ledger.instances
    );
    f.d.stroke(&f.h, 40, 5);
    let confirmed = latest::<editor::ConfirmLaunchReply>(&f.c.evidence().unwrap());
    assert_eq!(confirmed.status, 0);
    assert_ne!(confirmed.instance_id, l.boot.instance_id);
    let recovered = EditorClient::new(
        f.h.clone(),
        f.c.editor_bindings(confirmed.instance_id).unwrap(),
        5,
    )
    .unwrap();
    assert_eq!(recovered.snapshot().bytes, b"note=old\n");
    let mut r = Record::new(name);
    r.note("reset",&format!("{{\"old_focus_epoch\":{epoch},\"new_focus_epoch\":{},\"overflow_sequence\":{overflow_sequence},\"successor\":{after},\"old_held_release_suppressed\":true,\"fresh_attach_usable\":true}}",assigned.focus_epoch));
    r.note("preview_reset",&format!("{{\"sequence\":{reset_sequence},\"invalidated_plan_id\":{},\"fresh_plan_id\":{},\"confirmed_instance_id\":{},\"direct_selection_after_reset\":true,\"no_launcher_repair\":true}}",preview.plan_id,selected_preview.plan_id,confirmed.instance_id));
    r.finish(&f.c, 0);
}

#[test]
fn ui1_1_surface_freeze_alias_quota_and_chrome_clip() {
    let name = "ui1_1_surface_freeze_alias_quota_and_chrome_clip";
    let mut f = Fixture::new(b"note=old\n");
    let mut l = f.launch(1);
    let before = render(&f.h, &f.d, &mut l, 1);
    let seq = before.sequence + 1;
    let acquired: surface::AcquireReply = call(
        &f.h,
        &l.bindings.surface,
        surface::Acquire {
            request_id: 26,
            surface_id: l.surface.surface_id,
            surface_generation: l.surface.surface_generation,
            buffer_index: 0,
            reserved: 0,
        },
        2,
    );
    assert_eq!(acquired.status, 0);
    assert_eq!(acquired.byte_len as usize, FRAME_BYTES);
    assert_ne!(acquired.mapping_generation, 0);
    let d = desc(
        acquired.buffer_shm,
        ObjectKind::SurfaceBuffer,
        acquired.byte_len,
    );
    let lease =
        f.h.write_lease(&l.bindings.surface.peer, &d, acquired.mapping_generation, 2)
            .unwrap();
    let alias = lease.clone();
    let magenta = [255, 0, 255, 255].repeat(640 * 480);
    lease.copy_from(0, &magenta, 2).unwrap();
    let present = surface::Present {
        request_id: 27,
        surface_id: l.surface.surface_id,
        surface_generation: l.surface.surface_generation,
        mapping_generation: acquired.mapping_generation,
        sequence: seq,
        buffer_index: 0,
        reserved: 0,
    };
    let p: surface::PresentReply = call(&f.h, &l.bindings.surface, present, 2);
    assert_eq!(p.status, 0);
    assert!(matches!(lease.copy_from(0, &[0; 4], 2), Err(Status::Stale)));
    assert!(matches!(alias.copy_from(0, &[0; 4], 2), Err(Status::Stale)));
    let frozen = f.h.read_lease(&f.d.s.compositor.peer, &d, 2).unwrap();
    let mut frozen_bytes = vec![0; FRAME_BYTES];
    frozen.copy_into(0, &mut frozen_bytes, 2).unwrap();
    assert_eq!(frozen_bytes, magenta);
    deny(&f.h, &l.bindings.surface, present, 16, 2, Status::Stale);
    let mut invalid = present;
    invalid.buffer_index = 2;
    deny(&f.h, &l.bindings.surface, invalid, 16, 2, Status::Invalid);
    deny(
        &f.h,
        &l.bindings.surface,
        surface::Create {
            request_id: 28,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
            width: 640,
            height: 480,
            format: 1,
            reserved: 0,
        },
        40,
        2,
        Status::Exhausted,
    );
    assert!(matches!(
        f.h.compose_next(&l.bindings.surface, 2),
        Err(Status::Denied)
    ));
    let out = f.h.compose_next(&f.d.s.compositor, 2).unwrap();
    assert_eq!(crop(&out), magenta);
    let mut consumed_copy = [0x6d; 4];
    assert!(matches!(
        frozen.copy_into(0, &mut consumed_copy, 2),
        Err(Status::NotReady)
    ));
    assert_eq!(consumed_copy, [0x6d; 4]);
    assert_eq!(&before.bgra[..48 * 640 * 4], &out.bgra[..48 * 640 * 4]);
    assert_eq!(&before.bgra[528 * 640 * 4..], &out.bgra[528 * 640 * 4..]);
    assert_volatile_label(&out);
    let fresh: surface::AcquireReply = call(
        &f.h,
        &l.bindings.surface,
        surface::Acquire {
            request_id: 29,
            surface_id: l.surface.surface_id,
            surface_generation: l.surface.surface_generation,
            buffer_index: 0,
            reserved: 0,
        },
        3,
    );
    assert_eq!(fresh.status, 0);
    assert!(fresh.mapping_generation > acquired.mapping_generation);
    f.h.write_lease(
        &l.bindings.surface.peer,
        &desc(fresh.buffer_shm, ObjectKind::SurfaceBuffer, fresh.byte_len),
        fresh.mapping_generation,
        3,
    )
    .unwrap()
    .copy_from(0, &[0, 0, 0, 255], 3)
    .unwrap();
    assert!(matches!(alias.copy_from(0, &[0; 4], 3), Err(Status::Stale)));
    let mut r = Record::new(name);
    r.frame(before);
    r.frame(out);
    r.note("surface",&format!("{{\"frozen_sequence\":{seq},\"old_mapping\":{},\"fresh_mapping\":{},\"retained_alias_denied\":true,\"chrome_pixels_unchanged\":true}}",acquired.mapping_generation,fresh.mapping_generation));
    r.note("frozen_consumer",&format!("{{\"compositor_copy_ok\":true,\"byte_len\":{},\"sha256\":\"{}\",\"write_aliases_denied\":true,\"consumed_read_status\":6,\"consumed_read_no_bytes\":true}}",frozen_bytes.len(),hash(&frozen_bytes)));
    r.finish(&f.c, 0);
}

#[test]
fn ui1_1_wire_schema_descriptor_and_identity_fail_closed() {
    let name = "ui1_1_wire_schema_descriptor_and_identity_fail_closed";
    let mut f = Fixture::new(b"note=old\n");
    let mut l = f.launch(1);
    let obs = observe(&f.h, &l, 1);
    let probes = [
        (
            f.d.s.input.clone(),
            f.d.request(4, 1, 0).encode(f.d.s.input.handle).unwrap(),
            8usize,
        ),
        (
            l.bindings.focus_read.clone(),
            poll_request(&l, obs.focus_epoch)
                .encode(l.bindings.focus_read.handle)
                .unwrap(),
            36,
        ),
        (
            l.bindings.surface.clone(),
            surface::Acquire {
                request_id: 30,
                surface_id: l.surface.surface_id,
                surface_generation: l.surface.surface_generation,
                buffer_index: 0,
                reserved: 0,
            }
            .encode(l.bindings.surface.handle)
            .unwrap(),
            24,
        ),
        (
            l.bindings.self_status.clone(),
            status_request(&l, &f.d.s)
                .encode(l.bindings.self_status.handle)
                .unwrap(),
            44,
        ),
        (
            l.bindings.artifact.clone(),
            artifact::ReadSelected {
                request_id: 31,
                session_id: l.boot.session_id,
                session_generation: l.boot.session_generation,
                instance_id: l.boot.instance_id,
                instance_generation: l.boot.instance_generation,
            }
            .encode(l.bindings.artifact.handle)
            .unwrap(),
            36,
        ),
    ];
    let ledger = f.c.evidence().unwrap().ledger;
    let mut raw_probes = Vec::new();
    for (e, q, offset) in &probes {
        let good = encode_envelope_wire(q).unwrap();
        assert_eq!(
            encode_envelope_wire(&decode_envelope_wire(&good).unwrap()).unwrap(),
            good
        );
        for field in ["protocol", "operation", "length", "tail", "handle", "pad"] {
            let mut raw = good;
            match field {
                "protocol" => raw[..4].copy_from_slice(&999u32.to_le_bytes()),
                "operation" => raw[4..8].copy_from_slice(&999u32.to_le_bytes()),
                "length" => raw[16..20].copy_from_slice(&(q.payload_len - 1).to_le_bytes()),
                "tail" => raw[20 + q.payload_len as usize] = 1,
                "handle" => raw[14] |= 1,
                "pad" => raw[84] = 1,
                _ => unreachable!(),
            }
            let error = match decode_envelope_wire(&raw) {
                Err(error) => error,
                Ok(_) => panic!("{field} accepted"),
            };
            raw_probes.push(format!(
                "{{\"protocol\":{},\"field\":\"{field}\",\"request_hex\":\"{}\",\"status\":{}}}",
                q.protocol,
                hex(&raw),
                error as u32
            ));
        }
        let mut short = *q;
        short.payload_len -= 1;
        zero_reply(&f.h.dispatch(&e.peer, &short, 1), *offset, Status::Invalid);
        let mut future = *q;
        future.protocol = 999;
        zero_reply(
            &f.h.dispatch(&e.peer, &future, 1),
            *offset,
            Status::Unsupported,
        );
    }
    let reserved = f.d.request(4, 1, 0);
    let mut bad = reserved;
    bad.reserved = 1;
    assert!(bad.encode(f.d.s.input.handle).is_err());
    let response = editor::GetStatusReply {
        request_id: 32,
        instance_id: 0,
        instance_generation: 0,
        focus_epoch: 0,
        service_epoch: 0,
        state: 0,
        status: 0,
    };
    zero_reply(
        &f.h.dispatch(
            &l.bindings.self_status.peer,
            &response.encode(l.bindings.self_status.handle).unwrap(),
            1,
        ),
        44,
        Status::Unsupported,
    );
    assert_ledger_equal(&ledger, &f.c.evidence().unwrap().ledger);
    let source =
        f.h.draft_source(&l.bindings.artifact.peer, 1, b"new\n", 1)
            .unwrap();
    let wrong = ObjectDescriptor {
        handle: source.handle,
        kind: ObjectKind::Grants,
        object_generation: source.object_generation,
        byte_len: source.byte_len,
    };
    assert!(matches!(
        f.h.read_lease(&l.bindings.self_status.peer, &wrong, 1),
        Err(Status::Denied)
    ));
    assert!(matches!(
        f.h.write_lease(&l.bindings.artifact.peer, &source, 1, 1),
        Err(Status::Invalid)
    ));
    let writer =
        f.h.write_lease(&l.bindings.artifact.peer, &source, 0, 1)
            .unwrap();
    assert!(matches!(
        writer.copy_from(u32::MAX, &[1], 1),
        Err(Status::Invalid)
    ));
    let mut wrong_generation = source;
    wrong_generation.object_generation += 1;
    assert!(matches!(
        f.h.write_lease(&l.bindings.artifact.peer, &wrong_generation, 0, 1),
        Err(Status::Stale)
    ));
    for (offset, corruption) in [
        (0, 2u32.to_le_bytes().to_vec()),
        (24, vec![0; 32]),
        (56, u32::MAX.to_le_bytes().to_vec()),
    ] {
        // Independent fresh operation/source per malformed object; a cached
        // rejection of an earlier operation cannot satisfy another validation.
        let q = commit_request(&f.h, &l, b"new\n", 1, 1);
        let d = desc(q.source_shm, ObjectKind::DraftSource, q.byte_len);
        let write =
            f.h.write_lease(&l.bindings.artifact.peer, &d, 0, 1)
                .unwrap();
        write.copy_from(offset, &corruption, 1).unwrap();
        deny(&f.h, &l.bindings.artifact, q, 32, 1, Status::Invalid);
        assert_eq!(selected(&f.h, &l, 1), (1, b"note=old\n".to_vec()));
    }
    let current: artifact::ReadSelectedReply = call(
        &f.h,
        &l.bindings.artifact,
        artifact::ReadSelected {
            request_id: 34,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        1,
    );
    let impossible = desc(current.data_shm, ObjectKind::SelectedText, u32::MAX);
    assert!(matches!(
        f.h.read_lease(&l.bindings.artifact.peer, &impossible, 1),
        Err(Status::Invalid)
    ));
    f.d.key(&f.h, &mut l, 4, 2);
    assert!(l.client.snapshot().bytes.ends_with(b"a"));
    let mut r = Record::new(name);
    r.frame(render(&f.h, &f.d, &mut l, 2));
    r.note("raw_codec_probes", &format!("[{}]", raw_probes.join(",")));
    r.note("wire","{\"protocols\":[802,832,833,352,368],\"raw_negative_fields\":6,\"wrong_direction_denied\":true,\"schema_hash_range_generation_checked\":true,\"selected_unchanged\":true}");
    r.finish(&f.c, 0);
}

#[test]
fn ui1_1_editor_grants_are_exact_and_preview_single_use() {
    let name = "ui1_1_editor_grants_are_exact_and_preview_single_use";
    let mut f = Fixture::new(b"note=old\n");
    let l = f.launch(1);
    let grants = read(
        &f.h,
        &l.bindings.self_status,
        &desc(l.boot.grants_shm, ObjectKind::Grants, l.boot.grants_len),
        1,
    );
    assert_eq!(grants.len(), 464);
    assert_eq!(
        (
            u32_at(&grants, 0),
            u32_at(&grants, 4),
            u32_at(&grants, 8),
            u32_at(&grants, 12)
        ),
        (1, 464, 63, 4)
    );
    assert_eq!(u64_at(&grants, 32), l.boot.instance_id);
    assert_eq!(&grants[112..144], &APP);
    assert_eq!(&grants[144..176], &MANIFEST);
    let endpoints = [
        &l.bindings.self_status,
        &l.bindings.focus_read,
        &l.bindings.surface,
        &l.bindings.artifact,
    ];
    for (i, mask) in [1, 1, 15, 7].into_iter().enumerate() {
        let o = 240 + i * 56;
        assert_eq!(u64_at(&grants, o), endpoints[i].handle.pack());
        assert_ne!(u64_at(&grants, o + 8), 0);
        assert_eq!(u32_at(&grants, o + 40), i as u32 + 1);
        assert_eq!(u32_at(&grants, o + 44), mask);
        assert_eq!(u32_at(&grants, o + 48), 1);
        assert_eq!(u32_at(&grants, o + 52), 0);
    }
    let p = latest::<editor::PrepareLaunchReply>(&f.c.evidence().unwrap());
    deny(
        &f.h,
        &f.d.s.chrome,
        editor::ConfirmLaunch {
            request_id: 35,
            session_id: f.d.s.session_id,
            session_generation: f.d.s.session_generation,
            plan_id: p.plan_id,
            preview_revision: p.preview_revision,
        },
        44,
        1,
        Status::Stale,
    );
    assert_eq!(selected(&f.h, &l, 1).0, 1);
    let mut r = Record::new(name);
    r.note(
        "grants",
        &format!(
            "{{\"profile\":63,\"masks\":[1,1,15,7],\"grants_hex\":\"{}\",\"replay_denied\":true}}",
            hex(&grants)
        ),
    );
    r.finish(&f.c, 0);
    for (i, change) in ["policy", "application", "artifact", "expiry"]
        .into_iter()
        .enumerate()
    {
        let mut x = Fixture::ttl(b"note=old\n", 100, 1_000);
        let p = x.d.preview(&x.h, &x.c, 1);
        let q = editor::ConfirmLaunch {
            request_id: 36,
            session_id: x.d.s.session_id,
            session_generation: x.d.s.session_generation,
            plan_id: p.plan_id,
            preview_revision: p.preview_revision,
        };
        deny(&x.h, &x.d.s.chrome, q, 44, 1, Status::Denied); // endpoint alone lacks fresh Enter
        match change {
            "policy" => x.c.set_policy_revision(x.d.s.session_id, 2).unwrap(),
            "application" => {
                x.c.set_application_identity(x.d.s.session_id, [0x42; 32], [0x4e; 32])
                    .unwrap()
            }
            "artifact" => {
                x.c.set_selected_identity(x.d.s.session_id, b"changed\n", 2, 2)
                    .unwrap()
            }
            "expiry" => x.h.maintenance(101).unwrap(),
            _ => unreachable!(),
        }
        deny(
            &x.h,
            &x.d.s.chrome,
            q,
            44,
            if change == "expiry" { 101 } else { 2 },
            Status::Stale,
        );
        assert_eq!(x.c.evidence().unwrap().ledger.instances, 0);
        x.c.set_application_identity(x.d.s.session_id, APP, MANIFEST)
            .unwrap();
        let fresh = x.launch(if change == "expiry" { 102 } else { 3 });
        assert_ne!(fresh.boot.instance_id, 0);
        let mut rr = Record::new(name);
        rr.note(
            "stale_preview",
            &format!("{{\"cause\":\"{change}\",\"no_instance_before_fresh_approval\":true}}"),
        );
        rr.finish(&x.c, i as u32 + 1);
    }
}

#[test]
fn ui1_1_volatile_save_and_reopen_are_explicitly_labeled() {
    let name = "ui1_1_volatile_save_and_reopen_are_explicitly_labeled";
    let mut f = Fixture::new(b"note=old\n");
    let mut l = f.launch(1);
    f.d.ctrl(&f.h, &mut l, 4, 2);
    f.d.text(&f.h, &mut l, b"note=new\n", 2);
    f.d.ctrl(&f.h, &mut l, 22, 3);
    assert!(matches!(
        l.client.snapshot().save,
        SaveState::Saved { revision: 2 }
    ));
    assert_eq!(selected(&f.h, &l, 3), (2, b"note=new\n".to_vec()));
    let committed = latest::<artifact::CommitReply>(&f.c.evidence().unwrap());
    assert_eq!(committed.status, 0);
    let bytes = receipt(
        &f.h,
        &l.bindings.artifact,
        &f.d.s,
        committed.operation_id,
        3,
    );
    assert_eq!(
        (u32_at(&bytes, 8), u64_at(&bytes, 80), u64_at(&bytes, 88)),
        (0, 1, 2)
    );
    assert_eq!(&bytes[144..176], &sha2::Sha256::digest(b"note=new\n")[..]);
    let frame = render(&f.h, &f.d, &mut l, 3);
    assert_volatile_label(&frame);
    // Ctrl+Q uses trusted chrome routing; retained client is not invoked after retirement.
    f.d.send(&f.h, 224, 1, 4);
    f.d.stroke(&f.h, 20, 4);
    f.d.send(&f.h, 224, 2, 4);
    deny(
        &f.h,
        &l.bindings.self_status,
        status_request(&l, &f.d.s),
        44,
        4,
        Status::Stale,
    );
    let mut fresh = f.launch(5);
    assert_eq!(fresh.client.snapshot().bytes, b"note=new\n");
    assert_eq!(fresh.client.snapshot().confirmed_revision, 2);
    let mut r = Record::new(name);
    r.frame(frame);
    r.frame(render(&f.h, &f.d, &mut fresh, 5));
    r.note("save",&format!("{{\"operation_id\":{},\"revision\":2,\"receipt_hex\":\"{}\",\"volatile_label\":true,\"reopen_verified\":true}}",committed.operation_id,hex(&bytes)));
    r.finish(&f.c, 0);
    let mut x = Fixture::new(b"note=old\n");
    let mut l = x.launch(1);
    x.d.ctrl(&x.h, &mut l, 4, 2);
    x.d.text(&x.h, &mut l, b"note=new\n", 2);
    x.d.event(&x.h, &mut l, 224, 1, 2);
    let barrier =
        x.c.pause_next(x.d.s.session_id, PausePoint::AfterCommitPermit)
            .unwrap();
    x.d.send(&x.h, 22, 1, 2); // Ctrl+S press consumed by the actual Rust editor.
    let mut client = l.client;
    let join = thread::spawn(move || {
        let result = client.step(2);
        (client, result)
    });
    let entered = barrier.wait_until_entered(2_000).unwrap();
    assert!(entered.entered && entered.permit_issued && entered.operation_id > 0);
    let operation_id = entered.operation_id;
    let (client, result) = join.join().unwrap();
    l.client = client;
    assert!(matches!(result, Ok(_) | Err(Status::Unknown)));
    assert!(
        matches!(l.client.snapshot().save,SaveState::Unknown{operation_id:id} if id==operation_id)
    );
    let result = latest::<artifact::CommitReply>(&x.c.evidence().unwrap());
    assert_eq!(
        (
            result.status,
            result.operation_id,
            result.revision,
            result.receipt_shm,
            result.receipt_len
        ),
        (10, operation_id, 0, 0, 0)
    );
    x.d.event(&x.h, &mut l, 22, 2, 2);
    x.d.event(&x.h, &mut l, 224, 2, 2);
    x.d.ctrl(&x.h, &mut l, 22, 2); // A second human save must not dispatch/allocate again.
    x.d.key(&x.h, &mut l, 4, 2); // Local editing continues while the original save is unknown.
    assert_eq!(l.client.snapshot().bytes, b"note=new\na");
    assert!(
        matches!(l.client.snapshot().save,SaveState::Unknown{operation_id:id} if id==operation_id)
    );
    let before = x.c.evidence().unwrap();
    assert_eq!(
        (
            before.mutation_dispatch_count,
            before.permit_count,
            before.transition_count
        ),
        (1, 1, 0)
    );
    assert_eq!(
        before
            .ledger
            .operations_per_object
            .iter()
            .map(|(_, n)| *n)
            .sum::<u32>(),
        1,
        "repeated Ctrl+S must not reserve another operation"
    );
    for operation in [3, 5] {
        assert_eq!(
            before
                .exchanges
                .iter()
                .filter(|(q, _)| q.protocol == 368 && q.msg_type == operation)
                .count(),
            1,
            "actual client dispatch must contain exactly one AllocateSaveId and Commit"
        );
    }
    let still: artifact::SaveStatusReply = call(
        &x.h,
        &l.bindings.artifact,
        artifact::SaveStatus {
            request_id: 37,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            operation_id,
        },
        2,
    );
    assert_eq!(
        (
            still.status,
            still.operation_id,
            still.revision,
            still.receipt_shm
        ),
        (10, operation_id, 0, 0)
    );
    let mut b = Driver::register(&x.h, &x.c, 2, b"other\n", 2);
    let bl = b.launch(&x.h, &x.c, 2);
    deny(
        &x.h,
        &bl.bindings.artifact,
        artifact::SaveStatus {
            request_id: 53,
            session_id: bl.boot.session_id,
            session_generation: bl.boot.session_generation,
            operation_id,
        },
        32,
        2,
        Status::Denied,
    );
    deny(
        &x.h,
        &l.bindings.artifact,
        artifact::SaveStatus {
            request_id: 54,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            operation_id: u64::MAX,
        },
        32,
        2,
        Status::Denied,
    );
    assert_eq!(selected(&x.h, &bl, 2).1, b"other\n");
    x.c.release(&barrier).unwrap();
    let settled = barrier.wait_until_settled(2_000).unwrap();
    assert!(settled.settled);
    let bytes = receipt(&x.h, &l.bindings.artifact, &x.d.s, operation_id, 3);
    assert_eq!(u64_at(&bytes, 88), 2);
    match l.client.step(3) {
        Ok(_) | Err(Status::NotReady) => {}
        Err(e) => panic!("receipt reconciliation: {e:?}"),
    }
    assert_eq!(l.client.snapshot().confirmed_revision, 2);
    assert!(matches!(l.client.snapshot().save, SaveState::Unsaved));
    assert_eq!(l.client.snapshot().bytes, b"note=new\na");
    assert_eq!(selected(&x.h, &l, 3), (2, b"note=new\n".to_vec()));
    let after = x.c.evidence().unwrap();
    assert_eq!(
        (
            after.mutation_dispatch_count,
            after.permit_count,
            after.transition_count
        ),
        (1, 1, 1)
    );
    let mut rr = Record::new(name);
    barrier_note(&mut rr, "after_permit_entered", &entered);
    barrier_note(&mut rr, "after_permit_settled", &settled);
    rr.note("unknown_ui","{\"actual_ctrl_s\":true,\"unknown_state\":true,\"repeated_save_no_replay\":true,\"newer_draft_after_receipt\":\"unsaved\",\"confirmed_revision\":2}");
    rr.note("unknown_reconciliation",&format!("{{\"operation_id\":{},\"dispatches\":1,\"permits\":1,\"transitions\":1,\"receipt_hex\":\"{}\"}}",operation_id,hex(&bytes)));
    rr.finish(&x.c, 1);
}

#[test]
fn ui1_1_revoke_paused_key_frame_and_save() {
    let name = "ui1_1_revoke_paused_key_frame_and_save";
    for (i, point) in [PausePoint::BeforeKeyDelivery, PausePoint::BeforePresent]
        .into_iter()
        .enumerate()
    {
        let mut f = Fixture::new(b"note=old\n");
        let mut l = f.launch(1);
        let mut b = Driver::register(&f.h, &f.c, 2, b"other\n", 1);
        let mut bl = b.launch(&f.h, &f.c, 1);
        let before = l.client.snapshot();
        let prior_frame = render(&f.h, &f.d, &mut l, 1);
        let boot = l.boot;
        let endpoint = l.bindings.self_status.clone();
        let mut client = l.client;
        let token = f.c.pause_next(f.d.s.session_id, point).unwrap();
        if i == 0 {
            f.d.send(&f.h, 4, 1, 2);
        }
        let join = thread::spawn(move || {
            let result = if i == 0 {
                client.step(2).map(|_| ())
            } else {
                client.render(2).map(|_| ())
            };
            (client, result)
        });
        let entered = token.wait_until_entered(2_000).unwrap();
        assert!(entered.entered && !entered.permit_issued);
        revoke(&f.h, &f.d.s, &boot, 3);
        f.c.release(&token).unwrap();
        let settled = token.wait_until_settled(2_000).unwrap();
        assert!(settled.settled);
        let (client, result) = join.join().unwrap();
        assert!(matches!(result, Err(Status::Stale)));
        assert_draft_equal(&before, &client.snapshot());
        deny(
            &f.h,
            &f.d.s.chrome,
            focus::Assign {
                request_id: 138,
                session_id: boot.session_id,
                session_generation: boot.session_generation,
                instance_id: boot.instance_id,
                instance_generation: boot.instance_generation,
                surface_id: l.surface.surface_id,
                surface_generation: l.surface.surface_generation,
            },
            24,
            3,
            Status::Stale,
        );
        deny(
            &f.h,
            &endpoint,
            editor::GetStatus {
                request_id: 38,
                session_id: boot.session_id,
                session_generation: boot.session_generation,
                instance_id: boot.instance_id,
                instance_generation: boot.instance_generation,
            },
            44,
            3,
            Status::Stale,
        );
        assert_eq!(selected(&f.h, &bl, 3).1, b"other\n");
        let mut r = Record::new(name);
        r.frame(prior_frame);
        match f.h.compose_next(&f.d.s.compositor, 3) {
            Ok(frame) => {
                assert_eq!(
                    (frame.session_id, frame.instance_id),
                    (f.d.s.session_id, boot.instance_id)
                );
                assert_eq!((frame.focus_epoch, frame.sequence), (0, 0));
                assert_eq!(crop(&frame), vec![255; FRAME_BYTES]);
                r.frame(frame);
            }
            Err(Status::NotReady) => {}
            Err(e) => panic!("late frame probe: {e:?}"),
        }
        assert!(matches!(
            f.h.compose_next(&f.d.s.compositor, 3),
            Err(Status::NotReady)
        ));
        b.key(&f.h, &mut bl, 4, 3);
        assert!(bl.client.snapshot().bytes.ends_with(b"a"));
        r.frame(render(&f.h, &b, &mut bl, 3));
        barrier_note(&mut r, "before_effect_entered", &entered);
        barrier_note(&mut r, "before_effect_settled", &settled);
        r.note("late_frame_probe","{\"a_app_frame_absent\":true,\"repeated_compose_not_ready\":true,\"b_real_frame_ok\":true}");
        r.note(
            "retirement",
            &format!(
                "{{\"effect\":\"{}\",\"no_late_effect\":true,\"other_object_read_ok\":true,\"terminal_focus_assign_denied\":true}}",
                if i == 0 { "key" } else { "frame" }
            ),
        );
        r.finish(&f.c, i as u32);
    }
    for (j, after_permit) in [false, true].into_iter().enumerate() {
        let mut f = Fixture::new(b"note=old\n");
        let l = f.launch(1);
        let mut b = Driver::register(&f.h, &f.c, 2, b"other\n", 1);
        let bl = b.launch(&f.h, &f.c, 1);
        let q = commit_request(&f.h, &l, b"note=new\n", 1, 2);
        let token =
            f.c.pause_next(
                f.d.s.session_id,
                if after_permit {
                    PausePoint::AfterCommitPermit
                } else {
                    PausePoint::BeforeCommitPermit
                },
            )
            .unwrap();
        let h = f.h.clone();
        let e = l.bindings.artifact.clone();
        let join = thread::spawn(move || call::<_, artifact::CommitReply>(&h, &e, q, 2));
        let entered = token.wait_until_entered(2_000).unwrap();
        assert!(entered.entered);
        assert_eq!(entered.permit_issued, after_permit);
        revoke(&f.h, &f.d.s, &l.boot, 3);
        let mut r = Record::new(name);
        if after_permit {
            let frame = f.h.compose_next(&f.d.s.compositor, 3).unwrap();
            assert_eq!(
                (
                    frame.session_id,
                    frame.instance_id,
                    frame.focus_epoch,
                    frame.sequence
                ),
                (f.d.s.session_id, l.boot.instance_id, 0, 0)
            );
            assert_eq!(crop(&frame), vec![255; FRAME_BYTES]);
            assert_label(&frame, b"SAVE UNKNOWN");
            assert_label(&frame, b"DRAFT LOST");
            assert_volatile_label(&frame);
            r.frame(frame);
            r.note("unknown_recovery_pixels","{\"save_unknown\":true,\"draft_lost\":true,\"blank_app_crop\":true,\"no_guessed_outcome\":true}");
        }
        deny(
            &f.h,
            &l.bindings.artifact,
            artifact::SaveStatus {
                request_id: 39,
                session_id: l.boot.session_id,
                session_generation: l.boot.session_generation,
                operation_id: q.operation_id,
            },
            32,
            3,
            Status::Stale,
        );
        assert_eq!(selected(&f.h, &bl, 3).1, b"other\n");
        f.c.release(&token).unwrap();
        let settled = token.wait_until_settled(2_000).unwrap();
        assert!(settled.settled);
        let reply = join.join().unwrap();
        let e = f.c.evidence().unwrap();
        if after_permit {
            assert_eq!(
                (
                    reply.status,
                    reply.operation_id,
                    reply.revision,
                    reply.receipt_shm
                ),
                (10, q.operation_id, 0, 0)
            );
            assert_eq!((e.permit_count, e.transition_count), (1, 1));
            let recovery =
                f.c.recovery_receipt(f.d.s.session_id, q.operation_id)
                    .unwrap();
            let receipt = receipt(&f.h, &recovery, &f.d.s, q.operation_id, 4);
            assert_eq!(u64_at(&receipt, 88), 2);
        } else {
            assert_eq!(reply.status, Status::Stale as u32);
            assert_eq!(
                (
                    reply.operation_id,
                    reply.revision,
                    reply.receipt_shm,
                    reply.receipt_len
                ),
                (0, 0, 0, 0)
            );
            assert_eq!((e.permit_count, e.transition_count), (0, 0));
        }
        let fresh = f.launch(5);
        assert_eq!(
            selected(&f.h, &fresh, 5),
            if after_permit {
                (2, b"note=new\n".to_vec())
            } else {
                (1, b"note=old\n".to_vec())
            }
        );
        barrier_note(&mut r, "save_entered", &entered);
        barrier_note(&mut r, "save_settled", &settled);
        r.note("save_retirement",&format!("{{\"after_permit\":{after_permit},\"original_operation_id\":{},\"no_replay\":true,\"transitions\":{}}}",q.operation_id,e.transition_count));
        r.finish(&f.c, j as u32 + 2);
    }
}

#[test]
fn ui1_1_fault_restart_discards_unsaved_draft_and_old_grants() {
    let name = "ui1_1_fault_restart_discards_unsaved_draft_and_old_grants";
    let mut f = Fixture::new(b"note=old\n");
    let mut l = f.launch(1);
    f.d.ctrl(&f.h, &mut l, 4, 2);
    f.d.text(&f.h, &mut l, b"unsaved\n", 2);
    assert_eq!(l.client.snapshot().bytes, b"unsaved\n");
    f.c.fault_editor(l.boot.instance_id, 3).unwrap();
    let recovery =
        f.h.compose_next(&f.d.s.compositor, 3)
            .expect("actual trusted recovery output");
    assert_eq!(
        (recovery.session_id, recovery.instance_id),
        (f.d.s.session_id, l.boot.instance_id)
    );
    assert_eq!((recovery.focus_epoch, recovery.sequence), (0, 0));
    assert_eq!(crop(&recovery), vec![255; FRAME_BYTES]);
    assert_label(&recovery, b"DRAFT LOST");
    assert_volatile_label(&recovery);
    assert!(matches!(
        f.h.compose_next(&f.d.s.compositor, 3),
        Err(Status::NotReady)
    ));
    deny(
        &f.h,
        &l.bindings.self_status,
        status_request(&l, &f.d.s),
        44,
        3,
        Status::Stale,
    );
    deny(
        &f.h,
        &l.bindings.artifact,
        artifact::ReadSelected {
            request_id: 55,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        36,
        1,
        Status::Stale,
    );
    deny(
        &f.h,
        &l.bindings.self_status,
        editor::ObserveFocus {
            request_id: 56,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        24,
        1,
        Status::Stale,
    );
    assert!(matches!(
        f.h.read_lease(
            &l.bindings.self_status.peer,
            &desc(l.boot.grants_shm, ObjectKind::Grants, l.boot.grants_len),
            1
        ),
        Err(Status::Stale)
    ));
    let status: editor::GetStatusReply = call(&f.h, &f.d.s.chrome, status_request(&l, &f.d.s), 3);
    assert_eq!((status.status, status.state), (0, 4));
    assert_eq!(f.c.evidence().unwrap().ledger.instances, 1);
    deny(
        &f.h,
        &f.d.s.chrome,
        focus::Assign {
            request_id: 139,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
            surface_id: l.surface.surface_id,
            surface_generation: l.surface.surface_generation,
        },
        24,
        3,
        Status::Stale,
    );
    f.d.send(&f.h, 224, 1, 4);
    f.d.send(&f.h, 21, 1, 4);
    f.d.send(&f.h, 224, 2, 4);
    let p = latest::<editor::PrepareRestartReply>(&f.c.evidence().unwrap());
    assert_eq!(p.status, 0);
    let held_selector = f.c.evidence().unwrap();
    let confirmations = held_selector
        .exchanges
        .iter()
        .filter(|(q, _)| q.protocol == 352 && q.msg_type == 5)
        .count();
    // Ctrl is released, but R remains physically held. This fresh Enter cannot approve.
    f.d.stroke(&f.h, 40, 4);
    let premature = f.c.evidence().unwrap();
    assert_ledger_equal(&held_selector.ledger, &premature.ledger);
    assert_eq!(
        premature
            .exchanges
            .iter()
            .filter(|(q, _)| q.protocol == 352 && q.msg_type == 5)
            .count(),
        confirmations
    );
    f.d.send(&f.h, 21, 2, 4);
    f.d.stroke(&f.h, 40, 4);
    let launched = latest::<editor::ConfirmLaunchReply>(&f.c.evidence().unwrap());
    assert_eq!(launched.status, 0);
    assert_eq!(
        f.c.evidence()
            .unwrap()
            .exchanges
            .iter()
            .filter(|(q, _)| q.protocol == 352 && q.msg_type == 5)
            .count(),
        confirmations + 1
    );
    let bindings = f.c.editor_bindings(launched.instance_id).unwrap();
    let boot = editor::InstanceBootstrap::decode(&bindings.bootstrap).unwrap();
    assert_ne!(boot.instance_id, l.boot.instance_id);
    assert_ne!(boot.instance_generation, l.boot.instance_generation);
    let client = EditorClient::new(f.h.clone(), bindings, 4).unwrap();
    assert_eq!(client.snapshot().bytes, b"note=old\n");
    assert!(matches!(
        client.snapshot().save,
        SaveState::Saved { revision: 1 }
    ));
    deny(
        &f.h,
        &l.bindings.artifact,
        artifact::ReadSelected {
            request_id: 40,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        36,
        4,
        Status::Stale,
    );
    let mut r = Record::new(name);
    r.frame(recovery);
    r.note("fault_recovery",&format!("{{\"old_instance\":{},\"new_instance\":{},\"draft_lost\":true,\"explicit_ctrl_r_approval\":true,\"confirmed_revision\":1}}",l.boot.instance_id,boot.instance_id));
    r.note("selector_release",&format!("{{\"held_r_enter_no_confirm\":true,\"held_r_enter_no_allocation\":true,\"prior_confirm_requests\":{confirmations},\"fresh_enter_after_r_release_confirmed\":true,\"terminal_focus_assign_denied\":true}}"));
    r.finish(&f.c, 0);
}

#[test]
fn ui1_1_focus_compositor_restart_retires_old_epochs() {
    let name = "ui1_1_focus_compositor_restart_retires_old_epochs";
    for (i, service) in [ServiceKind::Focus, ServiceKind::Compositor]
        .into_iter()
        .enumerate()
    {
        let mut f = Fixture::new(b"note=old\n");
        let l = f.launch(1);
        let old = observe(&f.h, &l, 1);
        let producer = f.d.s.input.clone();
        let q = f.d.request(4, 1, 0);
        f.c.restart_service(service, 2).unwrap();
        deny(&f.h, &producer, q, 8, 2, Status::Stale);
        deny(
            &f.h,
            &l.bindings.focus_read,
            poll_request(&l, old.focus_epoch),
            36,
            2,
            Status::Stale,
        );
        deny(
            &f.h,
            &l.bindings.surface,
            surface::Acquire {
                request_id: 41,
                surface_id: l.surface.surface_id,
                surface_generation: l.surface.surface_generation,
                buffer_index: 0,
                reserved: 0,
            },
            24,
            2,
            Status::Stale,
        );
        let (input, device) = f.c.reattach_keyboard(f.d.s.session_id, 3).unwrap();
        f.d.s.input = input;
        f.d.s.device_generation = device;
        f.d.next = 1;
        f.d.mods = 0;
        f.d.attach(&f.h, 3);
        let mut fresh = f.launch(3);
        let now = observe(&f.h, &fresh, 3);
        assert!(now.service_epoch > old.service_epoch);
        assert!(now.focus_epoch > old.focus_epoch);
        assert_ne!(fresh.surface.surface_id, l.surface.surface_id);
        f.d.key(&f.h, &mut fresh, 4, 4);
        assert!(fresh.client.snapshot().bytes.ends_with(b"a"));
        let mut r = Record::new(name);
        r.frame(render(&f.h, &f.d, &mut fresh, 4));
        r.note("service_recovery",&format!("{{\"service\":\"{}\",\"old_epoch\":{},\"new_epoch\":{},\"old_focus\":{},\"new_focus\":{},\"old_contexts_stale\":true}}",if i==0 {"focus"}else{"compositor"},old.service_epoch,now.service_epoch,old.focus_epoch,now.focus_epoch));
        r.finish(&f.c, i as u32);
    }
}

#[test]
fn ui1_1_stalled_session_leaves_other_session_usable() {
    let name = "ui1_1_stalled_session_leaves_other_session_usable";
    for (i, save) in [false, true].into_iter().enumerate() {
        let mut f = Fixture::new(b"note=old\n");
        let l = f.launch(1);
        let mut b = Driver::register(&f.h, &f.c, 2, b"other\n", 1);
        let mut bl = b.launch(&f.h, &f.c, 1);
        let token =
            f.c.pause_next(
                f.d.s.session_id,
                if save {
                    PausePoint::AfterCommitPermit
                } else {
                    PausePoint::BeforeReadReply
                },
            )
            .unwrap();
        let h = f.h.clone();
        let endpoint = l.bindings.artifact.clone();
        let boot = l.boot;
        let q = if save {
            Some(commit_request(&f.h, &l, b"note=new\n", 1, 2))
        } else {
            None
        };
        let started = Instant::now();
        let join = thread::spawn(move || {
            if let Some(q) = q {
                let r: artifact::CommitReply = call(&h, &endpoint, q, 2);
                (r.status, r.operation_id, r.revision, r.receipt_shm)
            } else {
                let r: artifact::ReadSelectedReply = call(
                    &h,
                    &endpoint,
                    artifact::ReadSelected {
                        request_id: 42,
                        session_id: boot.session_id,
                        session_generation: boot.session_generation,
                        instance_id: boot.instance_id,
                        instance_generation: boot.instance_generation,
                    },
                    2,
                );
                (r.status, r.revision, r.object_generation, r.data_shm)
            }
        });
        let entered = token.wait_until_entered(2_000).unwrap();
        assert!(entered.entered);
        assert_eq!(entered.permit_issued, save);
        let b_start = Instant::now();
        b.key(&f.h, &mut bl, 5, 3);
        let frame = render(&f.h, &b, &mut bl, 3);
        assert_eq!(selected(&f.h, &bl, 3).1, b"other\n");
        let b_ms = b_start.elapsed().as_millis();
        assert!(b_ms < 1_000, "B blocked by A");
        let reply = join.join().unwrap();
        let elapsed = started.elapsed();
        assert!(elapsed >= Duration::from_millis(900) && elapsed < Duration::from_millis(2_000));
        if save {
            assert_eq!(reply, (10, q.unwrap().operation_id, 0, 0));
            deny(
                &f.h,
                &l.bindings.artifact,
                artifact::AllocateSaveId {
                    request_id: 43,
                    session_id: l.boot.session_id,
                    session_generation: l.boot.session_generation,
                    expected_revision: 1,
                },
                16,
                3,
                Status::NotReady,
            );
            assert!(matches!(
                f.c.restart_service(ServiceKind::Artifact, 3),
                Err(Status::NotReady)
            ));
        } else {
            assert_eq!(reply, (8, 0, 0, 0));
        }
        f.c.release(&token).unwrap();
        let settled = token.wait_until_settled(2_000).unwrap();
        assert!(settled.settled);
        let mut r = Record::new(name);
        r.frame(frame);
        if save {
            let original_operation = q.unwrap().operation_id;
            let original_receipt =
                receipt(&f.h, &l.bindings.artifact, &f.d.s, original_operation, 4);
            assert_eq!(u64_at(&original_receipt, 88), 2);
            close(&f.h, &f.d.s, &l.boot, 4);
            let mut retry = f.launch(4);
            assert_eq!(retry.client.snapshot().confirmed_revision, 2);
            f.d.key(&f.h, &mut retry, 4, 4);
            let draft = retry.client.snapshot();
            assert!(matches!(draft.save, SaveState::Unsaved));
            let before_timeout = f.c.evidence().unwrap();
            assert_eq!(
                (
                    before_timeout.mutation_dispatch_count,
                    before_timeout.permit_count,
                    before_timeout.transition_count
                ),
                (1, 1, 1)
            );
            f.d.event(&f.h, &mut retry, 224, 1, 5);
            let timeout_barrier =
                f.c.pause_next(f.d.s.session_id, PausePoint::BeforeCommitPermit)
                    .unwrap();
            f.d.send(&f.h, 22, 1, 5);
            let mut client = retry.client;
            let timeout_started = Instant::now();
            let timeout_join = thread::spawn(move || {
                let result = client.step(5);
                (client, result)
            });
            let timeout_entered = timeout_barrier.wait_until_entered(2_000).unwrap();
            assert!(
                timeout_entered.entered
                    && !timeout_entered.permit_issued
                    && timeout_entered.operation_id > 0
            );
            let (client, result) = timeout_join.join().unwrap();
            retry.client = client;
            assert!(
                matches!(result, Err(Status::Timeout)),
                "original pre-permit reply: {result:?}"
            );
            let timeout_elapsed = timeout_started.elapsed();
            assert!(
                timeout_elapsed >= Duration::from_millis(900)
                    && timeout_elapsed < Duration::from_millis(2_000)
            );
            let timed_out = f.c.evidence().unwrap();
            let (timed_request, timed_reply) = timed_out
                .exchanges
                .iter()
                .rev()
                .find(|(q, _)| q.protocol == 368 && q.msg_type == 5)
                .unwrap();
            let timed_request = artifact::Commit::decode(timed_request).unwrap();
            assert_eq!(timed_request.operation_id, timeout_entered.operation_id);
            zero_reply(timed_reply, 32, Status::Timeout);
            assert_eq!(
                (
                    timed_out.mutation_dispatch_count,
                    timed_out.permit_count,
                    timed_out.transition_count
                ),
                (2, 1, 1)
            );
            assert!(matches!(retry.client.snapshot().save, SaveState::Unsaved));
            assert_draft_equal(&draft, &retry.client.snapshot());
            assert!(matches!(
                f.h.write_lease(
                    &retry.bindings.artifact.peer,
                    &desc(
                        timed_request.source_shm,
                        ObjectKind::DraftSource,
                        timed_request.byte_len
                    ),
                    0,
                    5
                ),
                Err(Status::Stale)
            ));
            assert!(matches!(
                timeout_barrier.wait_until_settled(1),
                Err(Status::Timeout)
            ));
            f.d.event(&f.h, &mut retry, 22, 2, 5);
            f.d.event(&f.h, &mut retry, 224, 2, 5);
            f.d.ctrl(&f.h, &mut retry, 22, 5); // fresh human request, before the old worker is released
            assert!(matches!(
                retry.client.snapshot().save,
                SaveState::Saved { revision: 3 }
            ));
            let retry_reply = latest::<artifact::CommitReply>(&f.c.evidence().unwrap());
            assert_eq!((retry_reply.status, retry_reply.revision), (0, 3));
            assert!(retry_reply.operation_id > 0);
            assert_ne!(retry_reply.operation_id, timed_request.operation_id);
            let retry_receipt = receipt(
                &f.h,
                &retry.bindings.artifact,
                &f.d.s,
                retry_reply.operation_id,
                5,
            );
            assert_eq!(u32_at(&retry_receipt, 8), 0);
            assert_eq!(
                (u64_at(&retry_receipt, 40), u64_at(&retry_receipt, 48)),
                (retry.boot.instance_id, retry.boot.instance_generation)
            );
            assert_eq!(
                (
                    u64_at(&retry_receipt, 72),
                    u64_at(&retry_receipt, 80),
                    u64_at(&retry_receipt, 88)
                ),
                (retry_reply.operation_id, 2, 3)
            );
            assert_eq!(
                &retry_receipt[112..144],
                &sha2::Sha256::digest(b"note=new\n")[..]
            );
            assert_eq!(
                &retry_receipt[144..176],
                &sha2::Sha256::digest(&draft.bytes)[..]
            );
            assert_eq!(selected(&f.h, &retry, 5), (3, draft.bytes.clone()));
            let after_retry = f.c.evidence().unwrap();
            assert_eq!(
                (
                    after_retry.mutation_dispatch_count,
                    after_retry.permit_count,
                    after_retry.transition_count
                ),
                (3, 2, 2)
            );
            assert!(matches!(
                timeout_barrier.wait_until_settled(1),
                Err(Status::Timeout)
            ));
            b.key(&f.h, &mut bl, 4, 5);
            r.frame(render(&f.h, &b, &mut bl, 5));
            assert_eq!(selected(&f.h, &bl, 5).1, b"other\n");
            f.c.release(&timeout_barrier).unwrap();
            let timeout_settled = timeout_barrier.wait_until_settled(2_000).unwrap();
            assert!(timeout_settled.settled && !timeout_settled.permit_issued);
            let after_release = f.c.evidence().unwrap();
            assert_eq!(
                (
                    after_release.mutation_dispatch_count,
                    after_release.permit_count,
                    after_release.transition_count
                ),
                (3, 2, 2)
            );
            let allocate_requests = after_release
                .exchanges
                .iter()
                .filter(|(q, _)| q.protocol == 368 && q.msg_type == 3)
                .count();
            let allocated_operations = after_release
                .exchanges
                .iter()
                .filter(|(q, r)| {
                    q.protocol == 368
                        && q.msg_type == 3
                        && artifact::AllocateSaveIdReply::decode(r).unwrap().status == 0
                })
                .count();
            let original_commit_requests = after_release
                .exchanges
                .iter()
                .filter(|(q, _)| q.protocol == 368 && q.msg_type == 5)
                .count();
            assert_eq!(
                (
                    allocate_requests,
                    allocated_operations,
                    original_commit_requests
                ),
                (4, 3, 3)
            );
            assert!(
                after_release
                    .ledger
                    .operations_per_object
                    .iter()
                    .any(|(_, n)| *n == 3)
            );
            assert_eq!(selected(&f.h, &retry, 5), (3, draft.bytes.clone()));
            barrier_note(&mut r, "prepermit_entered", &timeout_entered);
            barrier_note(&mut r, "prepermit_settled", &timeout_settled);
            r.note("prepermit_retry", &format!("{{\"timed_out_operation_id\":{},\"retry_operation_id\":{},\"timeout_status\":8,\"timeout_reply_redacted\":true,\"draft_unsaved_after_timeout\":true,\"old_source_stale\":true,\"fresh_explicit_ctrl_s\":true,\"old_worker_still_paused\":true,\"retry_base_revision\":2,\"retry_revision\":3,\"retry_content_hash\":\"{}\",\"retry_receipt_hex\":\"{}\",\"before_dispatches\":{},\"before_permits\":{},\"before_transitions\":{},\"timeout_dispatches\":{},\"timeout_permits\":{},\"timeout_transitions\":{},\"retry_dispatches\":{},\"retry_permits\":{},\"retry_transitions\":{},\"after_release_dispatches\":{},\"after_release_permits\":{},\"after_release_transitions\":{},\"allocate_requests\":{allocate_requests},\"allocated_operations\":{allocated_operations},\"original_commit_requests\":{original_commit_requests},\"no_late_selection\":true,\"b_working\":true}}",
                timed_request.operation_id, retry_reply.operation_id, hash(&draft.bytes), hex(&retry_receipt),
                before_timeout.mutation_dispatch_count,before_timeout.permit_count,before_timeout.transition_count,
                timed_out.mutation_dispatch_count,timed_out.permit_count,timed_out.transition_count,
                after_retry.mutation_dispatch_count,after_retry.permit_count,after_retry.transition_count,
                after_release.mutation_dispatch_count,after_release.permit_count,after_release.transition_count));
        }
        barrier_note(&mut r, "stall_entered", &entered);
        barrier_note(&mut r, "stall_settled", &settled);
        r.note("deadline",&format!("{{\"operation\":\"{}\",\"response_status\":{},\"elapsed_ms\":{},\"b_elapsed_ms\":{b_ms},\"b_keys_frame_read_ok\":true,\"no_maintenance_poll\":true}}",if save {"save"}else{"read"},reply.0,elapsed.as_millis()));
        r.finish(&f.c, i as u32);
    }
}

#[test]
fn ui1_1_clock_capacity_and_counter_limits_fail_closed() {
    let name = "ui1_1_clock_capacity_and_counter_limits_fail_closed";
    let mut f = Fixture::ttl(b"note=old\n", 100, 100);
    let l = f.launch(1);
    let mut b = Driver::register(&f.h, &f.c, 2, b"other\n", 100);
    let bl = b.launch(&f.h, &f.c, 100);
    assert_eq!(selected(&f.h, &bl, 102).1, b"other\n"); // B is live until200; its request advances clock beyond A's101 expiry.
    assert!(matches!(
        f.c.register_session(3, APP, MANIFEST, b"third\n", 2),
        Err(Status::Exhausted)
    ));
    deny(
        &f.h,
        &l.bindings.self_status,
        status_request(&l, &f.d.s),
        44,
        1,
        Status::Stale,
    );
    let mut r = Record::new(name);
    deny(
        &f.h,
        &f.d.s.chrome,
        focus::Assign {
            request_id: 140,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
            surface_id: l.surface.surface_id,
            surface_generation: l.surface.surface_generation,
        },
        24,
        1,
        Status::Stale,
    );
    r.note(
        "monotonic_clock",
        "{\"b_advances_to\":102,\"backward_a\":1,\"a_revived\":false,\"terminal_focus_assign_denied\":true}",
    );
    deny(
        &f.h,
        &l.bindings.artifact,
        artifact::ReadSelected {
            request_id: 57,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        36,
        1,
        Status::Stale,
    );
    deny(
        &f.h,
        &l.bindings.self_status,
        editor::ObserveFocus {
            request_id: 58,
            session_id: l.boot.session_id,
            session_generation: l.boot.session_generation,
            instance_id: l.boot.instance_id,
            instance_generation: l.boot.instance_generation,
        },
        24,
        1,
        Status::Stale,
    );
    assert!(matches!(
        f.h.read_lease(
            &l.bindings.self_status.peer,
            &desc(l.boot.grants_shm, ObjectKind::Grants, l.boot.grants_len),
            1
        ),
        Err(Status::Stale)
    ));
    r.finish(&f.c, 0);
    let mut ops = Fixture::new(b"note=old\n");
    let ol = ops.launch(1);
    for revision in 1..=16 {
        let q = commit_request(&ops.h, &ol, b"note=new\n", revision, 2);
        let reply: artifact::CommitReply = call(&ops.h, &ol.bindings.artifact, q, 2);
        assert_eq!((reply.status, reply.revision), (0, revision + 1));
    }
    deny(
        &ops.h,
        &ol.bindings.artifact,
        artifact::AllocateSaveId {
            request_id: 44,
            session_id: ol.boot.session_id,
            session_generation: ol.boot.session_generation,
            expected_revision: 17,
        },
        16,
        2,
        Status::Exhausted,
    );
    assert_eq!(selected(&ops.h, &ol, 2).0, 17);
    let e = ops.c.evidence().unwrap();
    assert_eq!((e.permit_count, e.transition_count), (16, 16));
    assert!(e.ledger.operations_per_object.iter().any(|(_, n)| *n == 16));
    let mut rr = Record::new(name);
    rr.note(
        "operation_capacity",
        "{\"retained\":16,\"seventeenth_denied\":true,\"revision\":17}",
    );
    rr.finish(&ops.c, 1);
    let mut history = Fixture::new(b"note=old\n");
    let first = history.launch(1);
    close(&history.h, &history.d.s, &first.boot, 2);
    for _ in 1..16 {
        let p = history.d.preview(&history.h, &history.c, 3);
        assert_eq!(p.status, 0);
        history.d.stroke(&history.h, 40, 3);
        let issued = latest::<editor::ConfirmLaunchReply>(&history.c.evidence().unwrap());
        assert_eq!(issued.status, 0);
        let binding = history.c.editor_bindings(issued.instance_id).unwrap();
        let boot = editor::InstanceBootstrap::decode(&binding.bootstrap).unwrap();
        close(&history.h, &history.d.s, &boot, 3);
    }
    let p = history.d.preview(&history.h, &history.c, 4);
    if p.status == 0 {
        history.d.stroke(&history.h, 40, 4);
        let denied = latest::<editor::ConfirmLaunchReply>(&history.c.evidence().unwrap());
        assert_eq!(denied.status, 5);
        assert_eq!(
            (
                denied.instance_id,
                denied.instance_generation,
                denied.grants_shm,
                denied.grants_len
            ),
            (0, 0, 0, 0)
        );
    } else {
        assert_eq!(p.status, 5)
    }
    assert_eq!(history.c.evidence().unwrap().ledger.instances, 16);
    let mut rr = Record::new(name);
    rr.note(
        "instance_capacity",
        "{\"retained\":16,\"seventeenth_denied\":true}",
    );
    rr.finish(&history.c, 2);
    for (i, counter) in [
        CounterKind::Identity,
        CounterKind::FocusEpoch,
        CounterKind::QueueSequence,
        CounterKind::MappingGeneration,
        CounterKind::SelectedRevision,
        CounterKind::ServiceEpoch,
        CounterKind::TimeOrigin,
    ]
    .into_iter()
    .enumerate()
    {
        let mut x = Fixture::new(b"note=old\n");
        let mut l = x.launch(1);
        assert_eq!(selected(&x.h, &l, 1).0, 1);
        let prior_receipt = if i == 4 || i == 5 {
            let q = commit_request(&x.h, &l, b"note=old\n", 1, 1);
            let committed: artifact::CommitReply = call(&x.h, &l.bindings.artifact, q, 1);
            assert_eq!((committed.status, committed.revision), (0, 2));
            Some((
                q.operation_id,
                receipt(&x.h, &l.bindings.artifact, &x.d.s, q.operation_id, 1),
            ))
        } else {
            None
        };
        assert_eq!(selected(&x.h, &l, 1).1, b"note=old\n");
        let selected_reply = latest::<artifact::ReadSelectedReply>(&x.c.evidence().unwrap());
        let selected_lease =
            x.h.read_lease(
                &l.bindings.artifact.peer,
                &desc(
                    selected_reply.data_shm,
                    ObjectKind::SelectedText,
                    selected_reply.byte_len,
                ),
                1,
            )
            .unwrap();
        let mut selected_bytes = vec![0; selected_reply.byte_len as usize];
        selected_lease.copy_into(0, &mut selected_bytes, 1).unwrap();
        let original_focus = observe(&x.h, &l, 1).focus_epoch;
        let pending_preview = if i == 1 || i == 2 {
            let p = x.d.preview(&x.h, &x.c, 1);
            assert_eq!(p.status, 0);
            let preview = desc(p.preview_shm, ObjectKind::Preview, p.preview_len);
            assert_eq!(read(&x.h, &x.d.s.chrome, &preview, 1).len(), 464);
            let assigned: focus::AssignReply = call(
                &x.h,
                &x.d.s.chrome,
                focus::Assign {
                    request_id: 150,
                    session_id: l.boot.session_id,
                    session_generation: l.boot.session_generation,
                    instance_id: l.boot.instance_id,
                    instance_generation: l.boot.instance_generation,
                    surface_id: l.surface.surface_id,
                    surface_generation: l.surface.surface_generation,
                },
                1,
            );
            assert_eq!(assigned.status, 0);
            Some((p, preview))
        } else {
            None
        };
        let queued_epoch = if i == 1 || i == 2 {
            x.d.key(&x.h, &mut l, 5, 1);
            let epoch = observe(&x.h, &l, 1).focus_epoch;
            x.d.send(&x.h, 4, 1, 1);
            assert_eq!(x.c.evidence().unwrap().ledger.queued_keys, 1);
            Some(epoch)
        } else {
            None
        };
        let retained_mapping = if i == 3 {
            let acquired: surface::AcquireReply = call(
                &x.h,
                &l.bindings.surface,
                surface::Acquire {
                    request_id: 145,
                    surface_id: l.surface.surface_id,
                    surface_generation: l.surface.surface_generation,
                    buffer_index: 0,
                    reserved: 0,
                },
                1,
            );
            assert_eq!(acquired.status, 0);
            let writer =
                x.h.write_lease(
                    &l.bindings.surface.peer,
                    &desc(
                        acquired.buffer_shm,
                        ObjectKind::SurfaceBuffer,
                        acquired.byte_len,
                    ),
                    acquired.mapping_generation,
                    1,
                )
                .unwrap();
            writer.copy_from(0, &[0x61; 4], 1).unwrap();
            let alias = writer.clone();
            Some((acquired, writer, alias))
        } else {
            None
        };
        x.c.seed_exhaustion(counter).unwrap();
        let before_failure = x.c.evidence().unwrap();
        let mut failed_commit = None;
        match i {
            0 | 6 => assert!(matches!(
                x.c.register_session(2, APP, MANIFEST, b"other\n", 2),
                Err(Status::Exhausted)
            )),
            1 => deny(
                &x.h,
                &x.d.s.chrome,
                focus::Assign {
                    request_id: 45,
                    session_id: l.boot.session_id,
                    session_generation: l.boot.session_generation,
                    instance_id: l.boot.instance_id,
                    instance_generation: l.boot.instance_generation,
                    surface_id: l.surface.surface_id,
                    surface_generation: l.surface.surface_generation,
                },
                24,
                2,
                Status::Exhausted,
            ),
            2 => {
                x.d.next = u64::MAX;
                deny(
                    &x.h,
                    &x.d.s.input,
                    x.d.request(4, 1, 0),
                    8,
                    2,
                    Status::Exhausted,
                )
            }
            3 => deny(
                &x.h,
                &l.bindings.surface,
                surface::Acquire {
                    request_id: 46,
                    surface_id: l.surface.surface_id,
                    surface_generation: l.surface.surface_generation,
                    buffer_index: 1,
                    reserved: 0,
                },
                24,
                2,
                Status::Exhausted,
            ),
            4 => {
                let q = commit_request(&x.h, &l, b"new\n", u64::MAX, 2);
                deny(&x.h, &l.bindings.artifact, q, 32, 2, Status::Exhausted);
                failed_commit = Some(q);
            }
            5 => assert!(matches!(
                x.c.restart_service(ServiceKind::Focus, 2),
                Err(Status::Exhausted)
            )),
            _ => unreachable!(),
        }
        let after_failure = x.c.evidence().unwrap();
        match i {
            0 => {
                assert_ledger_equal(&before_failure.ledger, &after_failure.ledger);
                let mut copied = vec![0; selected_bytes.len()];
                selected_lease.copy_into(0, &mut copied, 2).unwrap();
                assert_eq!(copied, selected_bytes);
                let status: editor::GetStatusReply =
                    call(&x.h, &l.bindings.self_status, status_request(&l, &x.d.s), 2);
                assert_eq!((status.status, status.state), (0, 2));
                x.d.key(&x.h, &mut l, 4, 2);
                assert!(l.client.snapshot().bytes.ends_with(b"a"));
            }
            1 | 2 => {
                let draft = l.client.snapshot();
                deny(
                    &x.h,
                    &x.d.s.input,
                    x.d.request(4, 2, 0),
                    8,
                    2,
                    Status::Stale,
                );
                deny(
                    &x.h,
                    &l.bindings.focus_read,
                    poll_request(&l, queued_epoch.unwrap()),
                    36,
                    2,
                    Status::Stale,
                );
                deny(
                    &x.h,
                    &l.bindings.self_status,
                    editor::ObserveFocus {
                        request_id: 146,
                        session_id: l.boot.session_id,
                        session_generation: l.boot.session_generation,
                        instance_id: l.boot.instance_id,
                        instance_generation: l.boot.instance_generation,
                    },
                    24,
                    2,
                    Status::NotReady,
                );
                assert_eq!(x.c.evidence().unwrap().ledger.queued_keys, 0);
                assert_draft_equal(&draft, &l.client.snapshot());
                let (p, preview) = pending_preview.unwrap();
                deny(
                    &x.h,
                    &x.d.s.chrome,
                    editor::ConfirmLaunch {
                        request_id: 151,
                        session_id: l.boot.session_id,
                        session_generation: l.boot.session_generation,
                        plan_id: p.plan_id,
                        preview_revision: p.preview_revision,
                    },
                    44,
                    2,
                    Status::Stale,
                );
                assert!(matches!(
                    x.h.read_lease(&x.d.s.chrome.peer, &preview, 2),
                    Err(Status::Stale)
                ));
                assert_eq!(selected(&x.h, &l, 2).1, b"note=old\n");
            }
            3 => {
                let (acquired, writer, alias) = retained_mapping.unwrap();
                assert!(matches!(
                    writer.copy_from(0, &[0xff; 4], 2),
                    Err(Status::Stale)
                ));
                assert!(matches!(
                    alias.copy_from(0, &[0xff; 4], 2),
                    Err(Status::Stale)
                ));
                deny(
                    &x.h,
                    &l.bindings.surface,
                    surface::Present {
                        request_id: 147,
                        surface_id: l.surface.surface_id,
                        surface_generation: l.surface.surface_generation,
                        mapping_generation: acquired.mapping_generation,
                        sequence: 2,
                        buffer_index: 0,
                        reserved: 0,
                    },
                    16,
                    2,
                    Status::Stale,
                );
                assert!(matches!(
                    x.h.compose_next(&x.d.s.compositor, 2),
                    Err(Status::NotReady)
                ));
                assert_eq!(
                    after_failure.ledger.shared_objects + 2,
                    before_failure.ledger.shared_objects
                );
                assert_eq!(
                    after_failure.ledger.endpoints + 1,
                    before_failure.ledger.endpoints
                );
                assert_eq!(selected(&x.h, &l, 2).1, b"note=old\n");
            }
            4 => {
                deny(
                    &x.h,
                    &l.bindings.artifact,
                    artifact::AllocateSaveId {
                        request_id: 148,
                        session_id: l.boot.session_id,
                        session_generation: l.boot.session_generation,
                        expected_revision: u64::MAX,
                    },
                    16,
                    2,
                    Status::Stale,
                );
                deny(
                    &x.h,
                    &l.bindings.artifact,
                    failed_commit.unwrap(),
                    32,
                    2,
                    Status::Stale,
                );
                assert!(matches!(
                    x.h.draft_source(&l.bindings.artifact.peer, u64::MAX, b"new\n", 2),
                    Err(Status::Stale)
                ));
                let (operation, prior) = prior_receipt.as_ref().unwrap();
                assert_eq!(
                    receipt(&x.h, &l.bindings.artifact, &x.d.s, *operation, 2),
                    *prior
                );
                assert_eq!(selected(&x.h, &l, 2), (u64::MAX, b"note=old\n".to_vec()));
            }
            5 => {
                deny(
                    &x.h,
                    &x.d.s.input,
                    x.d.request(4, 1, 0),
                    8,
                    2,
                    Status::Stale,
                );
                deny(
                    &x.h,
                    &l.bindings.focus_read,
                    poll_request(&l, original_focus),
                    36,
                    2,
                    Status::Stale,
                );
                deny(
                    &x.h,
                    &l.bindings.self_status,
                    editor::ObserveFocus {
                        request_id: 149,
                        session_id: l.boot.session_id,
                        session_generation: l.boot.session_generation,
                        instance_id: l.boot.instance_id,
                        instance_generation: l.boot.instance_generation,
                    },
                    24,
                    2,
                    Status::Stale,
                );
                assert_eq!(selected(&x.h, &l, 2), (2, b"note=old\n".to_vec()));
                let (operation, prior) = prior_receipt.as_ref().unwrap();
                assert_eq!(
                    receipt(&x.h, &l.bindings.artifact, &x.d.s, *operation, 2),
                    *prior
                );
            }
            6 => {
                deny(
                    &x.h,
                    &l.bindings.self_status,
                    status_request(&l, &x.d.s),
                    44,
                    1,
                    Status::Stale,
                );
                let mut denied = [0x6d; 4];
                assert!(matches!(
                    selected_lease.copy_into(0, &mut denied, 1),
                    Err(Status::Stale)
                ));
                assert_eq!(denied, [0x6d; 4]);
            }
            _ => unreachable!(),
        }
        let after_probes = x.c.evidence().unwrap();
        assert_eq!(
            (
                after_failure.mutation_dispatch_count,
                after_failure.permit_count,
                after_failure.transition_count
            ),
            (
                after_probes.mutation_dispatch_count,
                after_probes.permit_count,
                after_probes.transition_count
            )
        );
        assert_eq!(
            (before_failure.permit_count, before_failure.transition_count),
            (after_probes.permit_count, after_probes.transition_count)
        );
        assert_eq!(
            after_probes.mutation_dispatch_count,
            before_failure.mutation_dispatch_count + u32::from(i == 4)
        );
        let mut rr = Record::new(name);
        rr.note(
            "counter_exhaustion",
            &format!(
                "{{\"counter_index\":{i},\"wrapped\":false,\"valid_witness_before_seed\":true}}"
            ),
        );
        rr.note("counter_domain_retirement", &format!(
            "{{\"counter_index\":{i},\"domain\":\"{}\",\"first_failure_status\":5,\"retained_endpoint_status\":{},\"positive_read\":{},\"receipt_preserved\":{},\"writer_aliases_retired\":{},\"queued_before\":{},\"queued_after\":{},\"before_dispatches\":{},\"failed_dispatches\":{},\"after_dispatches\":{},\"before_permits\":{},\"failed_permits\":{},\"after_permits\":{},\"before_transitions\":{},\"failed_transitions\":{},\"after_transitions\":{}}}",
            ["identity", "focus", "queue", "mapping", "revision", "service", "time"][i],
            if i == 0 { 0 } else { 4 }, i != 6, i == 4 || i == 5, i == 3,
            before_failure.ledger.queued_keys, after_probes.ledger.queued_keys,
            before_failure.mutation_dispatch_count, after_failure.mutation_dispatch_count, after_probes.mutation_dispatch_count,
            before_failure.permit_count, after_failure.permit_count, after_probes.permit_count,
            before_failure.transition_count, after_failure.transition_count, after_probes.transition_count,
        ));
        rr.finish(&x.c, i as u32 + 3);
    }
    for (preview, instance) in [(0, 1), (30_001, 1), (1, 0), (1, 600_001)] {
        assert!(matches!(
            HostDesktop::new(config(preview, instance)),
            Err(Status::Invalid)
        ));
    }
    let mut trace = Fixture::new(b"note=old\n");
    let l = trace.launch(1);
    for n in 0..256 {
        if trace.c.evidence().unwrap().exchanges.len() == 256 {
            break;
        }
        let mut q = status_request(&l, &trace.d.s);
        q.request_id = 10_000 + n;
        let reply: editor::GetStatusReply = call(&trace.h, &l.bindings.self_status, q, 2);
        assert_eq!((reply.status, reply.state), (0, 2));
    }
    let prior = trace.c.evidence().unwrap();
    assert_eq!(prior.exchanges.len(), 256);
    let mut q = status_request(&l, &trace.d.s);
    q.request_id = 20_000;
    let request = q.encode(l.bindings.self_status.handle).unwrap();
    let reply = trace.h.dispatch(&l.bindings.self_status.peer, &request, 2);
    let mut rr = Record::new(name);
    rr.note("trace_exhaustion","{\"retained_before_overflow\":256,\"terminal_status\":5,\"post_overflow_evidence_failed\":true,\"not_silently_truncated\":true}");
    rr.finish_trace_negative(&trace.c, 10, &prior, &request, &reply);
}
