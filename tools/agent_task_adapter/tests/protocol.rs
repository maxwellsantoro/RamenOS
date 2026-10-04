use agent_task_adapter::protocol::*;
use serde_json::{Value, json};
#[test]
fn encoding_rejects_ambiguous_authority_and_integer_forms() {
    let valid = json!({"schema_version":1,"request_id":"18446744073709551615","call":{"operation":"read_input","task_cap":"cap:ffffffffffffffff","resource":"resource:0000000000000001"}});
    assert!(decode_request(&serde_json::to_vec(&valid).unwrap()).is_ok());
    for (pointer, value) in [
        ("/request_id", json!(1)),
        ("/request_id", json!("01")),
        ("/request_id", json!("0")),
        ("/request_id", json!("18446744073709551616")),
        ("/request_id", json!("-1")),
        ("/call/task_cap", json!("cap:0000000000000000")),
        ("/call/task_cap", json!("cap:FFFFFFFFFFFFFFFF")),
        ("/schema_version", json!(3)),
        ("/call/operation", json!("subscribe_task")),
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            decode_request(&serde_json::to_vec(&bad).unwrap()).is_err(),
            "{bad}"
        );
    }
    let mut bad = valid.clone();
    bad["domain_id"] = json!(7);
    assert!(decode_request(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = valid.clone();
    bad["call"]["path"] = json!("/etc/passwd");
    assert!(decode_request(&serde_json::to_vec(&bad).unwrap()).is_err());
    assert!(decode_request(br#"{"schema_version":1,"request_id":"1","request_id":"2","call":{"operation":"get_task_state","task_cap":"cap:0000000000000001"}}"#).is_err());
}
#[test]
fn bounds_and_canonical_bytes_precede_dispatch() {
    let stage = |bytes: Value| json!({"schema_version":1,"request_id":"1","call":{"operation":"stage_candidate","task_cap":"cap:0000000000000001","bytes_base64":bytes}});
    for invalid in ["", "Zg", "Zh==", "!!!!"] {
        assert!(decode_request(&serde_json::to_vec(&stage(json!(invalid))).unwrap()).is_err());
    }
    assert!(
        decode_request(
            &serde_json::to_vec(&stage(json!(Bytes::new(vec![0; 65536]).unwrap()))).unwrap()
        )
        .is_ok()
    );
    assert!(Bytes::new(vec![0; 65537]).is_err());
    assert!(decode_request(&vec![b' '; MAX_REQUEST_BYTES + 1]).is_err());
    let bytes=br#"{"schema_version":1,"request_id":"1","call":{"operation":"request_grant","policy_cap":"cap:0000000000000001","task_id":"17","resource":"resource:0000000000000001","rights":[],"lifetime_ms":1}}"#;
    assert!(decode_request(bytes).is_err());
    let bytes = String::from_utf8(bytes.to_vec())
        .unwrap()
        .replace("[]", "[\"read\",\"read\"]");
    assert!(decode_request(bytes.as_bytes()).is_err());
}
#[test]
fn descriptions_and_serializer_are_backend_independent() {
    let first = tool_contract();
    let second = tool_contract();
    assert_eq!(first, second);
    let v: Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(v["tools"].as_array().unwrap().len(), 8);
    let denied = Response::error(Some(Decimal(1)), Status::Denied);
    let encoded = encode_response(&denied).unwrap();
    assert_eq!(
        String::from_utf8(encoded).unwrap(),
        r#"{"schema_version":1,"request_id":"1","status":"denied","result":null}"#
    );
    let s = Status::from_native(999);
    assert_eq!(s, None);
}

#[test]
fn subscriptions_require_v2_and_unique_bounded_event_types() {
    let valid = json!({"schema_version":2,"request_id":"1","call":{"operation":"subscribe_task","task_cap":"cap:0000000000000001","event_types":["output_changed","validation_changed"]}});
    assert!(decode_request(&serde_json::to_vec(&valid).unwrap()).is_ok());
    for events in [
        json!([]),
        json!(["output_changed", "output_changed"]),
        json!(["unknown"]),
        json!(["output_changed", "validation_changed", "output_changed"]),
    ] {
        let mut bad = valid.clone();
        bad["call"]["event_types"] = events;
        assert!(decode_request(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut old = valid;
    old["schema_version"] = json!(1);
    assert!(decode_request(&serde_json::to_vec(&old).unwrap()).is_err());
    let description: Value = serde_json::from_slice(&tool_contract_v2()).unwrap();
    assert_eq!(description["tools"].as_array().unwrap().len(), 11);
    let bad = Response {
        schema_version: 2,
        request_id: Some(Decimal(1)),
        status: Status::Ok,
        result: Some(Reply::PollTask {
            event_types: vec![EventType::OutputChanged],
            state: None,
        }),
    };
    assert!(encode_response(&bad).is_err());
}
