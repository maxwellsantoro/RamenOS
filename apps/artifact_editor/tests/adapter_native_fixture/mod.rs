//! Supplemental genuine native Read fixture. No recorder, Save/Commit or public hook.
//! Failed owners are quarantined as INCOMPLETE, never treated as joined or released.
#[allow(dead_code)]
#[path = "../../../../services/desktop/tests/support/editor_host_assertions.rs"]
mod volatile;

use desktop_service::editor_dev::{self as desktop, EditorMessage};
use kernel_api::generated::{
    desktop_artifact_v1 as artifact, desktop_editor_session_v1 as session,
    desktop_focus_v1 as focus,
};
use kernel_api::ipc::Envelope;
use std::{
    collections::BTreeSet,
    fs,
    io::Write as _,
    ops::Deref,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use store_service::editor_store::{self as store, StoreStatus};

const INPUT_LIMIT: usize = 128;
const JOIN_LIMIT: usize = 64;
const OWNER_TIMEOUT: Duration = Duration::from_millis(2000);

fn ok<T, E>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        // Used only for non-owning Status/io errors. Opaque failure owners below
        // are explicitly put in the construction guard before panic.
        Err(_) => panic!("supplemental native fixture operation failed"),
    }
}
fn canonical(request: Envelope) -> Envelope {
    let bytes = ok(desktop::encode_envelope_wire(&request));
    let result = ok(desktop::decode_envelope_wire(&bytes));
    assert_eq!(ok(desktop::encode_envelope_wire(&result)), bytes);
    result
}
fn temporary() -> tempfile::TempDir {
    #[cfg(target_os = "linux")]
    {
        ok(tempfile::Builder::new()
            .prefix("ramenos-adapter-")
            .tempdir())
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        let path = std::path::Path::new("/System/Volumes/Data/private/tmp");
        let metadata = ok(fs::symlink_metadata(path));
        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
        assert_eq!(
            (metadata.uid(), metadata.gid(), metadata.mode() & 0o7777),
            (0, 0, 0o1777)
        );
        ok(tempfile::Builder::new()
            .prefix("ramenos-adapter-")
            .tempdir_in(path))
    }
}

// This group exists before Store IO/Read admission. All opaque success/failure
// owners remain here through constructor unwind; no generic Err drops them.
struct Setup {
    h: desktop::HostDesktop,
    dc: Arc<desktop::FixtureController>,
    root: tempfile::TempDir,
    witness: Option<desktop::RegistryWitness>,
    enrollment: Option<desktop::NativeSaveEnrollment>,
    prepared: Option<Arc<store::PreparedNativeSaveStore>>,
    driver: Option<volatile::Driver>,
    selector: Option<desktop::SaveUiTicket>,
    pin: Option<desktop::SaveSelectionPin>,
    approval: Option<desktop::SaveUiTicket>,
    pending: Option<desktop::PendingSaveInstance>,
    activation: Option<store::NativeSaveActivation>,
    desktop_launch: Option<desktop::DesktopSaveLaunch>,
    paired: Option<store::PairedNativeSaveLaunch>,
    entry: Option<desktop::NativeSaveEntry>,
    registered: Option<store::RegisteredNativeSave>,
    reply: Option<store::NativeSaveReply>,
    editor: Option<desktop::NativeEditor>,
    _init_failure: Option<store::NativeSaveInitFailure>,
    _expose_failure: Option<store::NativeSaveExposeFailure>,
    _registration_failure: Option<store::NativeSaveRegistrationFailure>,
    fence: Option<store::QuiescedNativeSaveOwner>,
    joined: BTreeSet<u64>,
    input_count: usize,
}
struct ConstructionGuard(Option<Setup>);
impl Drop for ConstructionGuard {
    fn drop(&mut self) {
        if let Some(owner) = self.0.take() {
            std::mem::forget(owner);
            let _ = std::io::stderr().write_all(b"INCOMPLETE: constructor retains entire native adapter owner/root; closure unproved\n");
        }
    }
}
impl Setup {
    fn store(&self) -> &store::PreparedNativeSaveStore {
        self.prepared
            .as_deref()
            .expect("actual constructed Store owner")
    }
    fn send(&mut self, usage: u32, phase: u32, now: u64) {
        assert!(
            self.input_count < INPUT_LIMIT,
            "bounded actual input corpus"
        );
        self.input_count += 1;
        self.driver
            .as_mut()
            .expect("actual attached input owner")
            .send(&self.h, usage, phase, now);
    }
    fn join_one(&mut self, id: &desktop::ProducerId, until: Instant) -> Result<(), &'static str> {
        // IDs only deduplicate already accepted actual proofs. Every new join
        // passes its real opaque token to Store; Desktop verifies its proof.
        if self.joined.contains(&id.producer_id()) {
            return Ok(());
        }
        if self.joined.len() >= JOIN_LIMIT {
            return Err("native join token bound");
        }
        loop {
            match self.store().host().join_native_save_finished(id) {
                Ok(proof) => {
                    if !matches!(proof.outcome(), desktop::JoinOutcome::Returned) {
                        return Err("actual producer did not return");
                    }
                    let observation = self
                        .h
                        .native_join_observation(&proof)
                        .map_err(|_| "actual join observation denied")?;
                    if observation.producer_id != id.producer_id()
                        || proof.producer_id() != id.producer_id()
                        || !self.joined.insert(proof.producer_id())
                    {
                        return Err("actual producer proof correlation failed");
                    }
                    return Ok(());
                }
                Err(StoreStatus::NotReady) => {
                    if Instant::now() >= until {
                        return Err("actual join deadline");
                    }
                    thread::yield_now();
                }
                Err(_) => return Err("actual Store join denied"),
            }
        }
    }
    fn join_read_owners(&mut self) {
        // Enumerate genuine opaque aliases only, retaining entry/reply originals.
        // The Store roster includes both Desktop entry and Store execution pairs.
        let until = Instant::now() + OWNER_TIMEOUT;
        let ids = ok(self.store().host().native_save_producers());
        assert!(ids.len() <= JOIN_LIMIT);
        for id in ids {
            ok(self.join_one(&id, until));
        }
        let ids = self.h.native_producers();
        assert!(ids.len() <= JOIN_LIMIT);
        for id in ids {
            ok(self.join_one(&id, until));
        }
        for id in self
            .entry
            .as_ref()
            .expect("retained actual Read entry")
            .producer_ids()
        {
            assert!(
                self.joined.contains(&id.producer_id()),
                "entry producer actually joined"
            );
        }
        for id in self
            .reply
            .as_ref()
            .expect("retained actual Read reply")
            .producers()
        {
            assert!(
                self.joined.contains(&id.producer_id()),
                "reply producer actually joined"
            );
        }
        assert_eq!(self.h.native_producer_counts().held, 0);
    }
    fn cleanup(&mut self) -> Result<(), &'static str> {
        let until = Instant::now() + OWNER_TIMEOUT;
        loop {
            let ids = self
                .store()
                .host()
                .native_save_producers()
                .map_err(|_| "actual Store producer roster denied")?;
            if ids.len() > JOIN_LIMIT {
                return Err("Store producer roster bound");
            }
            for id in ids {
                self.join_one(&id, until)?;
            }
            let ids = self.h.native_producers();
            if ids.len() > JOIN_LIMIT {
                return Err("Desktop producer roster bound");
            }
            for id in ids {
                self.join_one(&id, until)?;
            }
            if self.h.native_producer_counts().held == 0 {
                break;
            }
            if Instant::now() >= until {
                return Err("actual owner cleanup deadline");
            }
            thread::yield_now();
        }
        if self.fence.is_none() {
            self.fence = Some(
                self.store()
                    .controller()
                    .quiesce_native_save(2000)
                    .map_err(|_| "actual Store quiesce incomplete")?,
            );
        }
        let snapshot = self
            .fence
            .as_ref()
            .ok_or("missing actual fence")?
            .fence_snapshot();
        if snapshot.base.live_ownership_after_join != 0 || self.h.native_producer_counts().held != 0
        {
            return Err("actual ownership remains after join");
        }
        // Root deletion happens only after actual supported joins and quiesce.
        // Keep TempDir in the owner group if removal fails; never close by Drop.
        fs::remove_dir_all(self.root.path()).map_err(|_| "joined fixture root removal failed")?;
        match fs::symlink_metadata(self.root.path()) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            _ => Err("fixture root removal not observed"),
        }
    }
}

// Public only within this private integration test module; no product API.
pub(super) struct Ready {
    pub(super) h: desktop::HostDesktop,
    pub(super) dc: Arc<desktop::FixtureController>,
    pub(super) editor: desktop::NativeEditor,
    setup: Setup,
}
pub(super) struct NativeFixture {
    owned: Option<Ready>,
}
impl Deref for NativeFixture {
    type Target = Ready;
    fn deref(&self) -> &Ready {
        self.owned.as_ref().expect("live actual native fixture")
    }
}
impl NativeFixture {
    pub(super) fn new(selected_bytes: &[u8]) -> Self {
        assert!(selected_bytes.len() <= 4096);
        assert!(
            selected_bytes
                .iter()
                .all(|byte| matches!(*byte, 9 | 10 | 32..=126))
        );
        let root = temporary();
        let (h, dc, witness, enrollment) = ok(desktop::HostDesktop::new_store_save(
            volatile::config(30000, 600000),
        ));
        let mut guard = ConstructionGuard(Some(Setup {
            h,
            dc: Arc::new(dc),
            root,
            witness: Some(witness),
            enrollment: Some(enrollment),
            prepared: None,
            driver: None,
            selector: None,
            pin: None,
            approval: None,
            pending: None,
            activation: None,
            desktop_launch: None,
            paired: None,
            entry: None,
            registered: None,
            reply: None,
            editor: None,
            _init_failure: None,
            _expose_failure: None,
            _registration_failure: None,
            fence: None,
            joined: BTreeSet::new(),
            input_count: 0,
        }));
        let owner = guard.0.as_mut().expect("installed constructor owner guard");
        let session =
            ok(owner
                .dc
                .register_store_session(1, volatile::APP, volatile::MANIFEST, 101, 1, 1));
        owner.driver = Some(volatile::Driver {
            s: session,
            queue: 0,
            next: 1,
            mods: 0,
        });
        owner.driver.as_mut().unwrap().attach(&owner.h, 1);
        let profile = store::NativeSaveFixtureProfile {
            origin_ms: 1,
            initial_objects: vec![store::InitialObject {
                owner_id: 1,
                object_id: 101,
                generation: 1,
                revision: 1,
                bytes: selected_bytes.to_vec(),
            }],
            fixture_signing_seed: [0x53; 32],
            fixture_key_id: "native-task-v0-fixture".into(),
        };
        let prepared = store::StoreFixture::prepare_native_save(
            &owner.root.path().join("owner"),
            profile,
            owner.witness.take().unwrap(),
            owner.enrollment.take().unwrap(),
        );
        match prepared {
            Ok(prepared) => owner.prepared = Some(Arc::new(prepared)),
            Err(failure) => {
                owner._init_failure = Some(failure);
                panic!("INCOMPLETE: actual Store initialization owner retained");
            }
        }
        // Genuine Ctrl+Escape launcher, Enter selector and fresh Enter approval.
        for (usage, phase) in [(224, 1), (41, 1), (41, 2), (224, 2), (40, 1)] {
            owner.send(usage, phase, 1);
        }
        owner.selector = Some(ok(owner
            .h
            .take_save_preview_action(&owner.driver.as_ref().unwrap().s.chrome.peer)));
        owner.pin = Some(ok(owner.store().pin_selection(
            &owner.driver.as_ref().unwrap().s.chrome.peer,
            101,
            30000,
        )));
        let prepared = canonical(ok(owner.h.prepare_save_preview(
            &owner.driver.as_ref().unwrap().s.chrome.peer,
            owner.selector.as_mut().unwrap(),
            owner.pin.as_ref().unwrap(),
            1,
        )));
        let prepared = ok(session::PrepareLaunchReply::decode(&prepared));
        assert_eq!(
            (
                prepared.status,
                prepared.granted_rights,
                prepared.preview_len
            ),
            (0, 63, 464)
        );
        owner.send(40, 2, 1);
        owner.send(40, 1, 1);
        owner.approval = Some(ok(owner
            .h
            .take_save_preview_action(&owner.driver.as_ref().unwrap().s.chrome.peer)));
        owner.send(40, 2, 1);
        owner.pending = Some(ok(owner.h.confirm_save_preview(
            &owner.driver.as_ref().unwrap().s.chrome.peer,
            owner.approval.as_mut().unwrap(),
            1,
        )));
        match owner.store().activate_save(owner.pending.as_ref().unwrap()) {
            Ok(activation) => owner.activation = Some(activation),
            Err(failure) => {
                owner._expose_failure = Some(failure);
                panic!("INCOMPLETE: actual activation owner retained");
            }
        }
        ok(owner.h.take_save_launch(
            &owner.driver.as_ref().unwrap().s.chrome.peer,
            &mut owner.desktop_launch,
            1,
        ));
        match owner
            .activation
            .as_ref()
            .unwrap()
            .pair_launch(&mut owner.desktop_launch)
        {
            Ok(paired) => owner.paired = Some(paired),
            Err(failure) => {
                owner._expose_failure = Some(failure);
                panic!("INCOMPLETE: actual paired-launch owner retained");
            }
        }
        assert!(owner.desktop_launch.is_none());
        let paired = owner.paired.as_ref().unwrap();
        let boot = ok(session::InstanceBootstrap::decode(&canonical(
            *paired.desktop().bindings().bootstrap(),
        )));
        let request = canonical(ok(artifact::ReadSelected {
            request_id: 400,
            session_id: boot.session_id,
            session_generation: boot.session_generation,
            instance_id: boot.instance_id,
            instance_generation: boot.instance_generation,
        }
        .encode(paired.endpoint().handle)));
        owner.entry = Some(ok(owner.h.begin_save_read(
            paired.desktop().bindings().artifact_origin(),
            &request,
            1,
        )));
        let call = ok(owner.entry.as_ref().unwrap().wait_once());
        match owner
            .store()
            .host()
            .register_native_save(owner.paired.as_ref().unwrap().endpoint(), call)
        {
            Ok(registered) => owner.registered = Some(registered),
            Err(failure) => {
                owner._registration_failure = Some(failure);
                panic!("INCOMPLETE: actual Read registration owner retained");
            }
        }
        owner.reply = Some(ok(owner
            .store()
            .host()
            .execute_native_save(owner.registered.as_ref().unwrap())));
        let wire = ok(artifact::ReadSelectedReply::decode(
            owner.reply.as_ref().unwrap().wire(),
        ));
        assert_eq!(wire.status, 0);
        owner.editor = Some(ok(owner.reply.as_mut().unwrap().take_editor()));
        owner.join_read_owners();
        let editor = owner.editor.as_ref().unwrap();
        let view = ok(owner.h.editor_observation(editor, 1));
        assert_eq!(view.bytes, selected_bytes);
        assert_eq!(
            (
                view.cursor as usize,
                view.first_visible_line,
                view.text_generation,
                view.view_version
            ),
            (selected_bytes.len(), 0, 1, 1)
        );
        assert!(matches!(view.phase, desktop::NativeEditorPhase::Loaded));
        let frame = ok(owner.h.render_native_editor(editor, 1));
        let composed = ok(owner
            .h
            .compose_native_save(&owner.driver.as_ref().unwrap().s.compositor, 1));
        assert_eq!(composed.instance_id, view.actor.instance_id);
        assert_eq!(composed.sequence, frame.sequence);
        let assigned: focus::AssignReply = volatile::call(
            &owner.h,
            &owner.driver.as_ref().unwrap().s.chrome,
            focus::Assign {
                request_id: 401,
                session_id: view.actor.session_id,
                session_generation: view.actor.session_generation,
                instance_id: view.actor.instance_id,
                instance_generation: view.actor.instance_generation,
                surface_id: frame.surface_id,
                surface_generation: frame.surface_generation,
            },
            1,
        );
        assert_eq!(assigned.status, 0);
        assert_ne!(assigned.focus_epoch, 0);
        // Validate readiness while the guard still owns every token. The final
        // transition below only moves validated fields; it performs no fallible IO.
        assert!(owner.editor.is_some() && owner.driver.is_some() && owner.prepared.is_some());
        let h = owner.h.clone();
        let dc = Arc::clone(&owner.dc);
        let editor = owner.editor.take().unwrap();
        let setup = guard.0.take().unwrap();
        Self {
            owned: Some(Ready {
                h,
                dc,
                editor,
                setup,
            }),
        }
    }
    pub(super) fn send(&mut self, usage: u32, phase: u32, now: u64) {
        self.owned
            .as_mut()
            .expect("live actual fixture")
            .setup
            .send(usage, phase, now);
    }
    pub(super) fn render_and_compose(&mut self, now: u64) -> desktop::NativeFrameObservation {
        let owner = self.owned.as_ref().expect("live actual fixture");
        let before = ok(owner.h.editor_observation(&owner.editor, now));
        let frame = ok(owner.h.render_native_editor(&owner.editor, now));
        let composed = ok(owner
            .h
            .compose_native_save(&owner.setup.driver.as_ref().unwrap().s.compositor, now));
        let after = ok(owner.h.editor_observation(&owner.editor, now));
        assert_eq!(after, before);
        assert_eq!(
            (
                &frame.actor,
                frame.document_id,
                frame.text_generation,
                frame.view_version
            ),
            (
                &before.actor,
                before.document_id,
                before.text_generation,
                before.view_version
            )
        );
        assert_eq!(
            (composed.instance_id, composed.sequence),
            (after.actor.instance_id, frame.sequence)
        );
        assert_eq!(composed.bgra.len(), 640 * 568 * 4);
        assert_eq!(
            volatile::hash(&volatile::crop(&composed)),
            volatile::hex(&frame.app_crop_sha256)
        );
        frame
    }
    pub(super) fn finish(mut self) {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            self.owned.as_mut().unwrap().setup.cleanup()
        }));
        if !matches!(outcome, Ok(Ok(()))) {
            if let Some(owner) = self.owned.take() {
                std::mem::forget(owner);
            }
            panic!(
                "INCOMPLETE: entire native adapter fixture quarantined; actual cleanup unproved"
            );
        }
        // Only actual closure and observed root removal allow ordinary destruction.
        self.owned.take();
    }
}
impl Drop for NativeFixture {
    fn drop(&mut self) {
        if self.owned.is_none() {
            return;
        }
        let already_panicking = thread::panicking();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            self.owned.as_mut().unwrap().setup.cleanup()
        }));
        if !matches!(outcome, Ok(Ok(()))) {
            if let Some(owner) = self.owned.take() {
                std::mem::forget(owner);
                let _ = std::io::stderr().write_all(b"INCOMPLETE: unwind retains entire native adapter owner/root; closure unproved\n");
            }
            if !already_panicking {
                panic!("INCOMPLETE: normal Drop could not prove actual native owner cleanup");
            }
        }
    }
}
