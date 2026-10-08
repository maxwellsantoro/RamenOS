//! Actual generator and script regressions; all generated files are private fixtures.
use std::fs::{self, File, FileTimes};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);
const VALID: &str = "namespace = \"harness.fixture\"\nversion = \"v1\"\nprotocol = 701\n[message.echo]\nmsg_type = 1\nfields = [\"value:u32\"]\n";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "ramen-codegen-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self, p: &str) -> PathBuf {
        self.0.join(p)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn generator() -> &'static str {
    env!("CARGO_BIN_EXE_idl_codegen")
}
fn invoke(f: &Fixture, lang: &str, out: &str) -> Output {
    Command::new(generator())
        .current_dir(&f.0)
        .args(["--in", "fixture.toml", "--out", out, "--lang", lang])
        .output()
        .unwrap()
}
fn success(o: Output) {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
fn old_mtime(p: &Path) -> std::time::SystemTime {
    let old = UNIX_EPOCH + Duration::from_secs(946_684_800);
    File::options()
        .write(true)
        .open(p)
        .unwrap()
        .set_times(FileTimes::new().set_modified(old))
        .unwrap();
    fs::metadata(p).unwrap().modified().unwrap()
}

#[test]
fn all_languages_render_again_without_rewriting_identical_bytes() {
    let f = Fixture::new();
    fs::write(f.path("fixture.toml"), VALID).unwrap();
    for (lang, out) in [
        ("rust", "output.rs"),
        ("c", "output.h"),
        ("wasm-imports", "imports.rs"),
        ("wasm-host", "host.rs"),
    ] {
        success(invoke(&f, lang, out));
        let original = fs::read(f.path(out)).unwrap();
        assert!(!original.is_empty());
        let stamp = old_mtime(&f.path(out));
        success(invoke(&f, lang, out));
        assert_eq!(fs::read(f.path(out)).unwrap(), original, "{lang}");
        assert_eq!(
            fs::metadata(f.path(out)).unwrap().modified().unwrap(),
            stamp,
            "identical {lang} was rewritten"
        );
    }
}

#[test]
fn changed_input_stale_and_missing_outputs_are_repaired() {
    let f = Fixture::new();
    fs::write(f.path("fixture.toml"), VALID).unwrap();
    for (lang, out) in [
        ("rust", "r.rs"),
        ("c", "c.h"),
        ("wasm-imports", "i.rs"),
        ("wasm-host", "h.rs"),
    ] {
        success(invoke(&f, lang, out));
        let first = fs::read(f.path(out)).unwrap();
        fs::write(
            f.path("fixture.toml"),
            VALID.replace("harness.fixture", "harness.changed"),
        )
        .unwrap();
        success(invoke(&f, lang, out));
        let changed = fs::read(f.path(out)).unwrap();
        assert_ne!(changed, first, "{lang}");
        fs::write(f.path(out), "stale").unwrap();
        success(invoke(&f, lang, out));
        assert_eq!(fs::read(f.path(out)).unwrap(), changed);
        fs::remove_file(f.path(out)).unwrap();
        success(invoke(&f, lang, out));
        assert_eq!(fs::read(f.path(out)).unwrap(), changed);
        fs::write(f.path("fixture.toml"), VALID).unwrap();
    }
}

#[test]
fn invalid_input_and_render_failure_do_not_touch_existing_output() {
    let f = Fixture::new();
    let out = f.path("output.rs");
    fs::write(&out, "retained output").unwrap();
    let stamp = old_mtime(&out);
    for invalid in [
        "not valid toml",
        &VALID.replace("value:u32", "value:unknown_type"),
    ] {
        fs::write(f.path("fixture.toml"), invalid).unwrap();
        assert!(!invoke(&f, "rust", "output.rs").status.success());
        assert_eq!(fs::read(&out).unwrap(), b"retained output");
        assert_eq!(fs::metadata(&out).unwrap().modified().unwrap(), stamp);
    }
    fs::write(f.path("fixture.toml"), VALID).unwrap();
    fs::create_dir(f.path("directory.rs")).unwrap();
    assert!(!invoke(&f, "rust", "directory.rs").status.success());
    let failed = Command::new(generator())
        .current_dir(&f.0)
        .env("PATH", &f.0)
        .args(["--in", "fixture.toml", "--out", "output.rs"])
        .output()
        .unwrap();
    assert!(!failed.status.success(), "missing rustfmt must not succeed");
    assert_eq!(fs::read(&out).unwrap(), b"retained output");
    assert_eq!(fs::metadata(&out).unwrap().modified().unwrap(), stamp);
}

#[cfg(unix)]
mod script {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    // Frozen existing catalog: output identity and language are independent of script parsing.
    const OUTPUTS: &[(&str, &str)] = &[
        ("kernel_api/src/generated/ping_harness.generated.rs", "rust"),
        (
            "kernel_api/src/generated/capsule_control_v0.generated.rs",
            "rust",
        ),
        ("tools/capsule/generated/capsule_control_v0.h", "c"),
        (
            "kernel_api/src/generated/echo_harness_v0.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/portal_file_picker.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/portal_clipboard.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/portal_notifications.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/portal_screen_capture.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/desktop_session_v1.generated.rs",
            "rust",
        ),
        ("kernel_api/src/generated/input_v1.generated.rs", "rust"),
        (
            "kernel_api/src/generated/desktop_focus_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/desktop_surface_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/desktop_editor_session_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/desktop_editor_process_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/desktop_artifact_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/domain_manager_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/gpu_quarantine_v1.generated.rs",
            "rust",
        ),
        ("kernel_api/src/generated/net_v1.generated.rs", "rust"),
        ("kernel_api/src/generated/block_v1.generated.rs", "rust"),
        (
            "kernel_api/src/generated/shmem_control_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/store_service_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/trace_service_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/echo_harness_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/trace_service_v2.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/semantic_state_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/semantic_store_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/agent_task_v1.generated.rs",
            "rust",
        ),
        (
            "kernel_api/src/generated/execution_fabric_v1.generated.rs",
            "rust",
        ),
        ("sdk/src/generated/harness_echo_v0.rs", "wasm-imports"),
        (
            "sdk/src/generated/services_semantic_state_v1.rs",
            "wasm-imports",
        ),
        (
            "sdk/src/generated/harness_shmem_control_v1.rs",
            "wasm-imports",
        ),
        (
            "services/native_runner/src/generated/services_semantic_state_v1_host.rs",
            "wasm-host",
        ),
        (
            "services/native_runner/src/generated/harness_shmem_control_v1_host.rs",
            "wasm-host",
        ),
        (
            "services/native_runner/src/generated/harness_echo_v1_host.rs",
            "wasm-host",
        ),
        (
            "services/native_runner/src/generated/harness_trace_v2_host.rs",
            "wasm-host",
        ),
    ];
    const AGGREGATOR: &str = "services/native_runner/src/generated/mod.rs";
    const AGGREGATOR_BYTES: &str = "//! Generated host bindings for the native WASM runner.\n//! Regenerated by tools/ci/run_codegen.sh; do not edit by hand.\n#![allow(dead_code)]\n\npub mod harness_echo_v1_host;\npub mod harness_shmem_control_v1_host;\npub mod harness_trace_v2_host;\npub mod services_semantic_state_v1_host;\n";

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                copy_tree(&e.path(), &to.join(e.file_name()));
            } else {
                fs::copy(e.path(), to.join(e.file_name())).unwrap();
            }
        }
    }
    fn executable(p: &Path, body: &str) {
        fs::write(p, body).unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fn fixture() -> Fixture {
        let f = Fixture::new();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        fs::create_dir_all(f.path("tools/ci")).unwrap();
        fs::copy(
            repo.join("tools/ci/run_codegen.sh"),
            f.path("tools/ci/run_codegen.sh"),
        )
        .unwrap();
        fs::copy(
            repo.join("tools/ci/cargo_artifact.py"),
            f.path("tools/ci/cargo_artifact.py"),
        )
        .unwrap();
        copy_tree(&repo.join("idl/harness"), &f.path("idl/harness"));
        copy_tree(&repo.join("idl/portals"), &f.path("idl/portals"));
        copy_tree(&repo.join("idl/services"), &f.path("idl/services"));
        fs::create_dir(f.path("bin")).unwrap();
        executable(
            &f.path("bin/generator"),
            "#!/usr/bin/env python3\nimport json,os,sys\nwith open('renders.jsonl','a') as f: f.write(json.dumps(sys.argv[1:])+'\\n')\nos.execv(os.environ['ACTUAL_GENERATOR'],[os.environ['ACTUAL_GENERATOR']]+sys.argv[1:])\n",
        );
        executable(
            &f.path("bin/cargo"),
            "#!/usr/bin/env python3\nimport json,os,sys\nwith open('cargo.jsonl','a') as f: f.write(json.dumps(sys.argv[1:])+'\\n')\nmode=os.environ.get('BUILD_MODE','valid')\nif sys.argv[1]=='run':\n a=sys.argv.index('--'); os.execv(os.environ['WRAPPED_GENERATOR'],[os.environ['WRAPPED_GENERATOR']]+sys.argv[a+1:])\nif sys.argv[1]!='build': sys.exit(91)\nif mode=='failure': sys.exit(31)\nr={'reason':'compiler-artifact','target':{'name':'idl_codegen','kind':['bin']},'profile':{'test':False},'executable':os.environ['WRAPPED_GENERATOR']}\nif mode!='missing': print(json.dumps(r))\nif mode=='ambiguous': print(json.dumps(r))\nprint(json.dumps({'reason':'build-finished','success':True}))\n",
        );
        f
    }
    fn run(f: &Fixture, mode: &str) -> Output {
        Command::new("bash")
            .arg("tools/ci/run_codegen.sh")
            .current_dir(&f.0)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    f.path("bin").display(),
                    std::env::var("PATH").unwrap()
                ),
            )
            .env("ACTUAL_GENERATOR", generator())
            .env("WRAPPED_GENERATOR", f.path("bin/generator"))
            .env("BUILD_MODE", mode)
            .env("TMPDIR", &f.0)
            .output()
            .unwrap()
    }
    fn lines(f: &Fixture, p: &str) -> Vec<String> {
        match fs::read_to_string(f.path(p)) {
            Ok(s) => s.lines().map(str::to_owned).collect(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => panic!("{e}"),
        }
    }
    fn render_inventory(f: &Fixture) {
        let renders = lines(f, "renders.jsonl");
        assert_eq!(renders.len(), OUTPUTS.len());
        for (row, (out, lang)) in renders.iter().zip(OUTPUTS) {
            // Each wrapper line comes from an actual generator invocation.
            assert!(row.contains(&format!("\"{out}\"")), "{row}");
            if *lang == "wasm-host" || *lang == "wasm-imports" {
                assert!(row.contains(&format!("\"{lang}\"")));
            }
            assert!(!fs::read(f.path(out)).unwrap().is_empty());
        }
    }

    #[test]
    fn script_builds_once_renders_all_and_preserves_every_identical_output() {
        let f = fixture();
        success(run(&f, "valid"));
        render_inventory(&f);
        let calls = lines(&f, "cargo.jsonl");
        assert_eq!(calls.len(), 1, "one build, not one Cargo run per output");
        assert!(
            calls[0].contains("\"build\"")
                && calls[0].contains("\"--locked\"")
                && calls[0].contains("\"--message-format=json\"")
        );
        assert_eq!(
            fs::read(f.path(AGGREGATOR)).unwrap(),
            AGGREGATOR_BYTES.as_bytes()
        );
        let originals: Vec<_> = OUTPUTS
            .iter()
            .map(|(p, _)| (*p, fs::read(f.path(p)).unwrap(), old_mtime(&f.path(p))))
            .chain(std::iter::once((
                AGGREGATOR,
                fs::read(f.path(AGGREGATOR)).unwrap(),
                old_mtime(&f.path(AGGREGATOR)),
            )))
            .collect();
        fs::remove_file(f.path("renders.jsonl")).unwrap();
        success(run(&f, "valid"));
        render_inventory(&f); // no skipping based on existing files
        assert_eq!(lines(&f, "cargo.jsonl").len(), 2);
        for (p, bytes, stamp) in originals {
            assert_eq!(fs::read(f.path(p)).unwrap(), bytes, "{p}");
            assert_eq!(
                fs::metadata(f.path(p)).unwrap().modified().unwrap(),
                stamp,
                "{p}"
            );
        }
    }

    #[test]
    fn script_repairs_changed_and_missing_aggregator_and_registered_outputs() {
        let f = fixture();
        success(run(&f, "valid"));
        let old = fs::read(f.path(OUTPUTS[0].0)).unwrap();
        fs::write(f.path(OUTPUTS[0].0), "corrupt").unwrap();
        fs::remove_file(f.path(OUTPUTS[2].0)).unwrap();
        fs::write(f.path(AGGREGATOR), "corrupt").unwrap();
        success(run(&f, "valid"));
        assert_eq!(fs::read(f.path(OUTPUTS[0].0)).unwrap(), old);
        assert!(f.path(OUTPUTS[2].0).is_file());
        assert_eq!(
            fs::read(f.path(AGGREGATOR)).unwrap(),
            AGGREGATOR_BYTES.as_bytes()
        );
        fs::remove_file(f.path(AGGREGATOR)).unwrap();
        success(run(&f, "valid"));
        assert_eq!(
            fs::read(f.path(AGGREGATOR)).unwrap(),
            AGGREGATOR_BYTES.as_bytes()
        );
    }

    #[test]
    fn script_failed_or_unselected_build_never_starts_generation() {
        for mode in ["failure", "missing", "ambiguous"] {
            let f = fixture();
            let result = run(&f, mode);
            assert!(
                !result.status.success(),
                "{mode} build unexpectedly accepted"
            );
            assert!(!String::from_utf8_lossy(&result.stdout).contains("CODEGEN: ok"));
            assert!(lines(&f, "renders.jsonl").is_empty());
            assert!(!f.path(AGGREGATOR).exists());
        }
    }

    #[test]
    fn script_generator_failure_propagates_without_success_or_aggregator() {
        let f = fixture();
        fs::write(f.path("idl/harness/ping_harness.toml"), "invalid toml").unwrap();
        let result = run(&f, "valid");
        assert!(!result.status.success());
        assert!(!String::from_utf8_lossy(&result.stdout).contains("CODEGEN: ok"));
        assert_eq!(lines(&f, "renders.jsonl").len(), 1);
        assert!(!f.path(OUTPUTS[0].0).exists());
        assert!(!f.path(AGGREGATOR).exists());
    }
}
