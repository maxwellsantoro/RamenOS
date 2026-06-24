use native_runner::{NativeRunner, RunConfig};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn runner_loads_hello_wasm() {
    let wasm_path = hello_wasm_path();
    if !wasm_path.exists() {
        eprintln!("Skipping: hello_wasm.wasm not found at {:?}", wasm_path);
        eprintln!("Build with: cargo build -p hello_wasm --target wasm32-unknown-unknown");
        return;
    }
    let wasm_bytes = std::fs::read(&wasm_path).expect("Failed to read WASM");
    let runner = NativeRunner::for_testing();
    let module = runner.load(&wasm_bytes).expect("Failed to load WASM");
    assert!(format!("{:?}", module).contains("Module"));
}

#[test]
fn runner_injects_capabilities() {
    let wasm_bytes = create_wasm_with_mutable_globals();
    let runner = NativeRunner::for_testing();
    let config = RunConfig {
        granted_handles: HashMap::from([(
            "RAMEN_CAP_ECHO_REQUEST".to_string(),
            0x1234_5678_1234_5678,
        )]),
    };
    let result = runner.load_and_run(&wasm_bytes, config);
    assert!(result.is_ok(), "Failed to run: {:?}", result.err());
    assert_eq!(result.unwrap().exit_code, 0);
}

#[test]
fn runner_fails_without_required_capability() {
    let wasm_bytes = create_wasm_with_mutable_globals();
    let runner = NativeRunner::for_testing();
    let config = RunConfig {
        granted_handles: HashMap::new(),
    };
    let result = runner.load_and_run(&wasm_bytes, config);
    assert!(result.is_err(), "Should fail without required capability");
    match result.err().unwrap() {
        native_runner::RunnerError::MissingCapability(_) => (),
        _ => panic!("Expected MissingCapability error"),
    }
}

fn hello_wasm_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/wasm32-unknown-unknown/debug/hello_wasm.wasm");
    path
}

/// Create a WASM module with mutable capability globals like the tests expect.
/// This is needed because hello_wasm uses static variables, not exported globals.
fn create_wasm_with_mutable_globals() -> Vec<u8> {
    wat::parse_str(
        r#"(module
            (global $RAMEN_CAP_ECHO_REQUEST (export "RAMEN_CAP_ECHO_REQUEST") (mut i64) (i64.const 0))
            (func (export "_start") (result i32)
                i32.const 0
            )
        )"#,
    )
    .expect("Failed to parse WASM")
}
