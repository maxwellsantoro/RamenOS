//! Trusted development-fixture exporter; never exposed as an agent tool.
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("output directory required")?,
    );
    fs::create_dir_all(&root)?;
    for (name, bytes) in [
        (
            "config.json",
            include_bytes!("../../../tools/agent_task/fixtures/config.json").as_slice(),
        ),
        (
            "schema.json",
            include_bytes!("../../../tools/agent_task/fixtures/schema.json").as_slice(),
        ),
        (
            "policy.json",
            include_bytes!("../../../tools/agent_task/fixtures/policy.json").as_slice(),
        ),
        (
            "notes.txt",
            include_bytes!("../../../tools/agent_task/fixtures/notes.txt").as_slice(),
        ),
    ] {
        fs::write(root.join(name), bytes)?;
    }
    let wasm = wat::parse_str(include_str!(
        "../../../tools/agent_task/fixtures/equality_validator.wat"
    ))?;
    fs::write(root.join("validator.wasm"), wasm)?;
    Ok(())
}
