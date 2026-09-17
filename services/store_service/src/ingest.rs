//! Stage a single source read, binding the published CAS name to the copied bytes.
use artifact_store_schema::ContentId;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct StagedBlob {
    path: PathBuf,
    pub content_id: ContentId,
    pub size_bytes: u64,
}

impl StagedBlob {
    pub fn read(source: &Path, store_root: &Path) -> io::Result<Self> {
        // O_NONBLOCK prevents a replacement FIFO from blocking the open. Validate
        // the opened descriptor, not a prior path lookup; never follow symlinks.
        let mut input = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(source)?;
        if !input.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ingest requires a regular file",
            ));
        }
        fs::create_dir_all(store_root)?;
        static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
        let (path, mut output) = loop {
            let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = store_root.join(format!(".ingest-{}-{serial}.tmp", std::process::id()));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => break (path, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        };
        // Establish cleanup before any fallible copying, hashing or synchronization.
        let mut staged = Self {
            path,
            content_id: ContentId::parse(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            )
            .expect("valid placeholder"),
            size_bytes: 0,
        };
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 8192];
        loop {
            let len = match input.read(&mut buffer) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => result?,
            };
            if len == 0 {
                break;
            }
            output.write_all(&buffer[..len])?;
            hash.update(&buffer[..len]);
            staged.size_bytes = staged
                .size_bytes
                .checked_add(len as u64)
                .ok_or_else(|| io::Error::other("artifact size overflow"))?;
        }
        output.sync_all()?;
        staged.content_id = ContentId::parse(&format!("sha256:{}", hex::encode(hash.finalize())))
            .map_err(|error| io::Error::other(error.to_string()))?;
        Ok(staged)
    }

    pub fn publish(self, destination: &Path) -> io::Result<()> {
        fs::rename(&self.path, destination)?;
        if let Some(parent) = destination.parent() {
            File::open(parent)?.sync_all()?;
        }
        Ok(())
    }
}

impl Drop for StagedBlob {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_ingest_commits_the_hashed_snapshot_after_source_changes() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::write(&source, b"original bytes").unwrap();
        let staged = StagedBlob::read(&source, root.path()).unwrap();
        let id = staged.content_id.clone();
        fs::write(&source, b"changed bytes").unwrap();
        let destination = root.path().join(format!("{}.blob", id.hash_hex()));
        staged.publish(&destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"original bytes");
        assert_eq!(
            artifact_store_core::hash_blob(&destination).unwrap(),
            id.as_str()
        );
    }

    #[test]
    fn review_ingest_rejects_fifo_and_symlink_without_blocking() {
        let root = tempfile::tempdir().unwrap();
        let fifo = root.path().join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        assert!(StagedBlob::read(&fifo, root.path()).is_err());
        let source = root.path().join("source");
        fs::write(&source, b"bytes").unwrap();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&source, &link).unwrap();
        assert!(StagedBlob::read(&link, root.path()).is_err());
    }

    #[test]
    fn review_ingest_staging_is_private_unique_and_cleaned_on_abort() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::write(&source, b"bytes").unwrap();
        let first = StagedBlob::read(&source, root.path()).unwrap();
        let second = StagedBlob::read(&source, root.path()).unwrap();
        assert_ne!(first.path, second.path);
        assert_eq!(
            fs::metadata(&first.path).unwrap().permissions().mode() & 0o077,
            0
        );
        let path = first.path.clone();
        drop(first);
        assert!(!path.exists());
        let second_path = second.path.clone();
        assert!(second.publish(&root.path().join("absent/blob")).is_err());
        assert!(
            !second_path.exists(),
            "failed publication cleans the staging file"
        );
    }
}
