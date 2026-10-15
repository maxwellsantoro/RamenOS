//! Consuming-join staging against the frozen API. A borrower panic is caught
//! while this owner remains outside the unwind boundary. Cleanup uncertainty
//! retains the child, launch, Store Core, actual IO owners and temporary root.

use crate::support::Fixture;
#[path = "cleanup_owners.rs"]
mod cleanup_owners;
use cleanup_owners::ParentOwners;
use desktop_service::dev::Status;
use desktop_service::editor_dev::{
    OwnedEditorChild, OwnedEditorChildExitProof, ProcessRetireReason,
};
use std::time::Instant;
use store_service::editor_store::PairedNativeSaveLaunch;

pub struct OwnedProcessFixture {
    parents: Option<ParentOwners<(Fixture, PairedNativeSaveLaunch)>>,
    child: Option<OwnedEditorChild>,
}

impl OwnedProcessFixture {
    pub fn new(fixture: Fixture, child: OwnedEditorChild, launch: PairedNativeSaveLaunch) -> Self {
        Self {
            parents: Some(ParentOwners::new((fixture, launch))),
            child: Some(child),
        }
    }

    pub fn borrow(&mut self) -> (&mut Fixture, &mut OwnedEditorChild) {
        let (fixture, _) = self.parents.as_mut().unwrap().borrow();
        (fixture, self.child.as_mut().unwrap())
    }

    /// Authored runtime assertion: a live nonterminal child must be returned by
    /// the consuming API promptly. Parent ownership remains outside that call.
    /// This is deliberately unexecutable while the process API is missing.
    pub fn assert_live_join_not_ready(&mut self) {
        let child = self
            .child
            .take()
            .expect("sole live child before consuming join");
        let entered = Instant::now();
        let (fixture, _) = self.parents.as_mut().unwrap().borrow();
        match fixture.h.join_process_editor(child) {
            Err(failure) => {
                let (status, child) = failure.into_parts();
                self.child = Some(child);
                assert_eq!(status, Status::NotReady);
            }
            Ok(_proof) => panic!("live child unexpectedly terminal before retire or expiry"),
        }
        assert!(
            entered.elapsed() < std::time::Duration::from_millis(100),
            "consuming NotReady must not wait for child expiry"
        );
    }

    /// No caller waiter, numeric-PID action or Rust-thread substitute is used.
    /// A failed consuming join supplies the sole actual owner for retry/return.
    /// The runtime implementation must itself honor this consuming API's
    /// non-unwinding owner-return and bounded nonblocking-NotReady contract.
    pub fn finish(mut self, now: u64) -> Result<OwnedEditorChildExitProof, Self> {
        if self.child.is_none() {
            // A faulty consuming API may have unwound without returning its
            // opaque child. No external fixture can reconstruct that authority.
            return Err(self);
        }
        let parents = self.parents.as_mut().unwrap();
        let (end, first) = parents.begin_cleanup();
        if first {
            let (f, _) = parents.borrow();
            let child = self.child.as_mut().unwrap();
            let _ =
                f.h.retire_process_editor(child, ProcessRetireReason::HumanClose, now);
        }
        loop {
            if !self.parents.as_ref().unwrap().join_permitted() {
                return Err(self);
            }
            let child = self.child.take().unwrap();
            let (f, _) = self.parents.as_mut().unwrap().borrow();
            match f.h.join_process_editor(child) {
                Ok(proof) => {
                    let (fixture, launch) = self.parents.take().unwrap().release_after_join();
                    drop(launch);
                    fixture.finish();
                    return Ok(proof);
                }
                Err(failure) => {
                    let (status, child) = failure.into_parts();
                    self.child = Some(child);
                    if status != Status::NotReady || Instant::now() >= end {
                        return Err(self);
                    }
                    std::thread::yield_now();
                }
            }
        }
    }
}

impl Drop for OwnedProcessFixture {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            // Unproved wait is not absence. In particular, don't let Fixture's
            // temporary directory delete data beneath an actual live writer.
            std::mem::forget(child);
            eprintln!("INCOMPLETE: retaining actual child; process reap unproved");
        }
        // ParentOwners quarantines even if a faulty consuming join unwound
        // after taking the child slot. Only the genuine Ok proof releases it.
    }
}
