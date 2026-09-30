//! Opt-in Linux typed launcher. The private Python broker owns LT enforcement.
use agent_task_adapter::{protocol::*, stdio::serve};
use std::os::unix::process::CommandExt;
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
};
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "LT backend protocol failure")
}
struct Broker(Child);
impl Drop for Broker {
    fn drop(&mut self) {
        // Normal EOF permits broker cleanup. No agent-created child or shell is
        // launched here; validator containment is owned by the Linux launcher.
        self.0.stdin.take();
        let _ = self.0.wait();
    }
}
fn frame(input: &mut impl BufRead) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    input
        .take((MAX_RESPONSE_BYTES + 2) as u64)
        .read_until(b'\n', &mut bytes)?;
    if !bytes.ends_with(b"\n") || bytes.len() > MAX_RESPONSE_BYTES + 1 {
        return Err(invalid());
    }
    bytes.pop();
    Ok(bytes)
}
fn run() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--describe" || args[0] == "--describe-v2") {
        io::stdout().write_all(&if args[0] == "--describe" {
            tool_contract()
        } else {
            tool_contract_v2()
        })?;
        io::stdout().write_all(b"\n")?;
        return Ok(());
    }
    if !cfg!(target_os = "linux")
        || args.len() != 6
        || args[0] != "--fixture"
        || args[2] != "--store"
        || args[4] != "--worker"
    {
        return Err(invalid());
    }
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../agent_task/lt_backend.py");
    let mut command = Command::new("python3");
    command.arg("-I");
    // -I removes environment and cwd imports; explicitly add the trusted source
    // directory so the broker can import its fixed containment helper.
    command.arg("-c").arg("import runpy,sys; from pathlib import Path; p=Path(sys.argv.pop(1)); sys.path.insert(0,str(p.parent)); runpy.run_path(str(p),run_name='__main__')").arg(script).args(&args)
        .env_clear().env("PATH",std::env::var_os("PATH").unwrap_or_else(||"/usr/bin:/bin".into()))
        .env("HOME",std::env::var_os("HOME").unwrap_or_else(||"/tmp".into()))
        .process_group(0).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    if let Some(image) = std::env::var_os("RAMEN_TASK_LINUX_IMAGE") {
        command.env("RAMEN_TASK_LINUX_IMAGE", image);
    }
    let mut broker = Broker(command.spawn()?);
    let mut output = BufReader::new(broker.0.stdout.take().ok_or_else(invalid)?);
    let bootstrap: Bootstrap =
        serde_json::from_slice(&frame(&mut output)?).map_err(|_| invalid())?;
    if bootstrap.schema_version != 1 {
        return Err(invalid());
    }
    serve(&bootstrap, |line| {
        let request = match decode_request(line) {
            Ok(r) => r,
            Err(_) => {
                return encode_response(&Response::error(None, Status::Invalid))
                    .map_err(|_| invalid());
            }
        };
        let payload = serde_json::to_vec(&request)?;
        let input = broker.0.stdin.as_mut().ok_or_else(invalid)?;
        input.write_all(&payload)?;
        input.write_all(b"\n")?;
        input.flush()?;
        let response: Response =
            serde_json::from_slice(&frame(&mut output)?).map_err(|_| invalid())?;
        if response.schema_version != request.schema_version
            || response.request_id.map(|r| r.0) != Some(request.request_id.0)
        {
            return Err(invalid());
        }
        if let Some(result) = &response.result {
            let call = serde_json::to_value(&request.call)?;
            let reply = serde_json::to_value(result)?;
            if call["operation"] != reply["operation"] {
                return Err(invalid());
            }
        }
        encode_response(&response).map_err(|_| invalid())
    })?;
    broker.0.stdin.take();
    if !broker.0.wait()?.success() {
        return Err(invalid());
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
