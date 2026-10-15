//! Preparation-only parent ownership policy, shared by the frozen-API consumer
//! and runnable controls. This type grants no process, wait or Store authority.

use std::time::{Duration, Instant};

pub struct ParentOwners<T> {
    owners: Option<T>,
    cleanup_deadline: Option<Instant>,
}

impl<T> ParentOwners<T> {
    pub fn new(owners: T) -> Self {
        Self {
            owners: Some(owners),
            cleanup_deadline: None,
        }
    }

    pub fn borrow(&mut self) -> &mut T {
        self.owners.as_mut().expect("retained actual parent owners")
    }

    /// Only the first cleanup request starts the budget. Returning the opaque
    /// child after NotReady, or any other uncertainty, cannot renew it.
    pub fn begin_cleanup(&mut self) -> (Instant, bool) {
        match self.cleanup_deadline {
            Some(deadline) => (deadline, false),
            None => {
                let deadline = Instant::now() + Duration::from_millis(2000);
                self.cleanup_deadline = Some(deadline);
                (deadline, true)
            }
        }
    }

    pub fn join_permitted(&self) -> bool {
        self.cleanup_deadline
            .is_some_and(|end| Instant::now() < end)
    }

    /// Caller must first receive the genuine runtime's opaque exit proof.
    /// This helper checks no native facts and cannot manufacture that proof.
    pub fn release_after_join(mut self) -> T {
        self.owners.take().expect("actual parent owner transfer")
    }
}

impl<T> Drop for ParentOwners<T> {
    fn drop(&mut self) {
        if let Some(owners) = self.owners.take() {
            // Independent of whether a consuming call still occupies a child
            // slot. Unwinding from a faulty join cannot authorize root deletion.
            std::mem::forget(owners);
            eprintln!("INCOMPLETE: retaining parent owners; child completion unproved");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_preparation_cleanup_retry_preserves_deadline() {
        let mut owners = ParentOwners::new(());
        let (original, first) = owners.begin_cleanup();
        assert!(first);
        assert!(owners.join_permitted());
        // A return/retry observes a distinct instant; it must not buy time or
        // issue another retire request, even after the original budget expires.
        owners.cleanup_deadline = Some(Instant::now() - Duration::from_millis(1));
        let expired = owners.cleanup_deadline.unwrap();
        assert!(expired < original);
        for _ in 0..3 {
            assert_eq!(owners.begin_cleanup(), (expired, false));
            assert!(
                !owners.join_permitted(),
                "expired retry cannot consume child again"
            );
        }
        owners.release_after_join();
    }

    #[test]
    fn process_preparation_consuming_unwind_retains_parent_root() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().to_owned();
        let root = ParentOwners::new(root);
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _held = root;
            // Models only the test owner's unwind boundary, not waitpid or a
            // real child. The parent root must survive the empty child slot.
            panic!("supported consuming-call policy panic");
        }));
        assert!(failed.is_err());
        assert!(path.is_dir(), "unproved child completion must retain root");
        // This control never creates a child or writer; explicit test cleanup
        // does not stand in for a runtime reap or Store producer join.
        std::fs::remove_dir(&path).unwrap();
    }

    #[test]
    fn process_preparation_joined_cleanup_releases_parent_root() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().to_owned();
        let mut owners = ParentOwners::new(root);
        assert!(owners.borrow().path().is_dir());
        owners.begin_cleanup();
        drop(owners.release_after_join());
        assert!(!path.exists());
    }
}
