//! Native READ precursor: real UIa approval, actual native entry, real Store read.
//! This does not claim Store-backed preview, an integrated Save task, or isolation.
#![cfg(feature = "editor_native_read_v0_dev")]
mod editor_native_read_support;
use artifact_store_schema::editor_save::TEXT_HEADER_LEN;
use desktop_service::dev::Status;
use desktop_service::editor_dev::{self as desktop, EditorMessage};
use editor_native_read_support::*;
use kernel_api::cap::Handle;
use kernel_api::generated::desktop_artifact_v1 as artifact;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use store_service::editor_store::{self as store, StoreStatus};

#[test]
fn native_read_positive() {
    let mut f = Fixture::new("native_read_positive", 2);
    let mut joined_ids = std::collections::BTreeSet::new();
    for i in 0..2 {
        let entered_before = Instant::now();
        let entry = f.rig.entry(i, i as u64 + 10);
        let call = success(entry.wait());
        assert!(call.entered() >= entered_before && call.entered() <= Instant::now());
        assert_eq!(
            call.deadline().duration_since(call.entered()),
            Duration::from_millis(1000)
        );
        let original = call.request();
        assert_eq!(
            success(desktop::encode_envelope_wire(&original)),
            success(desktop::encode_envelope_wire(
                &f.rig.request(i, i as u64 + 10)
            ))
        );
        denied(
            call.reserve_store_producer(desktop::ProducerKind::StoreIo),
            Status::Unsupported,
        );
        let reg = success(f.host().register_native_read(&f.endpoint(i).peer, call));
        let before = f.rig.h.native_producer_counts();
        assert_eq!((before.held, before.io, before.joining), (2, 0, 0));
        let reply = success(f.host().execute_native_read(&reg));
        let after = f.rig.h.native_producer_counts();
        assert_eq!(
            (after.held, after.io, after.joining),
            (before.held + 2, 0, 0),
            "actual Store dispatcher/supervisor adds two shared charged rows"
        );
        assert_eq!(f.host().native_read_producer_counts().held, after.held);
        let bytes = checked_selected(
            &reply,
            &f.rig.views[i],
            if i == 0 { A } else { B },
            i as u64 + 10,
        );
        assert_eq!(reply.producer_ids().len(), 2);
        let desktop_ids = entry.producer_ids().map(|id| join_desktop(&f.rig, id));
        for id in desktop_ids {
            assert!(joined_ids.insert(id));
        }
        let store_ids = reply
            .producer_ids()
            .iter()
            .map(|id| join_store(&f.host(), id))
            .collect::<Vec<_>>();
        for id in &store_ids {
            assert!(
                joined_ids.insert(*id),
                "Store pair differs from Desktop and all prior joins"
            );
        }
        let settled = f.rig.h.native_producer_counts();
        assert_eq!((settled.held, settled.io, settled.joining), (0, 0, 0));
        assert_eq!(f.host().native_read_producer_counts().held, 0);
        f.observations.push(json!({"kind":"positive","request":hex(&success(desktop::encode_envelope_wire(&original))),
            "reply":hex(&success(desktop::encode_envelope_wire(&reply.wire()))),"selected":hex(&bytes),
            "original_deadline_ms":1000,"io":settled.io,"shared_held_before_execute":before.held,
            "shared_held_after_execute":after.held,"actual_desktop_join_ids":desktop_ids,
            "actual_store_join_ids":store_ids,"shared_held_after_actual_joins":settled.held}));
        assert_eq!(f.host().native_read_producer_counts().io, 0);
        assert_unchanged(&f);
    }
    assert_eq!(joined_ids.len(), 8);
    f.finish();
}

#[test]
fn native_read_identity_denials() {
    // Pure views and matching payloads cannot substitute for an actual issuer.
    let mut own = Rig::new(1);
    let mut foreign = Rig::new(1);
    let binding = own.binding(0);
    let alien = foreign.binding(0);
    let witness = own.witness.as_ref().unwrap();
    let wrong = foreign.witness.as_ref().unwrap();
    assert!(!wrong.matches_binding(&binding));
    denied(binding.admit(wrong), Status::Denied);
    let guard = success(binding.admit(witness));
    denied(guard.view_binding(&alien), Status::Denied);
    drop(guard);
    let entry = own.entry(0, 20);
    let call = success(entry.wait());
    denied(wrong.join_finished(entry.producer_ids()[0]), Status::Denied);
    let held_before_alias = own.h.native_producer_counts();
    denied(
        wrong.duplicate_producer(entry.producer_ids()[0]),
        Status::Denied,
    );
    let alias = success(witness.duplicate_producer(entry.producer_ids()[0]));
    let after_alias = own.h.native_producer_counts();
    assert_eq!(
        (
            after_alias.held,
            after_alias.io,
            after_alias.joining,
            after_alias.joined_total
        ),
        (
            held_before_alias.held,
            held_before_alias.io,
            held_before_alias.joining,
            held_before_alias.joined_total
        )
    );
    assert!(witness.matches_call(&call));
    assert!(!wrong.matches_call(&call));
    denied(call.admit(wrong), Status::Denied);
    assert!(success(call.admit(witness)).view().origin_id > 0);
    denied(
        foreign.h.start_native_read(
            &own.live[0].bindings.artifact.peer,
            &foreign.request(0, 21),
            1,
        ),
        Status::Denied,
    );
    denied(
        own.h.start_native_read(
            &own.live[0].bindings.self_status.peer,
            &own.request(0, 22),
            1,
        ),
        Status::Denied,
    );
    for offset in [8usize, 16, 24, 32] {
        let mut request = own.request(0, 23);
        let mut field = u64::from_le_bytes(request.payload[offset..offset + 8].try_into().unwrap());
        field = field.checked_add(1).unwrap();
        request.payload[offset..offset + 8].copy_from_slice(&field.to_le_bytes());
        denied(
            own.h
                .start_native_read(&own.live[0].bindings.artifact.peer, &canonical(request), 1),
            Status::Denied,
        );
    }
    // No constructor, peer, or opaque proof is synthesized from the views above.
    drop(call);
    drop(binding);
    drop(alien);
    let original_join_id = join_desktop(&own, entry.producer_ids()[0]);
    let joined_once = own.h.native_producer_counts();
    assert_eq!(
        (joined_once.held, joined_once.joined_total),
        (
            held_before_alias.held - 1,
            held_before_alias.joined_total + 1
        )
    );
    denied(own.h.join_native_finished(&alias), Status::NotReady);
    denied(witness.duplicate_producer(&alias), Status::NotReady);
    let alias_rejected = own.h.native_producer_counts();
    assert_eq!(
        (
            alias_rejected.held,
            alias_rejected.io,
            alias_rejected.joining,
            alias_rejected.joined_total
        ),
        (
            joined_once.held,
            joined_once.io,
            joined_once.joining,
            joined_once.joined_total
        )
    );
    let remaining_joins = own.join_all();
    assert_eq!(remaining_joins.len() + 1, 2);
    assert_ne!(remaining_joins[0]["id"].as_u64().unwrap(), original_join_id);
    assert_eq!(
        own.h.native_producer_counts().joined_total,
        held_before_alias.joined_total + 2
    );
    assert!(foreign.join_all().is_empty());
    // Retired references are diagnostic tokens, not original or producer holders.
    // More than 64 sequential fully joined originals must recycle origin slots.
    drop(entry);
    let mut retired_aliases = vec![alias];
    let mut sequential_join_ids = std::collections::BTreeSet::new();
    for serial in 0..65u64 {
        let next = own.entry(0, 200 + serial);
        let original = success(next.wait());
        let retired = success(witness.duplicate_producer(next.producer_ids()[0]));
        for id in next.producer_ids() {
            assert!(sequential_join_ids.insert(join_desktop(&own, id)));
        }
        drop(original);
        drop(next);
        retired_aliases.push(retired);
        assert_eq!(own.h.native_producer_counts().held, 0);
        denied(
            witness.duplicate_producer(retired_aliases.last().unwrap()),
            Status::NotReady,
        );
    }
    assert_eq!(sequential_join_ids.len(), 130);
    assert_eq!(retired_aliases.len(), 66);
    // This tests slot recycling; it does not saturate 64 simultaneous origins.
    drop(retired_aliases);

    let mut f = Fixture::new("native_read_identity_denials", 2);
    let mut g = Fixture::new("native_read_identity_denials", 1);
    let call = success(f.rig.entry(0, 24).wait());
    denied(
        f.host().register_native_read(&f.endpoint(1).peer, call),
        StoreStatus::Denied,
    );
    let call = success(f.rig.entry(0, 25).wait());
    denied(
        g.host().register_native_read(&g.endpoint(0).peer, call),
        StoreStatus::Denied,
    );
    let call = success(f.rig.entry(0, 26).wait());
    denied(
        f.host().register_native_read(&g.endpoint(0).peer, call),
        StoreStatus::Denied,
    );
    assert_unchanged(&f);
    assert_unchanged(&g);
    f.positive(0, 27);
    f.positive(1, 28);
    g.positive(0, 29);
    f.observations.push(json!({"kind":"identity_denials","registry":true,"core":true,"peer":true,"class":true,"generation_fields":4}));
    f.finish();
    g.finish();
}

#[test]
fn native_read_control_denials() {
    let mut f = Fixture::new("native_read_control_denials", 1);
    let b = &f.rig.live[0].boot;
    let handle = f.rig.live[0].bindings.artifact.handle;
    let unsupported = [
        success(
            artifact::AllocateSaveId {
                request_id: 30,
                session_id: b.session_id,
                session_generation: b.session_generation,
                expected_revision: 1,
            }
            .encode(handle),
        ),
        success(
            artifact::Commit {
                request_id: 31,
                session_id: b.session_id,
                session_generation: b.session_generation,
                expected_revision: 1,
                operation_id: 1,
                source_shm: 1,
                byte_len: 64,
                reserved: 0,
            }
            .encode(handle),
        ),
        success(
            artifact::SaveStatus {
                request_id: 32,
                session_id: b.session_id,
                session_generation: b.session_generation,
                operation_id: 1,
            }
            .encode(handle),
        ),
        success(
            artifact::ReadSelectedReply {
                request_id: 33,
                revision: 0,
                data_shm: 0,
                object_generation: 0,
                byte_len: 0,
                status: 0,
            }
            .encode(handle),
        ),
    ];
    for q in unsupported {
        denied(
            f.rig
                .h
                .start_native_read(&f.rig.live[0].bindings.artifact.peer, &canonical(q), 1),
            Status::Unsupported,
        );
    }
    let base = f.rig.request(0, 34);
    let mut unknown_protocol = base;
    unknown_protocol.protocol = 0xffff_fffe;
    let mut unknown_operation = base;
    unknown_operation.msg_type = 99;
    for q in [unknown_protocol, unknown_operation] {
        denied(
            f.rig
                .h
                .start_native_read(&f.rig.live[0].bindings.artifact.peer, &q, 1),
            Status::Unsupported,
        );
    }
    let mut short = base;
    short.payload_len = 39;
    let mut long = base;
    long.payload_len = 65;
    let mut tail = base;
    tail.payload[40] = 1;
    let mut wide = base;
    wide.handle.generation += 1u64 << 32;
    for q in [short, long, tail, wide] {
        denied(
            f.rig
                .h
                .start_native_read(&f.rig.live[0].bindings.artifact.peer, &q, 1),
            Status::Invalid,
        );
    }
    let wire = success(desktop::encode_envelope_wire(&base));
    for (offset, value) in [(84usize, 1u8), (14, 1), (60, 1)] {
        let mut raw = wire;
        raw[offset] = value;
        // 60 is the first payload-tail byte; 14 lies in raw handle reserved bits.
        denied(desktop::decode_envelope_wire(&raw), Status::Invalid);
    }
    denied(desktop::decode_envelope_wire(&wire[..87]), Status::Invalid);
    let mut reserved = success(
        artifact::Commit {
            request_id: 35,
            session_id: b.session_id,
            session_generation: b.session_generation,
            expected_revision: 1,
            operation_id: 1,
            source_shm: 1,
            byte_len: 64,
            reserved: 0,
        }
        .encode(handle),
    );
    reserved.payload[52] = 1;
    denied(
        f.rig
            .h
            .start_native_read(&f.rig.live[0].bindings.artifact.peer, &reserved, 1),
        Status::Invalid,
    );
    assert_eq!(f.rig.h.native_producer_counts().held, 0);
    assert_unchanged(&f);
    f.positive(0, 36);
    f.observations.push(json!({"kind":"control_denials","unsupported_ops":[3,5,7,2,99],"malformed":true,"no_mutation":true}));
    f.finish();
}

#[test]
fn native_read_constructor_recheck() {
    // Exercise the exact same-held-gate two-binding accessor, then a real paired
    // constructor. A second admit while the first guard is held is not used.
    let mut rig = Rig::new(2);
    let a = rig.binding(0);
    let b = rig.binding(1);
    let guard = success(a.admit(rig.witness.as_ref().unwrap()));
    assert_eq!(guard.view().actor, rig.views[0].actor);
    assert_eq!(success(guard.view_binding(&b)).actor, rig.views[1].actor);
    drop(guard);
    ui_a::revoke(&rig.h, &rig.drivers[1].s, &rig.live[1].boot, 2);
    let guard = success(a.admit(rig.witness.as_ref().unwrap()));
    denied(guard.view_binding(&b), Status::Stale);
    drop(guard);
    let root = temporary();
    let path = root.path().join("owner");
    let failure = match store::StoreFixture::create_native_read(
        &path,
        profile(&rig.views),
        rig.witness.take().unwrap(),
        vec![a, b],
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("retired second binding exposed grants"),
    };
    let observed = failure.observation();
    assert_eq!(observed.status, StoreStatus::Stale);
    assert_eq!(observed.stage, store::ReadInitStage::Validate);
    assert_eq!(observed.root, store::ReadRootObservation::NotAttempted);
    assert!(!path.exists());
    assert_eq!(rig.h.native_producer_counts().held, 0);
    assert!(rig.join_all().is_empty());
    drop(failure);
    drop(rig);
    success(root.close());

    for changed in 0..5 {
        let mut rig = Rig::new(1);
        let binding = rig.binding(0);
        let root = temporary();
        let path = root.path().join("owner");
        let mut p = profile(&rig.views);
        match changed {
            0 => p.initial_objects[0].owner_id += 1,
            1 => p.initial_objects[0].object_id += 1,
            2 => p.initial_objects[0].generation += 1,
            3 => p.initial_objects[0].revision += 1,
            4 => p.initial_objects[0].bytes = b"different\n".to_vec(),
            _ => unreachable!(),
        }
        let failure = match store::StoreFixture::create_native_read(
            &path,
            p,
            rig.witness.take().unwrap(),
            vec![binding],
        ) {
            Err(failure) => failure,
            Ok(_) => panic!("mismatched profile exposed native grants"),
        };
        assert_eq!(failure.observation().status, StoreStatus::Denied);
        assert_eq!(
            failure.observation().root,
            store::ReadRootObservation::NotAttempted
        );
        assert!(!path.exists());
        assert_eq!(rig.h.native_producer_counts().held, 0);
        assert!(rig.join_all().is_empty());
        drop(failure);
        drop(rig);
        success(root.close());
    }
    // Retain the actual partial Core/witness failure holder, without deleting or
    // reopening an already existing root to make initialization succeed.
    let mut rig = Rig::new(1);
    let binding = rig.binding(0);
    let root = temporary();
    let path = root.path().join("owner");
    success(std::fs::create_dir(&path));
    success(std::fs::write(
        path.join("sentinel"),
        b"owned-before-constructor",
    ));
    let before = files(&path);
    let failure = match store::StoreFixture::create_native_read(
        &path,
        profile(&rig.views),
        rig.witness.take().unwrap(),
        vec![binding],
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("existing root was silently adopted/deleted"),
    };
    let observation = failure.observation();
    assert_eq!(observation.status, StoreStatus::Internal);
    assert_eq!(observation.stage, store::ReadInitStage::CreateRoot);
    assert!(observation.core_retained && !observation.panicked);
    assert_eq!(
        observation.root,
        store::ReadRootObservation::AttemptUncertain
    );
    assert_eq!(observation.root_path.as_deref(), Some(path.as_path()));
    assert_eq!(files(&path), before);
    assert_eq!(rig.h.native_producer_counts().held, 0);
    assert!(rig.join_all().is_empty());
    drop(failure);
    drop(rig);
    success(root.close());
    let mut positive = Fixture::new("native_read_constructor_recheck", 2);
    positive.positive(0, 40);
    positive.positive(1, 41);
    positive.finish();
}

#[test]
fn native_read_legacy_descriptor_denials() {
    let mut f = Fixture::new("native_read_legacy_descriptor_denials", 1);
    let entry = f.rig.entry(0, 50);
    let reg = f.register(0, &entry);
    let reply = success(f.host().execute_native_read(&reg));
    let selected = reply.selected().unwrap();
    let desc = selected.descriptor();
    let canonical_bytes = checked_selected(&reply, &f.rig.views[0], A, 50);
    let mut q = f.rig.request(0, 51);
    q.handle = f.endpoint(0).handle;
    denied(
        f.host().dispatch(&f.endpoint(0).peer, &canonical(q)),
        StoreStatus::Denied,
    );
    denied(
        f.host().read_lease(&f.endpoint(0).peer, &desc),
        StoreStatus::Denied,
    );
    denied(
        f.host().write_lease(&f.endpoint(0).peer, &desc),
        StoreStatus::Denied,
    );
    denied(
        f.host()
            .draft_source(&f.endpoint(0).peer, 1, b"forbidden\n"),
        StoreStatus::Denied,
    );
    let spec = store::GrantSpec {
        actor: f.rig.views[0].actor.clone(),
        selected_object_id: f.rig.views[0].object_id,
        selected_generation: f.rig.views[0].object_generation,
        rights: 1,
        ttl_ms: 30_000,
    };
    let e = f
        .native()
        .controller()
        .issue_editor(spec)
        .err()
        .expect("native Core denied generic issuance");
    assert_eq!(e.status, StoreStatus::Denied);
    let e = f
        .native()
        .controller()
        .issue_recovery(f.rig.views[0].actor.clone(), f.rig.views[0].object_id, 1)
        .err()
        .expect("native Core denied recovery issuance");
    assert_eq!(e.status, StoreStatus::Denied);
    let e = f
        .native()
        .controller()
        .quiesce(500)
        .err()
        .expect("native Core denied legacy quiescence fence");
    assert_eq!(e.status, StoreStatus::Denied);
    let mut variants = vec![];
    for kind in [
        store::SharedObjectKind::DraftSource,
        store::SharedObjectKind::Receipt,
    ] {
        let mut d = desc.clone();
        d.kind = kind;
        variants.push(d);
    }
    let mut d = desc.clone();
    d.handle = Handle::INVALID;
    variants.push(d);
    let mut d = desc.clone();
    d.handle.index = d.handle.index.checked_add(1u32 << 16).unwrap();
    variants.push(d);
    let mut d = desc.clone();
    d.handle.generation = d.handle.generation.checked_add(1u64 << 32).unwrap();
    variants.push(d.clone());
    d.object_generation = d.handle.generation;
    variants.push(d);
    let mut d = desc.clone();
    d.object_generation = d.object_generation.checked_add(1).unwrap();
    variants.push(d);
    let mut d = desc.clone();
    d.handle.index = d.handle.index.checked_add(1).unwrap();
    variants.push(d);
    let mut d = desc.clone();
    d.byte_len = d.byte_len.checked_add(1).unwrap();
    variants.push(d);
    for d in variants {
        unchanged_denial(selected, &d, StoreStatus::Denied);
    }
    let mut out = [0xa5; 8];
    denied(
        selected.copy_checked(&desc, desc.byte_len, &mut out),
        StoreStatus::Invalid,
    );
    assert_eq!(out, [0xa5; 8]);
    let mut retained = vec![0xa5; desc.byte_len as usize];
    success(selected.copy_checked(&desc, 0, &mut retained));
    assert_eq!(retained, canonical_bytes);
    assert_unchanged(&f);
    f.observations.push(json!({"kind":"descriptor_denials","typed_aliases":9,"legacy_paths":7,"actual_selected":hex(&retained)}));
    drop(reply);
    f.finish();
}

#[test]
fn native_read_once_deadline() {
    let mut f = Fixture::new("native_read_once_deadline", 1);
    let entry = f.rig.entry(0, 60);
    let call = success(entry.wait());
    let entered = call.entered();
    let deadline = call.deadline();
    assert_eq!(
        deadline.duration_since(entered),
        Duration::from_millis(1000)
    );
    denied(entry.wait(), Status::NotReady);
    let reg = success(f.host().register_native_read(&f.endpoint(0).peer, call));
    denied(
        f.host().close_native_read_deadline(&reg),
        StoreStatus::NotReady,
    );
    let reply = success(f.host().execute_native_read(&reg));
    let past_bytes = checked_selected(&reply, &f.rig.views[0], A, 60);
    zero_reply(
        &success(f.host().execute_native_read(&reg)),
        60,
        StoreStatus::NotReady,
    );
    wait_past(deadline);
    let mut denied_output = [0xa5; 8];
    denied(
        reply.selected().unwrap().copy_into(0, &mut denied_output),
        StoreStatus::Timeout,
    );
    assert_eq!(denied_output, [0xa5; 8]);
    success(f.host().close_native_read_deadline(&reg));
    assert_eq!(
        &past_bytes[TEXT_HEADER_LEN..],
        A,
        "already observed data remains observable"
    );
    assert_unchanged(&f);
    // Delayed registration/execution uses the same original deadline; Store must
    // not start a new 1000ms window when it receives the origin.
    let entry = f.rig.entry(0, 61);
    let call = success(entry.wait());
    let deadline = call.deadline();
    let reg = success(f.host().register_native_read(&f.endpoint(0).peer, call));
    wait_past(deadline);
    let expired = success(f.host().execute_native_read(&reg));
    zero_reply(&expired, 61, StoreStatus::Timeout);
    success(f.host().close_native_read_deadline(&reg));
    f.positive(0, 62);
    f.observations.push(json!({"kind":"deadline","original_ms":1000,"late_query_status":8,"repeat_status":6,"past_observation_hash":hex(&digest(&past_bytes))}));
    drop(reply);
    drop(expired);
    f.finish();
}

#[test]
fn native_read_retirement() {
    for mode in 0..5 {
        let mut f = Fixture::new("native_read_retirement", 2);
        let entry = f.rig.entry(0, 70);
        let reg = f.register(0, &entry);
        let old = success(f.host().execute_native_read(&reg));
        let past = checked_selected(&old, &f.rig.views[0], A, 70);
        let pause = f.pause_store(0);
        let entry = f.rig.entry(0, 71);
        let reg = Arc::new(f.register(0, &entry));
        let h = f.host();
        let r = reg.clone();
        let caller = std::thread::spawn(move || h.execute_native_read(&r));
        let barrier = success(f.store_pauses[pause].wait_until_entered(500));
        assert!(barrier.entered && !barrier.settled && !barrier.permit_issued);
        assert_eq!(barrier.request_id, 71);
        match mode {
            0 => ui_a::revoke(&f.rig.h, &f.rig.drivers[0].s, &f.rig.live[0].boot, 101),
            1 => success(f.rig.c.fault_editor(f.rig.live[0].boot.instance_id, 101)),
            2 => success(f.rig.h.maintenance(f.rig.views[0].expires_at_ms)),
            3 => success(
                f.native()
                    .controller()
                    .retire(f.endpoint(0), store::RetirementReason::Revoked),
            ),
            4 => success(
                f.native()
                    .controller()
                    .retire(f.endpoint(0), store::RetirementReason::ServiceRetired),
            ),
            _ => unreachable!(),
        }
        // Release deterministic work before assertions that could unwind while
        // the external caller still holds the real in-flight registration.
        f.release_store(pause);
        let reply = success(success(caller.join()));
        zero_reply(&reply, 71, StoreStatus::Stale);
        let settled = success(f.store_pauses[pause].wait_until_settled(500));
        assert!(settled.settled);
        let mut output = [0xa5; 8];
        denied(
            old.selected().unwrap().copy_into(0, &mut output),
            StoreStatus::Stale,
        );
        assert_eq!(output, [0xa5; 8]);
        assert_eq!(&past[TEXT_HEADER_LEN..], A);
        if mode <= 2 {
            denied(
                f.rig.h.start_native_read(
                    &f.rig.live[0].bindings.artifact.peer,
                    &f.rig.request(0, 72),
                    101,
                ),
                Status::Stale,
            );
        }
        f.positive(1, 73);
        assert_unchanged(&f);
        f.observations.push(json!({"kind":"retirement","mode":mode,"actual_barrier_entered":true,"actual_barrier_settled":settled.settled,
            "reply":hex(&success(desktop::encode_envelope_wire(&reply.wire()))),"copy_status":4,"other_object_success":true}));
        drop(old);
        drop(reply);
        drop(reg);
        f.finish();
    }
}

#[test]
fn native_read_owned_handles() {
    let mut f = Fixture::new("native_read_owned_handles", 1);
    let pause = f.rig.pause(0);
    let entry = f.rig.entry(0, 80);
    let barrier = success(f.rig.pauses[pause].wait_until_entered(500));
    assert!(barrier.entered && !barrier.settled);
    assert_ne!(barrier.dispatcher, barrier.supervisor);
    assert_ne!(barrier.origin_id, 0);
    assert_eq!(
        (
            f.rig.h.native_producer_counts().held,
            f.rig.h.native_producer_counts().io
        ),
        (2, 0)
    );
    denied(entry.wait(), Status::Timeout);
    assert_eq!(
        f.rig.h.native_producer_counts().held,
        2,
        "expired but unjoined actual handles remain charged"
    );
    // Borrow opaque actual IDs even after the result is Timeout; numeric barrier
    // fields are diagnostics and never used to manufacture a join authority.
    let ids = entry.producer_ids();
    // NativeReadEntry's fixed pair is dispatcher then supervisor. Its entered
    // dispatcher is still held in the owned pause and cannot have finished.
    denied(f.rig.h.join_native_finished(ids[0]), Status::NotReady);
    let still_held = f.rig.h.native_producer_counts();
    assert_eq!(
        (still_held.held, still_held.io, still_held.joining),
        (2, 0, 0)
    );
    f.rig.release(pause);
    assert!(success(f.rig.pauses[pause].wait_until_settled(500)).settled);
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut proof_ids = vec![];
    for id in ids {
        let proof = loop {
            match f.rig.h.join_native_finished(id) {
                Ok(proof) => break proof,
                Err(Status::NotReady) => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                Err(e) => panic!("join: {e:?}"),
            }
        };
        assert_eq!(proof.outcome(), desktop::JoinOutcome::Returned);
        proof_ids.push(proof.producer_id());
        denied(f.rig.h.join_native_finished(id), Status::NotReady);
    }
    assert_ne!(proof_ids[0], proof_ids[1]);
    assert_eq!(
        (proof_ids[0], proof_ids[1]),
        (barrier.dispatcher, barrier.supervisor)
    );
    let mut actual_ids = proof_ids.clone();
    actual_ids.sort_unstable();
    let mut observed_ids = vec![barrier.dispatcher, barrier.supervisor];
    observed_ids.sort_unstable();
    assert_eq!(
        actual_ids, observed_ids,
        "joined actual handles bind the entered service producers"
    );
    assert_eq!(f.rig.h.native_producer_counts().held, 0);
    assert_unchanged(&f);
    f.positive(0, 81);
    f.observations.push(json!({"kind":"owned_handles","after_timeout_charged":2,"unfinished_dispatcher_join_status":6,"after_unfinished_join_held":still_held.held,"actual_join_ids":proof_ids,"caller_threads_counted":false}));
    f.finish();
}

#[test]
fn native_read_counters_domains() {
    for counter in [
        desktop::NativeCounter::Binding,
        desktop::NativeCounter::Origin,
        desktop::NativeCounter::Producer,
    ] {
        let mut f = Fixture::new("native_read_counters_domains", 1);
        // Existing live observation and actual join ownership must survive the
        // seeded domain's failed next transition; no metadata-only wrap check.
        let entry = f.rig.entry(0, 90);
        let reg = f.register(0, &entry);
        let reply = success(f.host().execute_native_read(&reg));
        let before = checked_selected(&reply, &f.rig.views[0], A, 90);
        if counter as u32 == desktop::NativeCounter::Binding as u32 {
            f.rig.add_session(2, B);
        }
        let held = f.rig.h.native_producer_counts().held;
        success(f.rig.c.seed_native_counter(counter));
        for _ in 0..2 {
            match counter {
                desktop::NativeCounter::Binding => denied(
                    f.rig
                        .h
                        .approved_native_read(&f.rig.live[1].bindings.artifact.peer, 100),
                    Status::Exhausted,
                ),
                desktop::NativeCounter::Origin | desktop::NativeCounter::Producer => denied(
                    f.rig.h.start_native_read(
                        &f.rig.live[0].bindings.artifact.peer,
                        &f.rig.request(0, 91),
                        100,
                    ),
                    Status::Exhausted,
                ),
            }
        }
        assert_eq!(
            f.rig.h.native_producer_counts().held,
            held,
            "no partial producer installation on overflow"
        );
        assert_eq!(checked_selected(&reply, &f.rig.views[0], A, 90), before);
        assert_unchanged(&f);
        let joins = f.rig.join_all();
        assert_eq!(joins.len() as u32, held);
        f.observations.push(json!({"kind":"counter","domain":counter as u32,"repeat_status":5,"held_before":held,"held_after_failed_transition":held,
            "actual_old_read_hash":hex(&digest(&before)),"actual_join_count":joins.len()}));
        drop(reply);
        f.finish();
    }
    let mut f = Fixture::new("native_read_counters_domains", 1);
    let mut entries = vec![];
    let mut entered_ids = std::collections::BTreeSet::new();
    for i in 0..32 {
        let pause = f.rig.pause(0);
        let entry = f.rig.entry(0, 100 + i);
        let barrier = success(f.rig.pauses[pause].wait_until_entered(500));
        assert!(barrier.entered && !barrier.settled);
        assert_ne!(barrier.dispatcher, 0);
        assert_ne!(barrier.supervisor, 0);
        assert!(entered_ids.insert(barrier.dispatcher));
        assert!(entered_ids.insert(barrier.supervisor));
        entries.push(entry);
        assert_eq!(f.rig.h.native_producer_counts().held, 2 * (i + 1) as u32);
    }
    denied(entries[0].wait(), Status::Timeout);
    let counts = f.rig.h.native_producer_counts();
    assert_eq!((counts.held, counts.io), (64, 0));
    for _ in 0..2 {
        denied(
            f.rig.h.start_native_read(
                &f.rig.live[0].bindings.artifact.peer,
                &f.rig.request(0, 140),
                1,
            ),
            Status::Exhausted,
        );
    }
    assert_eq!(
        (
            f.rig.h.native_producer_counts().held,
            f.rig.h.native_producer_counts().io
        ),
        (64, 0)
    );
    assert_unchanged(&f);
    f.rig.release_all();
    for token in &f.rig.pauses {
        assert!(success(token.wait_until_settled(500)).settled);
    }
    let mut joined = std::collections::BTreeSet::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    for entry in &entries {
        for id in entry.producer_ids() {
            let proof = loop {
                match f.rig.h.join_native_finished(id) {
                    Ok(proof) => break proof,
                    Err(Status::NotReady) => {
                        assert!(Instant::now() < deadline);
                        std::thread::yield_now();
                    }
                    Err(e) => panic!("capacity actual join: {e:?}"),
                }
            };
            assert_eq!(proof.outcome(), desktop::JoinOutcome::Returned);
            assert!(joined.insert(proof.producer_id()));
        }
    }
    assert_eq!(joined.len(), 64);
    assert_eq!(joined, entered_ids);
    assert_eq!(f.rig.h.native_producer_counts().held, 0);
    f.observations.push(json!({"kind":"capacity","paused_entries":32,"actual_held":64,"io":0,"repeat_full_status":5,
        "actual_unique_joins":joined.into_iter().collect::<Vec<_>>(),"held_after_join":0,"origin64_or_binding16_saturation_claim":false}));
    drop(entries);
    f.finish();
    let mut unrelated = Fixture::new("native_read_counters_domains", 1);
    unrelated.positive(0, 150);
    let entry = unrelated.rig.entry(0, 151);
    let reg = unrelated.register(0, &entry);
    let reply = success(unrelated.host().execute_native_read(&reg));
    let past = checked_selected(&reply, &unrelated.rig.views[0], A, 151);
    success(
        unrelated
            .rig
            .c
            .seed_exhaustion(desktop::CounterKind::TimeOrigin),
    );
    let mut sentinel = [0xa5; 8];
    denied(
        reply.selected().unwrap().copy_into(0, &mut sentinel),
        StoreStatus::Stale,
    );
    assert_eq!(sentinel, [0xa5; 8]);
    assert_eq!(&past[TEXT_HEADER_LEN..], A);
    denied(
        unrelated.rig.h.start_native_read(
            &unrelated.rig.live[0].bindings.artifact.peer,
            &unrelated.rig.request(0, 152),
            1,
        ),
        Status::Stale,
    );
    assert_unchanged(&unrelated);
    unrelated.observations.push(json!({"kind":"native_time_origin_seed","copy_status":4,"sentinel_unchanged":true,"past_observation_retained":true}));
    drop(reply);
    drop(reg);
    drop(entry);
    unrelated.finish();
}
