//! Supplemental owner/recovery regressions; original Store inventories stay intact.
#![cfg(feature = "editor_native_save_v0_dev")]
#[allow(dead_code)]
#[path = "../../desktop/tests/support/editor_host_assertions.rs"]
mod ui_a;
use ui_a as ui;
#[allow(dead_code)]
#[path = "support/editor_store.rs"]
mod legacy;
use desktop_service::editor_dev::{self as desktop, EditorMessage};
use kernel_api::generated::desktop_artifact_v1 as a;
use std::time::{Duration, Instant};
use store_service::editor_store::{self as store, StoreStatus};

struct NativeRig {
    root: tempfile::TempDir,
    desktop: desktop::HostDesktop,
    controller: desktop::FixtureController,
    drivers: Vec<ui::Driver>,
    store: store::PreparedNativeSaveStore,
}
impl NativeRig {
    fn new() -> Self {
        let (desktop, controller, witness, enrollment) = store_ok(
            desktop::HostDesktop::new_store_save(ui::config(30000, 600000)),
        );
        let mut drivers = Vec::new();
        for i in 0..2 {
            let session = store_ok(controller.register_store_session(
                i + 1,
                ui::APP,
                ui::MANIFEST,
                101 + i,
                1,
                1,
            ));
            let mut driver = ui::Driver {
                s: session,
                queue: 0,
                next: 1,
                mods: 0,
            };
            driver.attach(&desktop, 1);
            drivers.push(driver);
        }
        #[cfg(target_os = "macos")]
        let root = store_ok(tempfile::tempdir_in("/System/Volumes/Data/private/tmp"));
        #[cfg(not(target_os = "macos"))]
        let root = store_ok(tempfile::tempdir());
        let store = store_ok(store::StoreFixture::prepare_native_save(
            &root.path().join("owner"),
            store::NativeSaveFixtureProfile {
                origin_ms: 1,
                initial_objects: (0..2)
                    .map(|i| store::InitialObject {
                        owner_id: i + 1,
                        object_id: 101 + i,
                        generation: 1,
                        revision: 1,
                        bytes: if i == 0 {
                            b"first\n".to_vec()
                        } else {
                            b"second\n".to_vec()
                        },
                    })
                    .collect(),
                fixture_signing_seed: [0x53; 32],
                fixture_key_id: "reliability-fixture".into(),
            },
            witness,
            enrollment,
        ));
        Self {
            root,
            desktop,
            controller,
            drivers,
            store,
        }
    }
    fn launch(&mut self, i: usize) -> store::PairedNativeSaveLaunch {
        let driver = &mut self.drivers[i];
        driver.launcher(&self.desktop, 1);
        driver.send(&self.desktop, 40, 1, 1);
        let mut selector = store_ok(self.desktop.take_save_preview_action(&driver.s.chrome.peer));
        let pin = store_ok(
            self.store
                .pin_selection(&driver.s.chrome.peer, 101 + i as u64, 30000),
        );
        store_ok(
            self.desktop
                .prepare_save_preview(&driver.s.chrome.peer, &mut selector, &pin, 1),
        );
        driver.send(&self.desktop, 40, 2, 1);
        driver.send(&self.desktop, 40, 1, 1);
        let mut approval = store_ok(self.desktop.take_save_preview_action(&driver.s.chrome.peer));
        driver.send(&self.desktop, 40, 2, 1);
        let pending = store_ok(self.desktop.confirm_save_preview(
            &driver.s.chrome.peer,
            &mut approval,
            1,
        ));
        let active = store_ok(self.store.activate_save(&pending));
        let mut launch = None;
        store_ok(
            self.desktop
                .take_save_launch(&driver.s.chrome.peer, &mut launch, 1),
        );
        store_ok(active.pair_launch(&mut launch))
    }
    fn read(
        &self,
        launch: &store::PairedNativeSaveLaunch,
        id: u64,
    ) -> (desktop::NativeSaveEntry, store::RegisteredNativeSave) {
        let bootstrap = store_ok(
            kernel_api::generated::desktop_editor_session_v1::InstanceBootstrap::decode(
                launch.desktop().bindings().bootstrap(),
            ),
        );
        let request = store_ok(
            a::ReadSelected {
                request_id: id,
                session_id: bootstrap.session_id,
                session_generation: bootstrap.session_generation,
                instance_id: bootstrap.instance_id,
                instance_generation: bootstrap.instance_generation,
            }
            .encode(launch.endpoint().handle),
        );
        let entry = store_ok(self.desktop.begin_save_read(
            launch.desktop().bindings().artifact_origin(),
            &request,
            1,
        ));
        let registered = store_ok(
            self.store
                .host()
                .register_native_save(launch.endpoint(), store_ok(entry.wait_once())),
        );
        (entry, registered)
    }
    fn join(&self, entry: &desktop::NativeSaveEntry, reply: &store::NativeSaveReply) {
        for id in entry.producer_ids().into_iter().chain(reply.producers()) {
            let end = Instant::now() + Duration::from_secs(2);
            loop {
                match self.store.host().join_native_save_finished(id) {
                    Ok(joined) => {
                        assert_eq!(joined.outcome(), desktop::JoinOutcome::Returned);
                        break;
                    }
                    Err(StoreStatus::NotReady) => {
                        assert!(Instant::now() < end);
                        std::thread::yield_now();
                    }
                    Err(error) => panic!("join failed: {error:?}"),
                }
            }
        }
    }
}
fn store_ok<T, E>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|_| panic!("required positive failed"))
}
fn selected(reply: &store::NativeSaveReply, expected: &[u8]) {
    let wire = store_ok(a::ReadSelectedReply::decode(reply.wire()));
    assert_eq!(wire.status, 0, "approved concurrent Read must succeed");
    let mut bytes = vec![0; wire.byte_len as usize];
    store_ok(
        reply
            .selected()
            .expect("actual owned selected lease")
            .copy_into(0, &mut bytes),
    );
    assert_eq!(&bytes[64..], expected);
}
#[test]
fn store_pending_selected_reply_survives_reclaim_and_dropped_owners_release_capacity() {
    let mut rig = NativeRig::new();
    let first = rig.launch(0);
    let second = rig.launch(1);
    let pause = store_ok(
        rig.store
            .controller()
            .pause_next(first.endpoint(), store::PausePoint::BeforeReadReply),
    );
    let (entry_a, call_a) = rig.read(&first, 11);
    let running_a = store_ok(rig.store.host().start_native_save(&call_a));
    assert!(store_ok(pause.wait_until_entered(2000)).entered);
    let (entry_b, call_b) = rig.read(&second, 12);
    let reply_b = store_ok(rig.store.host().execute_native_save(&call_b));
    selected(&reply_b, b"second\n");
    rig.join(&entry_b, &reply_b);
    store_ok(rig.store.controller().release(&pause));
    let deadline = Instant::now() + Duration::from_secs(2);
    let reply_a = loop {
        if let Some(reply) = store_ok(running_a.poll_once()) {
            break reply;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    selected(&reply_a, b"first\n");
    rig.join(&entry_a, &reply_a);
    drop((reply_b, call_b, entry_b));
    for id in 20..60 {
        let (entry, call) = rig.read(&second, id);
        let reply = store_ok(rig.store.host().execute_native_save(&call));
        selected(&reply, b"second\n");
        selected(&reply_a, b"first\n");
        rig.join(&entry, &reply);
        assert!(
            store_ok(rig.store.controller().evidence())
                .ledger
                .shared_objects
                <= 32
        );
        drop((entry, call, reply));
    }
    assert_eq!(rig.desktop.native_producer_counts().held, 0);
    drop((reply_a, running_a, call_a, entry_a));
    let (entry, call) = rig.read(&first, 80);
    let reply = store_ok(rig.store.host().execute_native_save(&call));
    selected(&reply, b"first\n");
    rig.join(&entry, &reply);
    drop((entry, call, reply));
    let owner = store_ok(rig.store.controller().quiesce_native_save(2000));
    drop(owner);
    let _ = (&rig.root, &rig.controller);
}

#[test]
fn store_journal_sync_failure_reopens_after_actual_joins_and_publishes_again() {
    let mut fixture = legacy::Fixture::new();
    let actor = legacy::actor(1, 1);
    let endpoint = fixture.endpoint(&actor, 31, 7, 600000);
    let before = std::fs::read(fixture.root().join("object-31/selection.json")).unwrap();
    store_ok(
        fixture
            .control
            .fail_next_io(&endpoint, store::IoPoint::JournalSync),
    );
    let request = a::AllocateSaveId {
        request_id: 1,
        session_id: actor.session_id,
        session_generation: actor.session_generation,
        expected_revision: 1,
    };
    let reply = legacy::exchange(&fixture.host, &endpoint, request);
    legacy::status(&reply, 16, StoreStatus::Internal);
    let staged = std::fs::read(fixture.root().join("object-31/selection.next")).unwrap();
    assert_ne!(before, staged);
    assert_eq!(
        std::fs::read(fixture.root().join("object-31/selection.json")).unwrap(),
        before
    );
    assert!(
        store_ok(fixture.control.evidence())
            .io_events
            .iter()
            .any(|event| event.point == store::IoPoint::JournalSync
                && matches!(event.outcome, store::IoOutcome::SimulatedFailure))
    );
    let owner = fixture.fence();
    assert_eq!(owner.fence_snapshot().live_ownership_after_join, 0);
    export_investigation(
        (
            "store_journal_sync_failure_reopens_after_actual_joins_and_publishes_again",
            "pre-rename",
        ),
        &before,
        Some(&staged),
        &reply,
        &store_ok(fixture.control.evidence()),
        &owner.fence_snapshot(),
    );
    let failed_stage = store_ok(
        artifact_store_schema::editor_save::EditorSelectionJournalV0::decode_canonical(&staged),
    );
    let failed_operation = failed_stage
        .operations
        .as_slice()
        .last()
        .unwrap()
        .allocation
        .operation_id;
    fixture.reopen(owner);
    export_reopened(
        (
            "store_journal_sync_failure_reopens_after_actual_joins_and_publishes_again",
            "pre-rename",
        ),
        &fixture.root().join("object-31/selection.json"),
    );
    let recovered = fixture.object(31);
    assert!(
        recovered
            .operations
            .iter()
            .all(|row| row.allocation.operation_id != failed_operation)
    );
    assert_eq!(fixture.counts(31), (0, 0, 0));
    let fresh = fixture.endpoint(&actor, 31, 7, 600000);
    let operation = legacy::attempt(&fixture, &fresh, &actor, 1, legacy::NEW, 100);
    assert!(
        operation.operation > failed_operation,
        "failed allocation identity is never reused"
    );
    let reply = legacy::exchange(&fixture.host, &fresh, operation.request);
    let receipt = legacy::receipt(&fixture.host, &fresh, &reply);
    assert_eq!(
        receipt.outcome,
        artifact_store_schema::editor_save::EditorReceiptOutcomeV0::Committed
    );
    assert_eq!(fixture.counts(31), (1, 1, 1));
}

#[test]
fn store_journal_staging_rejects_corrupt_foreign_and_symlink_replacements() {
    for replacement in 0..4 {
        let mut fixture = legacy::Fixture::new();
        let actor = legacy::actor(1, 1);
        let endpoint = fixture.endpoint(&actor, 31, 7, 600000);
        store_ok(
            fixture
                .control
                .fail_next_io(&endpoint, store::IoPoint::JournalSync),
        );
        let reply = legacy::exchange(
            &fixture.host,
            &endpoint,
            a::AllocateSaveId {
                request_id: 1,
                session_id: actor.session_id,
                session_generation: actor.session_generation,
                expected_revision: 1,
            },
        );
        legacy::status(&reply, 16, StoreStatus::Internal);
        let owner = fixture.fence();
        let path = fixture.root().join("object-31/selection.next");
        let owned = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        match replacement {
            0 => std::fs::write(&path, b"corrupt").unwrap(),
            1 => std::fs::write(
                &path,
                std::fs::read(fixture.root().join("object-32/selection.json")).unwrap(),
            )
            .unwrap(),
            2 => std::os::unix::fs::symlink("selection.json", &path).unwrap(),
            // Same bytes under a different file identity are not the held writer's staging.
            3 => std::fs::write(&path, owned).unwrap(),
            _ => unreachable!(),
        }
        let before_selection =
            std::fs::read(fixture.root().join("object-31/selection.json")).unwrap();
        let failure = match store::StoreFixture::reopen(owner) {
            Ok(_) => panic!("foreign or corrupt staging must deny before publication"),
            Err(failure) => failure,
        };
        assert_eq!(failure.error.status, StoreStatus::Unknown);
        assert!(std::fs::symlink_metadata(&path).is_ok());
        assert_eq!(
            std::fs::read(fixture.root().join("object-31/selection.json")).unwrap(),
            before_selection
        );
        drop(failure);
    }
}

struct CommitAttempt {
    entry: desktop::NativeSaveEntry,
    call: store::RegisteredNativeSave,
    operation: u64,
    _intent: desktop::NativeSaveIntent,
    _source: store::NativeDraftSource,
}
impl NativeRig {
    fn editor(&self, launch: &store::PairedNativeSaveLaunch, i: usize) -> desktop::NativeEditor {
        let (entry, call) = self.read(launch, 200);
        let mut reply = store_ok(self.store.host().execute_native_save(&call));
        let editor = store_ok(reply.take_editor());
        self.join(&entry, &reply);
        let frame = store_ok(self.desktop.render_native_editor(&editor, 1));
        store_ok(
            self.desktop
                .compose_native_save(&self.drivers[i].s.compositor, 1),
        );
        let actor = store_ok(self.desktop.editor_observation(&editor, 1)).actor;
        let assigned: kernel_api::generated::desktop_focus_v1::AssignReply = ui::call(
            &self.desktop,
            &self.drivers[i].s.chrome,
            kernel_api::generated::desktop_focus_v1::Assign {
                request_id: 201,
                session_id: actor.session_id,
                session_generation: actor.session_generation,
                instance_id: actor.instance_id,
                instance_generation: actor.instance_generation,
                surface_id: frame.surface_id,
                surface_generation: frame.surface_generation,
            },
            1,
        );
        assert_eq!(assigned.status, 0);
        editor
    }
    fn unchanged_save(
        &mut self,
        launch: &store::PairedNativeSaveLaunch,
        editor: &desktop::NativeEditor,
        i: usize,
    ) -> CommitAttempt {
        for (usage, phase) in [(224, 1), (22, 1), (22, 2), (224, 2)] {
            self.drivers[i].send(&self.desktop, usage, phase, 1);
            store_ok(self.desktop.step_native_editor(editor, 1));
        }
        let intent = store_ok(self.desktop.take_save_intent(editor));
        let view = intent.snapshot().view();
        let request = store_ok(
            a::AllocateSaveId {
                request_id: 300,
                session_id: view.actor.session_id,
                session_generation: view.actor.session_generation,
                expected_revision: view.expected_revision,
            }
            .encode(launch.endpoint().handle),
        );
        let entry = store_ok(self.desktop.begin_save_allocate(
            launch.desktop().bindings().artifact_origin(),
            &intent,
            &request,
            1,
        ));
        let call = store_ok(
            self.store
                .host()
                .register_native_save(launch.endpoint(), store_ok(entry.wait_once())),
        );
        let reply = store_ok(self.store.host().execute_native_save(&call));
        let allocation = store_ok(a::AllocateSaveIdReply::decode(reply.wire()));
        assert_eq!(allocation.status, 0);
        let source = store_ok(self.store.host().source_for_intent(
            launch.endpoint(),
            reply.allocation().unwrap(),
            &intent,
        ));
        let header = store_ok(
            artifact_store_schema::editor_save::EditorTextHeaderV0::try_new(
                view.object_id,
                view.expected_revision,
                intent.snapshot().body(),
            ),
        );
        let mut bytes = store_ok(header.encode_le()).to_vec();
        bytes.extend_from_slice(intent.snapshot().body());
        store_ok(store_ok(source.write_lease()).copy_from(0, &bytes));
        self.join(&entry, &reply);
        let descriptor = source.descriptor();
        let request = store_ok(
            a::Commit {
                request_id: 301,
                session_id: view.actor.session_id,
                session_generation: view.actor.session_generation,
                expected_revision: view.expected_revision,
                operation_id: allocation.operation_id,
                source_shm: descriptor.handle.pack(),
                byte_len: descriptor.byte_len,
                reserved: 0,
            }
            .encode(launch.endpoint().handle),
        );
        let entry = store_ok(self.desktop.begin_save_commit(
            launch.desktop().bindings().artifact_origin(),
            source.bound(),
            &request,
            1,
        ));
        let call = store_ok(
            self.store
                .host()
                .register_native_save(launch.endpoint(), store_ok(entry.wait_once())),
        );
        CommitAttempt {
            entry,
            call,
            operation: allocation.operation_id,
            _intent: intent,
            _source: source,
        }
    }
}
#[test]
fn store_pending_receipt_survives_reclaim_without_mutation_replay() {
    let mut rig = NativeRig::new();
    let first = rig.launch(0);
    let second = rig.launch(1);
    let editor = rig.editor(&first, 0);
    let attempt = rig.unchanged_save(&first, &editor, 0);
    let pause = store_ok(
        rig.store
            .controller()
            .pause_native_save(first.endpoint(), store::PausePoint::AfterAcknowledgement),
    );
    let running = store_ok(rig.store.host().start_native_save(&attempt.call));
    assert!(store_ok(pause.wait_until_entered(2000)).entered);
    let (entry_b, call_b) = rig.read(&second, 400);
    let reply_b = store_ok(rig.store.host().execute_native_save(&call_b));
    selected(&reply_b, b"second\n");
    rig.join(&entry_b, &reply_b);
    store_ok(rig.store.controller().release(&pause));
    let deadline = Instant::now() + Duration::from_secs(2);
    let reply = loop {
        if let Some(reply) = store_ok(running.poll_once()) {
            break reply;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    let wire = store_ok(a::CommitReply::decode(reply.wire()));
    assert_eq!(wire.status, 0);
    let lease = reply.receipt().expect("actual transferred receipt owner");
    let mut bytes = [0; 176];
    store_ok(lease.copy_into(0, &mut bytes));
    let receipt =
        store_ok(artifact_store_schema::editor_save::EditorSaveReceiptV0::decode_le(&bytes));
    assert_eq!(receipt.operation_id, attempt.operation);
    assert_eq!(receipt.result_revision, 2);
    rig.join(&attempt.entry, &reply);
    drop((reply_b, call_b, entry_b));
    for id in 410..450 {
        let (entry, call) = rig.read(&second, id);
        let other = store_ok(rig.store.host().execute_native_save(&call));
        selected(&other, b"second\n");
        store_ok(lease.copy_into(0, &mut bytes));
        rig.join(&entry, &other);
    }
    let evidence = store_ok(rig.store.controller().evidence());
    let object = evidence
        .objects
        .iter()
        .find(|object| object.selected.selected_object_id == 101)
        .unwrap();
    assert_eq!(
        (
            object.mutation_dispatch_count,
            object.permit_count,
            object.transition_count
        ),
        (1, 1, 1)
    );
    assert_eq!(rig.desktop.native_producer_counts().held, 0);
    drop((reply, running, attempt));
    drop(store_ok(rig.store.controller().quiesce_native_save(2000)));
}

#[test]
fn store_native_journal_failures_preserve_original_outcomes_without_replay() {
    for (point, pause_point, committed) in [
        (
            store::IoPoint::JournalSync,
            store::PausePoint::AfterCommitPermit,
            false,
        ),
        (
            store::IoPoint::JournalDirectorySync,
            store::PausePoint::BeforeSelectionRename,
            true,
        ),
    ] {
        let mut rig = NativeRig::new();
        let first = rig.launch(0);
        let second = rig.launch(1);
        let editor = rig.editor(&first, 0);
        let attempt = rig.unchanged_save(&first, &editor, 0);
        let recovery = store_ok(
            rig.store
                .controller()
                .recovery_for_original(&rig.drivers[0].s.chrome.peer, attempt.call.original()),
        );
        let pause = store_ok(
            rig.store
                .controller()
                .pause_native_save(first.endpoint(), pause_point),
        );
        let running = store_ok(rig.store.host().start_native_save(&attempt.call));
        assert!(store_ok(pause.wait_until_entered(2000)).permit_issued);
        store_ok(rig.store.controller().fail_next_io(first.endpoint(), point));
        store_ok(rig.store.controller().release(&pause));
        let deadline = Instant::now() + Duration::from_secs(2);
        let reply = loop {
            if let Some(reply) = store_ok(running.poll_once()) {
                break reply;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        let wire = store_ok(a::CommitReply::decode(reply.wire()));
        assert_eq!(
            (wire.status, wire.operation_id),
            (StoreStatus::Unknown as u32, attempt.operation)
        );
        assert!(reply.receipt().is_none());
        let object_root = rig.root.path().join("owner/object-101");
        assert_eq!(object_root.join("selection.next").exists(), !committed);
        let disk = store_ok(
            artifact_store_schema::editor_save::EditorSelectionJournalV0::decode_canonical(
                &std::fs::read(object_root.join("selection.json")).unwrap(),
            ),
        );
        assert_eq!(disk.current.revision, if committed { 2 } else { 1 });
        let evidence = store_ok(rig.store.controller().evidence());
        assert!(
            evidence
                .io_events
                .iter()
                .any(|event| event.selected_object_id == 101
                    && event.point == point
                    && matches!(event.outcome, store::IoOutcome::SimulatedFailure))
        );
        rig.join(&attempt.entry, &reply);
        // An unrelated consumer progresses while the original object remains uncertain.
        let (entry_b, call_b) = rig.read(&second, 500);
        let reply_b = store_ok(rig.store.host().execute_native_save(&call_b));
        selected(&reply_b, b"second\n");
        rig.join(&entry_b, &reply_b);
        drop((entry_b, call_b, reply_b));
        let owner = store_ok(rig.store.controller().quiesce_native_save(2000));
        assert_eq!(owner.fence_snapshot().base.live_ownership_after_join, 0);
        let selected_disk = std::fs::read(object_root.join("selection.json")).unwrap();
        let staged_disk = std::fs::read(object_root.join("selection.next")).ok();
        export_investigation(
            (
                "store_native_journal_failures_preserve_original_outcomes_without_replay",
                if committed {
                    "post-rename"
                } else {
                    "pre-rename"
                },
            ),
            &selected_disk,
            staged_disk.as_deref(),
            reply.wire(),
            &evidence,
            &owner.fence_snapshot().base,
        );
        rig.store = store_ok(store::StoreFixture::reopen_native_save(owner));
        export_reopened(
            (
                "store_native_journal_failures_preserve_original_outcomes_without_replay",
                if committed {
                    "post-rename"
                } else {
                    "pre-rename"
                },
            ),
            &object_root.join("selection.json"),
        );
        assert!(!object_root.join("selection.next").exists());
        let actor = attempt._intent.snapshot().view().actor;
        let status = store_ok(
            a::SaveStatus {
                request_id: 600,
                session_id: actor.session_id,
                session_generation: actor.session_generation,
                operation_id: attempt.operation,
            }
            .encode(first.endpoint().handle),
        );
        let entry = store_ok(
            rig.desktop
                .begin_save_recovery(recovery.origin(), &status, 1),
        );
        let observed = store_ok(
            rig.store
                .host()
                .recover_original(store_ok(entry.wait_once()), &recovery),
        );
        let receipt = match observed.observation() {
            store::NativeOriginalObservation::Receipt { binding, receipt } => {
                assert_eq!(binding.allocation.operation_id, attempt.operation);
                let mut bytes = [0; 176];
                store_ok(receipt.copy_into(0, &mut bytes));
                store_ok(artifact_store_schema::editor_save::EditorSaveReceiptV0::decode_le(&bytes))
            }
            _ => panic!("joined original must reconcile to one complete receipt"),
        };
        assert_eq!(receipt.operation_id, attempt.operation);
        assert_eq!(
            receipt.outcome,
            if committed {
                artifact_store_schema::editor_save::EditorReceiptOutcomeV0::Committed
            } else {
                artifact_store_schema::editor_save::EditorReceiptOutcomeV0::DefinitiveNoncommit
            }
        );
        for id in entry.producer_ids().into_iter().chain(observed.producers()) {
            let end = Instant::now() + Duration::from_secs(2);
            loop {
                match rig.store.host().join_native_save_finished(id) {
                    Ok(proof) => {
                        assert_eq!(proof.outcome(), desktop::JoinOutcome::Returned);
                        break;
                    }
                    Err(StoreStatus::NotReady) => {
                        assert!(Instant::now() < end);
                        std::thread::yield_now();
                    }
                    Err(error) => panic!("actual recovery join: {error:?}"),
                }
            }
        }
        let before = store_ok(rig.store.controller().evidence());
        let object = before
            .objects
            .iter()
            .find(|object| object.selected.selected_object_id == 101)
            .unwrap();
        assert_eq!(
            (
                object.mutation_dispatch_count,
                object.permit_count,
                object.transition_count
            ),
            (1, 1, u32::from(committed))
        );
        // A distinct explicit operation works on the reopened owner.
        let next = rig.launch(0);
        let next_editor = rig.editor(&next, 0);
        let fresh = rig.unchanged_save(&next, &next_editor, 0);
        assert!(fresh.operation > attempt.operation);
        let saved = store_ok(rig.store.host().execute_native_save(&fresh.call));
        assert_eq!(store_ok(a::CommitReply::decode(saved.wire())).status, 0);
        rig.join(&fresh.entry, &saved);
        let after = store_ok(rig.store.controller().evidence());
        let object = after
            .objects
            .iter()
            .find(|object| object.selected.selected_object_id == 101)
            .unwrap();
        assert_eq!(
            (
                object.mutation_dispatch_count,
                object.permit_count,
                object.transition_count
            ),
            (2, 2, u32::from(committed) + 1)
        );
        drop((saved, fresh, observed, entry, reply, running, attempt));
        assert_eq!(rig.desktop.native_producer_counts().held, 0);
        drop(store_ok(rig.store.controller().quiesce_native_save(2000)));
    }
}

// Optional gate diagnostics retain real disk/result/trace observations before
// reopen consumes the original owner. These bytes confer no cleanup authority.
fn export_investigation(
    scope: (&str, &str),
    selected: &[u8],
    staged: Option<&[u8]>,
    original_reply: &kernel_api::ipc::Envelope,
    evidence: &store::StoreEvidence,
    fence: &store::FenceSnapshot,
) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let (case, leg) = scope;
    let Some(destination) = std::env::var_os("RAMEN_NATIVE_TASK_EVIDENCE") else {
        return;
    };
    let root = std::path::PathBuf::from(destination);
    assert!(root.is_absolute());
    let metadata = std::fs::symlink_metadata(&root).unwrap();
    assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    let parent = root.join(case);
    match std::fs::create_dir(&parent) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = std::fs::symlink_metadata(&parent).unwrap();
            assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
        }
        Err(error) => panic!("fresh diagnostic parent: {error:?}"),
    }
    let directory = parent.join(leg);
    std::fs::create_dir(&directory).expect("fresh bounded investigation leg");
    assert!(selected.len() <= 131072 && staged.is_none_or(|bytes| bytes.len() <= 131072));
    assert!(evidence.io_events.len() <= 256);
    assert!(fence.service_producers.len() <= 64 && fence.object_workers.len() <= 2);
    let trace = serde_json::to_vec(&evidence.io_events.iter().map(|event| serde_json::json!({
        "selected_object_id": event.selected_object_id, "operation_id": event.operation_id,
        "writer_id": event.writer_id, "sequence": event.sequence,
        "point": format!("{:?}", event.point), "outcome": match event.outcome {
            store::IoOutcome::Began => "Began", store::IoOutcome::Completed => "Completed",
            store::IoOutcome::SimulatedFailure => "SimulatedFailure", store::IoOutcome::ActualFailure => "ActualFailure",
        },
        "artifact_hash": event.artifact_hash,
    })).collect::<Vec<_>>()).unwrap();
    assert_eq!(fence.live_ownership_after_join, 0);
    let fence_bytes = serde_json::to_vec(fence).unwrap();
    let wire = store_ok(store::encode_envelope_wire(original_reply));
    let nonce = std::env::var("RAMEN_NATIVE_TASK_RUN_NONCE").ok();
    assert!(nonce.as_ref().is_none_or(
        |nonce| nonce.len() == 32 && nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
    ));
    let result = serde_json::to_vec(&serde_json::json!({
        "run_nonce": nonce,
        "schema_version": 1, "scope": "actual-host-fixture-joined-owner",
        "selected_byte_len": selected.len(), "staged_byte_len": staged.map(<[u8]>::len),
        "original_message_type": original_reply.msg_type,
        "original_status": u32::from_le_bytes(original_reply.payload[if original_reply.msg_type == 4 { 16..20 } else { 32..36 }].try_into().unwrap()),
        "live_ownership_after_join": fence.live_ownership_after_join,
    })).unwrap();
    let mut files = vec![
        ("selection.json", selected),
        ("original-reply.bin", wire.as_slice()),
        ("io-events.json", trace.as_slice()),
        ("fence.json", fence_bytes.as_slice()),
        ("result.json", result.as_slice()),
    ];
    if let Some(staged) = staged {
        files.push(("selection.next", staged));
    }
    for (name, bytes) in files {
        assert!(bytes.len() <= 2 * 1024 * 1024);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(directory.join(name))
            .unwrap();
        file.write_all(bytes).unwrap();
    }
}

fn export_reopened(scope: (&str, &str), actual_selection: &std::path::Path) {
    use std::io::{Read, Write};
    use std::os::unix::fs::OpenOptionsExt;
    let Some(destination) = std::env::var_os("RAMEN_NATIVE_TASK_EVIDENCE") else {
        return;
    };
    let root = std::path::PathBuf::from(destination);
    assert!(root.is_absolute());
    let directory = root.join(scope.0).join(scope.1);
    for path in [&root, &root.join(scope.0), &directory] {
        let metadata = std::fs::symlink_metadata(path).unwrap();
        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    }
    let mut selection = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(actual_selection)
        .unwrap();
    let metadata = selection.metadata().unwrap();
    assert!(metadata.is_file() && metadata.len() <= 131072);
    let mut selected = Vec::new();
    (&mut selection)
        .take(131073)
        .read_to_end(&mut selected)
        .unwrap();
    assert!(selected.len() <= 131072);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(directory.join("reopened-selection.json"))
        .unwrap();
    file.write_all(&selected).unwrap();
}
