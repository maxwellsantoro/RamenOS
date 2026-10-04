//! A process watchdog keeps a broken deadline implementation from hanging CI.
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn review_execution_deadline_bounds_stalled_host_ipc() {
    use std::io::Read;
    use std::os::unix::net::UnixListener;
    for transport in [
        native_runner::KernelIpcTransport::default(),
        native_runner::KernelIpcTransport::ChardevSerial,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("ipc");
        let listener = UnixListener::bind(&socket).unwrap();
        let peer = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut frame = [0u8; 88];
            if transport == native_runner::KernelIpcTransport::ChardevSerial {
                stream.read_exact(&mut [0; 4]).unwrap();
            }
            stream.read_exact(&mut frame).unwrap();
            std::thread::sleep(Duration::from_millis(400));
        });
        let runner = native_runner::NativeRunner::new(native_runner::RunnerConfig {
            kernel_ipc: socket,
            kernel_ipc_transport: transport,
            trace_output: None,
            timeout_ms: 60,
        })
        .unwrap();
        let wasm = wat::parse_str(r#"(module (import "ramen::harness.echo" "echo_request::call" (func $call (param i64 i64 i32 i32 i32 i32) (result i32))) (memory (export "memory") 1) (data (i32.const 0) "\40\00\00\00") (func (export "_start") (result i32) i64.const 1 i64.const 1 i32.const 0 i32.const 0 i32.const 16 i32.const 0 call $call))"#).unwrap();
        let module = runner.load(&wasm).unwrap();
        let started = Instant::now();
        let result = runner.run(module, native_runner::RunConfig::default());
        let elapsed = started.elapsed();
        peer.join().unwrap();
        assert!(
            elapsed < Duration::from_millis(250),
            "host IPC exceeded invocation deadline: {elapsed:?}"
        );
        assert!(
            matches!(
                result,
                Err(native_runner::RunnerError::ExecutionTimeout { timeout_ms: 60 })
            ),
            "{result:?}"
        );
    }
}

#[test]
fn review_execution_deadline_interrupts_guest_and_start_section() {
    for start_section in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let wasm = dir.path().join("loop.wasm");
        let body = if start_section {
            "(func $init (loop $forever (br $forever))) (start $init) (func (export \"_start\") (result i32) i32.const 0)"
        } else {
            "(func (export \"_start\") (result i32) (loop $forever (br $forever)) i32.const 0)"
        };
        std::fs::write(&wasm, wat::parse_str(format!("(module {body})")).unwrap()).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_native_runner"))
            .args(["--kernel-ipc", "/dev/null", "--timeout-ms", "30", "--wasm"])
            .arg(&wasm)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let watchdog = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= watchdog {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("execution deadline did not interrupt guest");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("execution timed out after 30 ms"), "{error}");
    }
}

#[test]
fn review_execution_deadline_rejects_zero_budget() {
    assert!(matches!(
        native_runner::NativeRunner::new(native_runner::RunnerConfig {
            kernel_ipc: "/dev/null".into(),
            kernel_ipc_transport: native_runner::KernelIpcTransport::default(),
            trace_output: None,
            timeout_ms: 0,
        }),
        Err(native_runner::RunnerError::InvalidArgument(_))
    ));
}

#[test]
fn review_execution_deadlines_are_invocation_local() {
    const CHILD: &str = "RAMEN_REVIEW_CONCURRENT_DEADLINES";
    if std::env::var_os(CHILD).is_some() {
        let runner = std::sync::Arc::new(
            native_runner::NativeRunner::new(native_runner::RunnerConfig {
                kernel_ipc: "/dev/null".into(),
                kernel_ipc_transport: native_runner::KernelIpcTransport::default(),
                trace_output: None,
                timeout_ms: 120,
            })
            .unwrap(),
        );
        let wasm = wat::parse_str("(module (func (export \"_start\") (result i32) (loop $forever (br $forever)) i32.const 0))").unwrap();
        let first = runner.load(&wasm).unwrap();
        let second = runner.load(&wasm).unwrap();
        let worker_runner = runner.clone();
        let (ready, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let start = Instant::now();
            ready.send(()).unwrap();
            let result = worker_runner.run(first, native_runner::RunConfig::default());
            assert!(matches!(
                result,
                Err(native_runner::RunnerError::ExecutionTimeout { timeout_ms: 120 })
            ));
            assert!(start.elapsed() >= Duration::from_millis(100));
        });
        receiver.recv().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let start = Instant::now();
        let result = runner.run(second, native_runner::RunConfig::default());
        assert!(matches!(
            result,
            Err(native_runner::RunnerError::ExecutionTimeout { timeout_ms: 120 })
        ));
        assert!(
            start.elapsed() >= Duration::from_millis(100),
            "another invocation's epoch interrupted this run early"
        );
        worker.join().unwrap();
        // A completed invocation cancels its timer without waiting out its budget.
        let normal =
            wat::parse_str("(module (func (export \"_start\") (result i32) i32.const 42))")
                .unwrap();
        assert_eq!(
            runner
                .load_and_run(&normal, native_runner::RunConfig::default())
                .unwrap()
                .exit_code,
            42
        );
        return;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "review_execution_deadlines_are_invocation_local",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let watchdog = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= watchdog {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("concurrent invocation deadlines did not terminate");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
