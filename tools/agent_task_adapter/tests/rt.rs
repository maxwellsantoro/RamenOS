#![cfg(feature = "agent_task_v1_dev")]
use agent_task_adapter::{protocol::*, rt::*};
use serde_json::{Value, json};
fn call(rt: &mut RtAdapter, id: u64, call: Value) -> Value {
    serde_json::from_slice(
        &rt.execute_json(
            &serde_json::to_vec(
                &json!({"schema_version":1,"request_id":id.to_string(),"call":call}),
            )
            .unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}
fn open(path: &std::path::Path, domain: u64) -> RtAdapter {
    RtAdapter::launch(
        path,
        development_fixture(),
        std::env::var_os("RAMEN_TASK_VALIDATOR_WORKER")
            .unwrap()
            .into(),
        domain,
    )
    .unwrap()
}
fn grant(rt: &mut RtAdapter, rights: Value) -> String {
    let b = rt.bootstrap();
    let r = call(
        rt,
        1,
        json!({"operation":"request_grant","policy_cap":b.policy_cap,"task_id":b.task_id,"resource":b.resources[0].resource,"rights":rights,"lifetime_ms":60000}),
    );
    assert_eq!(r["status"], "ok");
    r["result"]["task_cap"].as_str().unwrap().into()
}
#[test]
fn useful_json_task_retry_restart_and_mapping_cleanup() {
    let d = tempfile::tempdir().unwrap();
    let mut rt = open(d.path(), 7);
    let cap = grant(
        &mut rt,
        json!(["read", "stage", "validate", "commit", "observe"]),
    );
    let state = call(
        &mut rt,
        2,
        json!({"operation":"get_task_state","task_cap":cap}),
    );
    let original = state["result"]["state"]["content_id"].clone();
    let read = call(
        &mut rt,
        3,
        json!({"operation":"read_input","task_cap":cap,"resource":"resource:0000000000000001"}),
    );
    let bytes: Bytes = serde_json::from_value(read["result"]["bytes_base64"].clone()).unwrap();
    let mut config: Value = serde_json::from_slice(bytes.as_slice()).unwrap();
    let schema = call(
        &mut rt,
        4,
        json!({"operation":"read_input","task_cap":cap,"resource":"resource:0000000000000064"}),
    );
    let schema: Bytes = serde_json::from_value(schema["result"]["bytes_base64"].clone()).unwrap();
    config["enabled"] =
        serde_json::from_slice::<Value>(schema.as_slice()).unwrap()["enabled"].clone();
    let staged = call(
        &mut rt,
        5,
        json!({"operation":"stage_candidate","task_cap":cap,"bytes_base64":Bytes::new(serde_json::to_vec(&config).unwrap()).unwrap()}),
    );
    let candidate = staged["result"]["candidate_cap"].clone();
    let wrong_pin = call(
        &mut rt,
        40,
        json!({"operation":"validate_candidate","task_cap":cap,
        "candidate_cap":candidate,"validator_id":format!("sha256:{}","0".repeat(64))}),
    );
    assert_eq!(wrong_pin["status"], "denied");
    assert!(wrong_pin["result"].is_null());
    let validator = state["result"]["state"]["validator_id"].clone();
    let validated = call(
        &mut rt,
        6,
        json!({"operation":"validate_candidate","task_cap":cap,"candidate_cap":candidate,"validator_id":validator}),
    );
    assert_eq!(validated["result"]["outcome"], "valid");
    let commit = json!({"operation":"commit_candidate","task_cap":cap,"candidate_cap":candidate,"expected_revision":"0","expected_content_id":original});
    let receipt = call(&mut rt, 7, commit.clone());
    assert_eq!(receipt["status"], "ok");
    assert_eq!(call(&mut rt, 7, commit.clone()), receipt);
    let mut reused = commit.clone();
    reused["expected_revision"] = json!("1");
    assert_eq!(call(&mut rt, 7, reused)["status"], "request_reuse");
    assert_eq!(rt.mapping_count(), 0);
    let journal = rt.evidence().unwrap();
    let fixture = development_fixture();
    store_service::agent_task::verify_evidence(&journal, &fixture.contract, &fixture.input)
        .unwrap();
    if let Some(path) = std::env::var_os("RAMEN_TASK_ADAPTER_EVIDENCE_DIR") {
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(std::path::Path::new(&path).join("journal.json"), &journal).unwrap();
        std::fs::write(std::path::Path::new(&path).join("report.json"),serde_json::to_vec_pretty(&json!({"schema_version":1,"environment":"host","claim":"scripted-json-to-native-task","accepted":receipt["result"]["content_id"],"model_comparison":false,"lt_equivalence":false,"target_kernel_enforcement":false,"receipt_replay":true})).unwrap()).unwrap();
    }
    drop(rt);
    let mut rt = open(d.path(), 7);
    let renewed = grant(&mut rt, json!(["commit"]));
    let mut retry = commit;
    retry["task_cap"] = json!(renewed);
    assert_eq!(call(&mut rt, 7, retry), receipt);
}
#[test]
fn denials_are_backend_enforced_and_redacted() {
    let d = tempfile::tempdir().unwrap();
    let mut rt = open(d.path(), 7);
    let cap = grant(&mut rt, json!(["read", "observe"]));
    for call_value in [
        json!({"operation":"read_input","task_cap":cap,"resource":"resource:00000000000003e7"}),
        json!({"operation":"stage_candidate","task_cap":cap,"bytes_base64":"e30="}),
        json!({"operation":"get_receipt","task_cap":cap,"commit_request_id":"1"}),
        json!({"operation":"read_input","task_cap":rt.bootstrap().policy_cap,"resource":"resource:0000000000000001"}),
    ] {
        let r = call(&mut rt, 8, call_value);
        assert_eq!(r["status"], "denied");
        assert!(r["result"].is_null());
    }
    assert_eq!(rt.mapping_count(), 0);
    let before = rt.evidence().unwrap();
    let bad=rt.execute_json(br#"{"schema_version":1,"request_id":"9","call":{"operation":"read_input","task_cap":"cap:0000000000000001","resource":"resource:0000000000000001","domain_id":7}}"#).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&bad).unwrap()["status"],
        "invalid"
    );
    assert_eq!(before, rt.evidence().unwrap());
    drop(rt);
    let mut foreign = open(d.path(), 8);
    let b = foreign.bootstrap();
    let r = call(
        &mut foreign,
        10,
        json!({"operation":"request_grant","policy_cap":b.policy_cap,"task_id":b.task_id,"resource":b.resources[0].resource,"rights":["read"],"lifetime_ms":1000}),
    );
    assert_eq!(r["status"], "denied");
    assert!(r["result"].is_null());
}
#[test]
fn revocation_reaches_native_service_and_invalid_candidates_never_commit() {
    let d = tempfile::tempdir().unwrap();
    let mut rt = open(d.path(), 7);
    let cap = grant(
        &mut rt,
        json!(["read", "stage", "validate", "commit", "observe"]),
    );
    let state = call(
        &mut rt,
        2,
        json!({"operation":"get_task_state","task_cap":cap}),
    );
    let staged = call(
        &mut rt,
        3,
        json!({"operation":"stage_candidate","task_cap":cap,"bytes_base64":"e30="}),
    );
    let candidate = staged["result"]["candidate_cap"].clone();
    let unvalidated = call(
        &mut rt,
        21,
        json!({"operation":"commit_candidate","task_cap":cap,
        "candidate_cap":candidate,"expected_revision":"0","expected_content_id":state["result"]["state"]["content_id"]}),
    );
    assert_eq!(unvalidated["status"], "validation_failed");
    assert!(unvalidated["result"].is_null());
    let invalid = call(
        &mut rt,
        4,
        json!({"operation":"validate_candidate","task_cap":cap,"candidate_cap":candidate,"validator_id":state["result"]["state"]["validator_id"]}),
    );
    assert_eq!(invalid["result"]["outcome"], "invalid");
    assert_eq!(
        call(
            &mut rt,
            5,
            json!({"operation":"commit_candidate","task_cap":cap,"candidate_cap":candidate,"expected_revision":"0","expected_content_id":state["result"]["state"]["content_id"]})
        )["status"],
        "validation_failed"
    );
    let policy = rt.bootstrap().policy_cap;
    assert_eq!(
        call(
            &mut rt,
            6,
            json!({"operation":"revoke_grant","policy_cap":policy,"task_cap":cap})
        )["status"],
        "ok"
    );
    let denied = call(
        &mut rt,
        7,
        json!({"operation":"get_task_state","task_cap":cap}),
    );
    assert_eq!(denied["status"], "denied");
    assert!(denied["result"].is_null());
    assert_eq!(rt.mapping_count(), 0);
}
