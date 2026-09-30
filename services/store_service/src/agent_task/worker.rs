//! Outer deadline includes spawn, job IPC, compilation and all guest execution.
use artifact_store_schema::agent_task::{ValidatorJobV0, ValidatorResultV0};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        // A private process group contains the worker and any owned descendants.
        unsafe {
            libc::kill(-(self.0.id() as i32), libc::SIGKILL);
        }
        let _ = self.0.wait();
        #[cfg(target_os = "linux")]
        loop {
            let status = unsafe { libc::waitpid(-(self.0.id() as i32), std::ptr::null_mut(), 0) };
            if status > 0 {
                continue;
            }
            if status < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }
    }
}
fn nonblocking(fd: i32) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
pub fn supervise(path: &Path, job: &ValidatorJobV0) -> Result<(ValidatorResultV0, u64), u32> {
    use kernel_api::agent_task_protocol::*;
    let start = Instant::now();
    let deadline = start + Duration::from_millis(job.budget.wall_ms);
    #[cfg(target_os = "linux")]
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } != 0 {
        return Err(STATUS_IO);
    }
    let mut command = Command::new(path);
    command
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut worker = Worker(command.spawn().map_err(|_| STATUS_IO)?);
    let input = worker.0.stdin.take().ok_or(STATUS_IO)?;
    let mut output = worker.0.stdout.take().ok_or(STATUS_IO)?;
    nonblocking(input.as_raw_fd()).map_err(|_| STATUS_IO)?;
    nonblocking(output.as_raw_fd()).map_err(|_| STATUS_IO)?;
    let payload = serde_json::to_vec(job).map_err(|_| STATUS_INVALID)?;
    if payload.len() > 5_000_000 {
        return Err(STATUS_CAPACITY);
    }
    let mut sent = 0;
    let mut input = Some(input);
    let mut bytes = Vec::new();
    let max_output = job.budget.max_diagnostics_bytes as usize * 5 + 4096;
    loop {
        if Instant::now() >= deadline {
            return Err(STATUS_TIMEOUT);
        }
        if let Some(pipe) = input.as_mut() {
            match pipe.write(&payload[sent..]) {
                Ok(n) => sent += n,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => return Err(STATUS_IO),
            }
            if sent == payload.len() {
                input.take();
            }
        }
        let mut chunk = [0; 4096];
        match output.read(&mut chunk) {
            Ok(0) => {
                // EOF means all output writers have closed. Kill remaining group
                // members in Drop before reaping; retain no result on timeout.
                if input.is_some() {
                    return Err(STATUS_IO);
                }
                let mut result: ValidatorResultV0 =
                    serde_json::from_slice(&bytes).map_err(|_| STATUS_VALIDATION_FAILED)?;
                if result.schema_version != 1
                    || result.diagnostics.len() > job.budget.max_diagnostics_bytes as usize
                    || (result.outcome
                        == artifact_store_schema::agent_task::ValidationOutcomeV0::Valid
                        && result.guest_elapsed_ms > job.budget.guest_ms)
                {
                    return Err(STATUS_VALIDATION_FAILED);
                }
                if result.truncated
                    && result.outcome
                        == artifact_store_schema::agent_task::ValidationOutcomeV0::Valid
                {
                    result.outcome =
                        artifact_store_schema::agent_task::ValidationOutcomeV0::HostFailure;
                }
                return Ok((result, start.elapsed().as_millis() as u64));
            }
            Ok(n) => {
                if bytes.len() + n > max_output {
                    return Err(STATUS_CAPACITY);
                }
                bytes.extend_from_slice(&chunk[..n]);
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err(STATUS_IO),
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use artifact_store_schema::agent_task::ExecutionBudgetV0;
    use kernel_api::agent_task_protocol::*;
    use std::os::unix::fs::PermissionsExt;
    fn job() -> ValidatorJobV0 {
        ValidatorJobV0 {
            schema_version: 1,
            store_root: String::new(),
            validator_id: String::new(),
            candidate_id: String::new(),
            schema_id: String::new(),
            budget: ExecutionBudgetV0 {
                guest_ms: 500,
                wall_ms: 1500,
                host_call_ms: 50,
                max_diagnostics_bytes: 32,
            },
        }
    }
    fn script(body: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trusted-test-worker");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        (dir, path)
    }
    #[test]
    fn stalled_worker_input_or_compiler_is_killed_with_owned_descendants() {
        let (dir, path) = script("sleep 30 &\necho $! > \"$0.pid\"\nwait");
        let start = Instant::now();
        assert_eq!(supervise(&path, &job()).unwrap_err(), STATUS_TIMEOUT);
        assert!(start.elapsed() < Duration::from_secs(4));
        let pid: i32 = std::fs::read_to_string(dir.path().join("trusted-test-worker.pid"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        #[cfg(target_os = "linux")]
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "descendant must be reaped"
        );
        #[cfg(not(target_os = "linux"))]
        {
            let _ = pid;
        } // macOS delegates orphan reaping to launchd; Linux proves it.
    }
    #[test]
    fn oversized_and_forged_worker_results_cannot_validate() {
        let (_dir, path) = script("cat >/dev/null; printf '%100000s' x");
        assert_eq!(supervise(&path, &job()).unwrap_err(), STATUS_CAPACITY);
        let (_dir, path) = script(
            "printf '{\"schema_version\":1,\"outcome\":\"valid\",\"diagnostics\":[],\"truncated\":true,\"guest_elapsed_ms\":0}'",
        );
        let (result, _) = supervise(&path, &job()).unwrap();
        assert!(result.truncated);
        assert_eq!(
            result.outcome,
            artifact_store_schema::agent_task::ValidationOutcomeV0::HostFailure
        );
        let (_dir, path) = script(
            "printf '{\"schema_version\":1,\"outcome\":\"valid\",\"diagnostics\":[],\"truncated\":false,\"guest_elapsed_ms\":501}'",
        );
        assert_eq!(
            supervise(&path, &job()).unwrap_err(),
            STATUS_VALIDATION_FAILED
        );
    }
}
