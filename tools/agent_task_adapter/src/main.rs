//! Opt-in trusted launcher. Model bytes arrive only on bounded stdin JSON lines.
use agent_task_adapter::{
    protocol::tool_contract,
    rt::{RtAdapter, fixture_from_directory},
    stdio::serve,
};
use std::{
    io::{self, Write},
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
    serve(&rt.bootstrap(), |line| rt.execute_json(line))?;
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
