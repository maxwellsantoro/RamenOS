//! Preserve the ordinary service launch when offline operator binaries exist.
use std::{path::Path, process::Command};

#[test]
fn ordinary_cargo_run_selects_store_service_with_offline_rotation_binary_present() {
    // Metadata resolves Cargo's package and discovered targets without building
    // or launching either executable, and requires no registry access.
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args([
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let package = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["name"] == "store_service")
        .unwrap();
    assert_eq!(package["default_run"], "store_service");
    for expected in ["store_service", "store_authority_rotate"] {
        assert!(
            package["targets"].as_array().unwrap().iter().any(|target| {
                target["name"] == expected
                    && target["kind"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|kind| kind == "bin")
            }),
            "Cargo did not discover required binary {expected}"
        );
    }
}
