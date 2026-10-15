//! Offline authority rollover. Receives public signed proof material only.
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use store_service::live_authority::{LiveAuthority, read_authority_transition};

fn run() -> Result<()> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(
        arguments.len() == 2,
        "usage: store_authority_rotate ROOT PROOF_JSON"
    );
    let root = PathBuf::from(&arguments[0]);
    let proof = read_authority_transition(&PathBuf::from(&arguments[1]))
        .context("invalid offline authority proof")?;
    let floor = LiveAuthority::rotate_offline(&root, &proof)
        .context("offline authority rollover failed; retain state and verify the cutover")?;
    // This public pin is emitted only after fsync, atomic rename, directory
    // fsync and exact readback succeeded. The operator owns external pin storage.
    println!("{}", floor.encode());
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("store_authority_rotate: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
