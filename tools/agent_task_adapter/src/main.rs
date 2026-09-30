//! Opt-in trusted launcher. Model bytes arrive only on bounded stdin JSON lines.
use agent_task_adapter::{
    protocol::{MAX_REQUEST_BYTES, tool_contract},
    rt::{RtAdapter, fixture_from_directory},
};
use std::{
    io::{self, BufRead, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    path::PathBuf,
};
// Existing host libraries print verbose diagnostics. Keep those out of the
// model transport and avoid backpressure from an undrained diagnostic pipe.
// Durable task audit stays in the service journal. This affects only this
// explicitly enabled standalone launcher, never production service logging.
struct BackendDiagnostics(std::fs::File);
impl BackendDiagnostics {
    fn quiet() -> io::Result<Self> {
        let saved = unsafe { libc::fcntl(libc::STDERR_FILENO, libc::F_DUPFD_CLOEXEC, 3) };
        if saved < 0 {
            return Err(io::Error::last_os_error());
        }
        let saved = unsafe { std::fs::File::from_raw_fd(saved) };
        let sink = std::fs::OpenOptions::new().write(true).open("/dev/null")?;
        if unsafe { libc::dup2(sink.as_raw_fd(), libc::STDERR_FILENO) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(saved))
    }
}
impl Drop for BackendDiagnostics {
    fn drop(&mut self) {
        unsafe {
            libc::dup2(self.0.as_raw_fd(), libc::STDERR_FILENO);
        }
    }
}
fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--describe" {
        io::stdout().write_all(&tool_contract())?;
        io::stdout().write_all(b"\n")?;
        return Ok(());
    }
    if args.len() != 6 || args[0] != "--fixture" || args[2] != "--store" || args[4] != "--worker" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "use --describe or --fixture DIR --store DIR --worker PATH",
        ));
    }
    let _diagnostics = BackendDiagnostics::quiet()?;
    let fixture = fixture_from_directory(&PathBuf::from(&args[1]))?;
    let domain = fixture.contract.domain_id;
    let mut rt = RtAdapter::launch(
        &PathBuf::from(&args[3]),
        fixture,
        PathBuf::from(&args[5]),
        domain,
    )?;
    let mut out = io::stdout().lock();
    out.write_all(&serde_json::to_vec(&rt.bootstrap())?)?;
    out.write_all(b"\n")?;
    out.flush()?;
    let mut input = io::stdin().lock();
    loop {
        let mut line = Vec::new();
        let count = (&mut input)
            .take((MAX_REQUEST_BYTES + 2) as u64)
            .read_until(b'\n', &mut line)?;
        if count == 0 {
            break;
        }
        if line.ends_with(b"\n") {
            line.pop();
        }
        if line.len() > MAX_REQUEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request frame limit",
            ));
        }
        out.write_all(&rt.execute_json(&line)?)?;
        out.write_all(b"\n")?;
        out.flush()?;
    }
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!(
            "task adapter stopped; no automatic retry (a lost commit reply may require explicit receipt lookup)"
        );
        std::process::exit(1);
    }
}
