//! Actual opaque Store failure owners remain outside borrowed unwind scopes.
//! These default-off host controls do not establish an editor-child boundary.

use super::{fixture, support};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};
use store_service::editor_store::{self as store, StoreStatus};
use support::*;

fn panic_message(caught: std::thread::Result<()>, expected: &'static str) {
    let panic = caught.expect_err("the explicit borrowed-owner panic must occur");
    assert_eq!(panic.downcast_ref::<&str>().copied(), Some(expected));
}

fn healthy_read(recording: &Arc<recording::CaseRecorder>, name: &'static str) {
    let mut spec = fixture::preparation_spec(name);
    spec.scenario = "independent_healthy_read";
    spec.leg = 1;
    let mut f = Fixture::recorded(recording, spec, 1);
    let (launch, editor) = fixture::initialize(&mut f, 1);
    assert_eq!(ok(f.h.editor_observation(&editor, 1)).bytes, OLD);
    assert_eq!(f.counts(101), (0, 0, 0));
    drop((launch, editor));
    f.finish();
}

fn init_observation(
    failure: &store::NativeSaveInitFailure,
) -> (
    store::PreviewInitStage,
    StoreStatus,
    bool,
    store::ReadRootObservation,
    Option<PathBuf>,
    bool,
    bool,
) {
    let actual = failure.observation();
    (
        actual.stage,
        actual.status,
        actual.panicked,
        actual.root,
        actual.root_path,
        actual.core_retained,
        actual.issuer_retained,
    )
}

#[test]
fn process_preparation_init_validation_failure_retains_issuer() {
    let name = "process_preparation_init_validation_failure_retains_issuer";
    let recording = ok(recording::CaseRecorder::open(name));
    let mut invalid = profile(1);
    invalid.initial_objects.clear();
    let failure = match Fixture::try_recorded_profile_with_root_probe(
        &recording,
        fixture::preparation_spec(name),
        1,
        invalid,
        |_| {},
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("empty actual Store profile must fail validation"),
    };
    let root = failure.root_path().to_owned();
    let before = init_observation(failure.failure());
    assert_eq!(
        before,
        (
            store::PreviewInitStage::Validate,
            StoreStatus::Invalid,
            false,
            store::ReadRootObservation::NotAttempted,
            Some(root.join("owner")),
            false,
            true,
        )
    );
    assert!(!root.join("owner").exists());
    panic_message(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_eq!(init_observation(failure.failure()), before);
            panic!("borrowed init validation failure retains actual issuer");
        })),
        "borrowed init validation failure retains actual issuer",
    );
    assert_eq!(init_observation(failure.failure()), before);
    drop(failure);
    assert!(
        root.is_dir(),
        "no returned init close proof; quarantine actual root"
    );
    healthy_read(&recording, name);
}

#[test]
fn process_preparation_init_create_root_failure_retains_core() {
    let name = "process_preparation_init_create_root_failure_retains_core";
    let recording = ok(recording::CaseRecorder::open(name));
    let failure = match Fixture::try_recorded_profile_with_root_probe(
        &recording,
        fixture::preparation_spec(name),
        1,
        profile(1),
        |root| ok(fs::create_dir(root.join("owner"))),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("preexisting owner directory must fail actual create_root"),
    };
    let root = failure.root_path().to_owned();
    let before = init_observation(failure.failure());
    assert_eq!(
        before,
        (
            store::PreviewInitStage::CreateRoot,
            StoreStatus::Internal,
            false,
            store::ReadRootObservation::AttemptUncertain,
            Some(root.join("owner")),
            true,
            true,
        )
    );
    panic_message(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert!(root.join("owner").is_dir());
            assert_eq!(init_observation(failure.failure()), before);
            panic!("borrowed init create failure retains actual Core and issuer");
        })),
        "borrowed init create failure retains actual Core and issuer",
    );
    assert_eq!(init_observation(failure.failure()), before);
    drop(failure);
    assert!(
        root.is_dir(),
        "uncertain create failure cannot authorize root removal"
    );
    assert!(root.join("owner").is_dir());
    healthy_read(&recording, name);
}

fn same_fence(before: &store::NativeSaveFenceSnapshot, after: &store::NativeSaveFenceSnapshot) {
    assert_eq!(before.schema_version, after.schema_version);
    assert_eq!(
        ok(serde_json::to_vec(&before.base)),
        ok(serde_json::to_vec(&after.base))
    );
    assert_eq!(before.original_tickets.len(), after.original_tickets.len());
    for (a, b) in before.original_tickets.iter().zip(&after.original_tickets) {
        assert_eq!(a.original_request, b.original_request);
        assert_eq!((a.origin_id, a.intent_id), (b.origin_id, b.intent_id));
        assert_eq!(a.kind as u8, b.kind as u8);
        assert_eq!(a.actor, b.actor);
        assert_eq!(
            (a.object_id, a.original_epoch),
            (b.object_id, b.original_epoch)
        );
        assert_eq!(a.allocation, b.allocation);
        assert_eq!(a.binding, b.binding);
        assert_eq!(
            (a.closed, a.commit_dispatched),
            (b.closed, b.commit_dispatched)
        );
        assert_eq!(a.completion as u8, b.completion as u8);
    }
}

fn genuine_save(f: &mut Fixture) {
    let (mut app, endpoint) = f.app_with_endpoint(0, 101, 1);
    f.replace(0, &mut app, NEW, 2);
    let actor = ok(app.observe(2)).actor;
    f.save(0, &mut app, 2);
    assert_eq!(f.counts(101), (1, 1, 1));
    let operation = object(f, 101).operations[0].allocation.operation_id;
    app_chain_artifacts(f, 101, NEW, operation, &actor, endpoint.handle);
    assert_eq!(selected_blob(f, 101), NEW);
    drop((app, endpoint));
}

// O_NOFOLLOW is platform-specific. This fixture supports the repository's
// Linux/macOS host lanes; its held descriptor is never a runtime repair API.
#[cfg(target_os = "linux")]
const NOFOLLOW: i32 = 0x20000;
#[cfg(target_os = "macos")]
const NOFOLLOW: i32 = 0x100;

struct HeldJournal {
    path: PathBuf,
    file: File,
    dev: u64,
    ino: u64,
    bytes: Vec<u8>,
}

impl HeldJournal {
    fn open(root: &Path) -> Self {
        let matching = files(root)
            .into_iter()
            .filter(|(path, _)| path.file_name().is_some_and(|p| p == "selection.json"))
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 1, "one genuine Store object journal");
        let (relative, expected) = matching.into_iter().next().unwrap();
        assert!(!expected.is_empty() && expected.len() <= 131072);
        let path = root.join(relative);
        let named = ok(fs::symlink_metadata(&path));
        assert!(named.file_type().is_file());
        let mut file = ok(OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(NOFOLLOW)
            .open(&path));
        let held = ok(file.metadata());
        assert!(held.file_type().is_file());
        assert_eq!((held.dev(), held.ino()), (named.dev(), named.ino()));
        assert_eq!(held.len(), expected.len() as u64);
        let mut bytes = vec![0; expected.len()];
        ok(file.read_exact(&mut bytes));
        assert_eq!(bytes, expected);
        Self {
            path,
            file,
            dev: held.dev(),
            ino: held.ino(),
            bytes,
        }
    }

    fn restore(&mut self) {
        let named = ok(fs::symlink_metadata(&self.path));
        let held = ok(self.file.metadata());
        assert!(named.file_type().is_file() && held.file_type().is_file());
        assert_eq!((named.dev(), named.ino()), (self.dev, self.ino));
        assert_eq!((held.dev(), held.ino()), (self.dev, self.ino));
        assert_eq!(held.len(), (self.bytes.len() / 2) as u64);
        ok(self.file.seek(SeekFrom::Start(0)));
        ok(self.file.write_all(&self.bytes));
        ok(self.file.set_len(self.bytes.len() as u64));
        ok(self.file.sync_all());
        assert_eq!(ok(fs::read(&self.path)), self.bytes);
    }
}

#[test]
fn process_preparation_reopen_failure_panic_preserves_owner_then_recovers() {
    let name = "process_preparation_reopen_failure_panic_preserves_owner_then_recovers";
    let mut f = fixture::fixture(name, 1);
    genuine_save(&mut f);
    let first = object(&f, 101).operations[0].clone();
    let root = f.root.as_ref().unwrap().path().to_owned();
    let mut journal = HeldJournal::open(&f.path);
    let mut original = None;
    assert_eq!(
        f.try_reopen_with_probe(|owner| {
            original = Some(owner.fence_snapshot());
            ok(store::StoreFixture::corrupt_native_fenced(
                owner,
                101,
                store::StorageFault::TruncatedJournal,
            ));
        }),
        Err(StoreStatus::Unknown)
    );
    let original = original.expect("actual pre-consumption fence");
    assert!(
        !original.original_tickets.is_empty(),
        "real app Save originals retained"
    );
    assert_eq!(original.base.live_ownership_after_join, 0);
    let failure = f
        .staged_reopen_failure()
        .expect("actual returned opaque failure retained");
    assert_eq!(failure.error.status, StoreStatus::Unknown);
    same_fence(&original, &failure.owner.fence_snapshot());
    let damaged = files(&f.path);
    panic_message(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let failure = f.staged_reopen_failure().expect("borrowed actual failure");
            same_fence(&original, &failure.owner.fence_snapshot());
            panic!("borrowed reopen failure preserves original owner");
        })),
        "borrowed reopen failure preserves original owner",
    );
    assert!(root.is_dir());
    assert_eq!(files(&f.path), damaged);
    same_fence(
        &original,
        &f.staged_reopen_snapshot().expect("same retained fence"),
    );
    // Trusted fixture repair restores only the exact bytes saved before fault,
    // through the same no-follow regular-file descriptor and inode.
    journal.restore();
    ok(f.try_reopen());
    assert!(f.staged_reopen_failure().is_none());
    assert!(f.staged_reopen_snapshot().is_none());
    let launch = f.launch(0, 101, 3);
    let (reply, revision, bytes) = fixture::read(&mut f, &launch, 501, 3);
    assert_eq!((revision, bytes.as_slice()), (2, NEW));
    drop((reply, launch));

    let (mut app, endpoint) = f.app_with_endpoint(0, 101, 4);
    let actor = ok(app.observe(4)).actor;
    f.replace(0, &mut app, OLD, 4);
    f.chord(0, &mut app, 22, 4);
    f.until(&mut app, 4, |o| o.confirmed_revision == 3);
    f.composed(0, &mut app, 4);
    assert_eq!(f.counts(101), (2, 2, 2));
    let durable = object(&f, 101);
    assert_eq!(durable.operations.len(), 2);
    assert_eq!(durable.operations[0].allocation, first.allocation);
    assert_eq!(durable.operations[0].receipt, first.receipt);
    let next = durable.operations.last().expect("fresh genuine app Save");
    assert_ne!(next.allocation.operation_id, first.allocation.operation_id);
    assert_eq!(next.allocation.expected_revision, 2);
    app_chain_artifacts(
        &f,
        101,
        OLD,
        next.allocation.operation_id,
        &actor,
        endpoint.handle,
    );
    assert_eq!(selected_blob(&f, 101), OLD);
    drop((app, endpoint, journal));
    ok(f.try_reopen());
    let launch = f.launch(0, 101, 5);
    let (reply, revision, bytes) = fixture::read(&mut f, &launch, 502, 5);
    assert_eq!((revision, bytes.as_slice()), (3, OLD));
    drop((reply, launch));
    f.finish();
}

#[test]
fn process_preparation_reopen_exhaustion_retry_retains_original_fence() {
    let name = "process_preparation_reopen_exhaustion_retry_retains_original_fence";
    let recording = ok(recording::CaseRecorder::open(name));
    let mut f = Fixture::recorded(&recording, fixture::preparation_spec(name), 1);
    ok(f.prepared()
        .controller()
        .seed_counter(101, store::CounterKind::ServiceEpoch));
    assert_eq!(f.try_reopen(), Err(StoreStatus::Exhausted));
    let original = f
        .staged_reopen_snapshot()
        .expect("actual failure owner fence");
    assert_eq!(original.base.objects.len(), 1);
    assert_eq!(original.base.objects[0].service_epoch_at_fence, u64::MAX);
    assert_eq!(original.base.objects[0].reserved_successor_epoch, None);
    let before = files(&f.path);
    for _ in 0..2 {
        assert_eq!(f.try_reopen(), Err(StoreStatus::Exhausted));
        let failure = f
            .staged_reopen_failure()
            .expect("same actual failure owner");
        assert_eq!(failure.error.status, StoreStatus::Exhausted);
        same_fence(&original, &failure.owner.fence_snapshot());
        assert_eq!(files(&f.path), before, "no epoch-wrap metadata effects");
    }
    // Exhaustion is proved before a successor attempt can exist. Only this
    // known returned failure may use the explicit consuming close control.
    f.finish_staged_reopen_failure();
    healthy_read(&recording, name);
}

#[test]
fn process_preparation_pre_reopen_probe_panic_retains_fence_then_recovers() {
    let name = "process_preparation_pre_reopen_probe_panic_retains_fence_then_recovers";
    let mut f = fixture::fixture(name, 1);
    genuine_save(&mut f);
    let root = f.root.as_ref().unwrap().path().to_owned();
    let before = files(&f.path);
    let original = std::cell::RefCell::new(None);
    panic_message(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = f.try_reopen_with_probe(|owner| {
                *original.borrow_mut() = Some(owner.fence_snapshot());
                panic!("borrowed pre-reopen probe retains actual fence");
            });
        })),
        "borrowed pre-reopen probe retains actual fence",
    );
    let original = original
        .into_inner()
        .expect("real quiesced owner reached borrowed probe");
    assert_eq!(original.base.live_ownership_after_join, 0);
    assert!(root.is_dir());
    assert_eq!(files(&f.path), before);
    assert!(
        f.staged_reopen_failure().is_none(),
        "no consuming attempt occurred"
    );
    same_fence(
        &original,
        &f.staged_reopen_snapshot().expect("pre-call owner retained"),
    );
    ok(f.try_reopen());
    let launch = f.launch(0, 101, 3);
    let (reply, revision, bytes) = fixture::read(&mut f, &launch, 503, 3);
    assert_eq!((revision, bytes.as_slice()), (2, NEW));
    drop((reply, launch));
    f.finish();
}

#[test]
fn process_preparation_consuming_reopen_unwind_quarantines_without_retry() {
    let name = "process_preparation_consuming_reopen_unwind_quarantines_without_retry";
    let recording = ok(recording::CaseRecorder::open(name));
    let mut f = Fixture::recorded(&recording, fixture::preparation_spec(name), 1);
    let (launch, editor) = fixture::initialize(&mut f, 1);
    drop((launch, editor));
    let root = f
        .root
        .as_ref()
        .expect("actual parent root")
        .path()
        .to_owned();
    let before = files(&f.path);
    let core_ordinal = f.core_ordinal;
    // This is a deliberately faulty test consumer after genuine owner transfer,
    // not an injected Store implementation panic or a replacement join proof.
    panic_message(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = f.try_reopen_with_consumer_unwind();
        })),
        "supported consuming reopen policy panic",
    );
    assert!(root.is_dir());
    assert!(
        f.staged_reopen_snapshot().is_none(),
        "the consumer lost the actual fence"
    );
    assert!(
        f.staged_reopen_failure().is_none(),
        "no opaque failure returned"
    );
    let archived = f.recorded_fences.len();
    assert_eq!(
        archived, 1,
        "one real fence archived before consuming transfer"
    );
    for _ in 0..2 {
        assert_eq!(f.try_reopen(), Err(StoreStatus::Unknown));
        assert!(f.staged_reopen_snapshot().is_none());
        assert!(f.staged_reopen_failure().is_none());
        assert_eq!(f.core_ordinal, core_ordinal, "no successor adopted");
        assert_eq!(f.recorded_fences.len(), archived, "no new fence minted");
        assert_eq!(
            files(&f.path),
            before,
            "uncertain owner loss permits no new effects"
        );
    }
    drop(f);
    assert!(
        root.is_dir(),
        "unreturned consuming owner requires parent quarantine"
    );
    // Do not delete the root: this policy control has no returned close owner.
    healthy_read(&recording, name);
}
