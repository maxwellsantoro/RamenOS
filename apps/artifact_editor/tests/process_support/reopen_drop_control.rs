//! The existing consuming-reopen helper must not turn an opaque failure into
//! permission to delete the parent root. This is trusted in-process preparation.

use super::{fixture, support};
use store_service::editor_store as store;
use support::*;

#[test]
fn process_preparation_failed_reopen_drop_retains_root() {
    let name = "process_preparation_failed_reopen_drop_retains_root";
    let recording = ok(recording::CaseRecorder::open(name));
    let mut f = Fixture::recorded(&recording, fixture::preparation_spec(name), 1);
    ok(f.prepared()
        .controller()
        .seed_counter(101, store::CounterKind::ServiceEpoch));
    let root = f
        .root
        .as_ref()
        .expect("actual parent root")
        .path()
        .to_owned();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.reopen()));
    assert!(
        caught.is_err(),
        "the original consuming reopen rejects MAX epoch"
    );
    assert!(root.is_dir(), "borrowed fixture still owns the actual root");
    drop(f);
    assert!(
        root.is_dir(),
        "unreturned opaque reopen ownership requires quarantine"
    );
    // Deliberately do not remove this quarantined root: neither the panic nor
    // the pathname is a returned Store owner or a successful cleanup proof.
    let mut spec = fixture::preparation_spec(name);
    spec.scenario = "after_failed_reopen_drop";
    spec.leg = 1;
    let mut healthy = Fixture::recorded(&recording, spec, 1);
    let (launch, editor) = fixture::initialize(&mut healthy, 1);
    assert_eq!(ok(healthy.h.editor_observation(&editor, 1)).bytes, OLD);
    drop((launch, editor));
    healthy.finish();
}
