//! Consuming-join staging against the frozen API. A borrower panic is caught
//! while this owner remains outside the unwind boundary. Cleanup uncertainty
//! retains the child, launch, Store Core, actual IO owners and temporary root.

use crate::support::Fixture;
use desktop_service::dev::Status;
use desktop_service::editor_dev::{
    OwnedEditorChild, OwnedEditorChildExitProof, ProcessRetireReason,
};
use std::time::{Duration, Instant};
use store_service::editor_store::PairedNativeSaveLaunch;

pub struct OwnedProcessFixture {
    fixture: Option<Fixture>,
    child: Option<OwnedEditorChild>,
    launch: Option<PairedNativeSaveLaunch>,
}

impl OwnedProcessFixture {
    pub fn new(fixture: Fixture, child: OwnedEditorChild, launch: PairedNativeSaveLaunch) -> Self {
        Self {
            fixture: Some(fixture),
            child: Some(child),
            launch: Some(launch),
        }
    }

    pub fn borrow(&mut self) -> (&mut Fixture, &mut OwnedEditorChild) {
        (self.fixture.as_mut().unwrap(), self.child.as_mut().unwrap())
    }

    /// No caller waiter, numeric-PID action or Rust-thread substitute is used.
    /// A failed consuming join supplies the sole actual owner for retry/return.
    /// The runtime implementation must itself honor this consuming API's
    /// non-unwinding owner-return and bounded nonblocking-NotReady contract.
    pub fn finish(mut self, now: u64) -> Result<OwnedEditorChildExitProof, Self> {
        let end = Instant::now() + Duration::from_millis(2000);
        {
            let f = self.fixture.as_ref().unwrap();
            let child = self.child.as_mut().unwrap();
            let _ =
                f.h.retire_process_editor(child, ProcessRetireReason::HumanClose, now);
        }
        loop {
            let child = self.child.take().unwrap();
            match self.fixture.as_ref().unwrap().h.join_process_editor(child) {
                Ok(proof) => {
                    self.launch.take();
                    self.fixture.take().unwrap().finish();
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
            if let Some(launch) = self.launch.take() {
                std::mem::forget(launch);
            }
            if let Some(fixture) = self.fixture.take() {
                std::mem::forget(fixture);
            }
            eprintln!(
                "INCOMPLETE: retaining actual child/launch/Core/IO/root; process reap unproved"
            );
        }
    }
}
