#!/usr/bin/env bash
# SW0 A1.1 host service proof. No model, target-kernel or hardware claim.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
cargo build -p native_runner --features agent_task_v1_dev --bin task_validator_worker
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
export RAMEN_TASK_VALIDATOR_WORKER="$TARGET_DIR/debug/task_validator_worker"
export RAMEN_TASK_EVIDENCE_DIR="${RAMEN_TASK_EVIDENCE_DIR:-$ROOT_DIR/out/agent-task-proof-rt}"
mkdir -p "$RAMEN_TASK_EVIDENCE_DIR"
cargo test -p kernel_api --test agent_task_protocol
cargo test -p artifact_store_schema --test agent_task_contract
cargo test -p store_service --features agent_task_v1_dev --lib agent_task
cargo test -p store_service --features agent_task_v1_dev --test agent_task_service
python3 - "$RAMEN_TASK_EVIDENCE_DIR" <<'PY'
import hashlib,json,pathlib,platform,subprocess,sys
root=pathlib.Path(sys.argv[1])
manifest={"schema_version":1,"environment":"host","claim":"scripted-task-and-named-service-boundaries","source_revision":subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(),"dirty_diff_sha256":hashlib.sha256(subprocess.check_output(["git","diff","HEAD"])).hexdigest(),"host":platform.platform(),"physical_hardware":False,"task_kernel_enforcement":False,"model_comparison":False}
files=sorted(set(subprocess.check_output(["git","ls-files","-co","--exclude-standard","-z"]).decode().split("\0"))-{ "" })
source={p:hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest() for p in files if pathlib.Path(p).is_file()}
manifest["cargo_lock_sha256"]=hashlib.sha256(pathlib.Path("Cargo.lock").read_bytes()).hexdigest()
manifest["rustc_version"]=subprocess.check_output(["rustc","--version"],text=True).strip()
manifest["development_features"]=["store_service/agent_task_v1_dev","native_runner/agent_task_v1_dev"]
manifest["source_files_sha256"]=hashlib.sha256(json.dumps(source,sort_keys=True).encode()).hexdigest()
(root/"source-files.json").write_text(json.dumps(source,indent=2)+"\n")
manifest["artifacts"]={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in root.glob("*.json") if p.name!="manifest.json"}
(root/"manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")
PY
echo "FOUNDRY_AGENT_TASK_PROOF_RT: PASS environment=host claim=scripted-task-and-named-service-boundaries"
