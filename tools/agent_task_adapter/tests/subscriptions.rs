#![cfg(feature = "agent_task_v1_dev")]
use agent_task_adapter::rt::*;
use serde_json::{Value, json};
fn call(rt: &mut RtAdapter, n: u64, c: Value) -> Value {
    serde_json::from_slice(
        &rt.execute_json(
            &serde_json::to_vec(&json!({"schema_version":2,"request_id":n.to_string(),"call":c}))
                .unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn bounded_subscription_cleanup_and_redaction() {
    let dir = tempfile::tempdir().unwrap();
    let mut rt = RtAdapter::launch(
        dir.path(),
        development_fixture(),
        std::env::var_os("RAMEN_TASK_VALIDATOR_WORKER")
            .unwrap()
            .into(),
        7,
    )
    .unwrap();
    let b = rt.bootstrap();
    let grant = call(
        &mut rt,
        1,
        json!({"operation":"request_grant","policy_cap":b.policy_cap,"task_id":b.task_id,"resource":b.resources[0].resource,"rights":["observe"],"lifetime_ms":60000}),
    );
    assert_eq!(grant["status"], "ok");
    let cap = grant["result"]["task_cap"].clone();
    let mut subs = Vec::new();
    for n in 0..16 {
        let r = call(
            &mut rt,
            2 + n,
            json!({"operation":"subscribe_task","task_cap":cap,"event_types":["output_changed","validation_changed"]}),
        );
        assert_eq!(r["status"], "ok");
        subs.push(r["result"]["subscription_cap"].clone());
    }
    assert_eq!(
        call(
            &mut rt,
            20,
            json!({"operation":"subscribe_task","task_cap":cap,"event_types":["output_changed"]})
        )["status"],
        "capacity"
    );
    let r = call(
        &mut rt,
        21,
        json!({"operation":"poll_task","task_cap":cap,"subscription_cap":subs[0]}),
    );
    assert_eq!(r["result"]["event_types"], json!([]));
    assert!(r["result"]["state"].is_null());
    assert_eq!(
        call(
            &mut rt,
            22,
            json!({"operation":"unsubscribe_task","task_cap":cap,"subscription_cap":subs[0]})
        )["status"],
        "ok"
    );
    assert_eq!(
        call(
            &mut rt,
            23,
            json!({"operation":"poll_task","task_cap":cap,"subscription_cap":subs[0]})
        )["status"],
        "not_found"
    );
    let replacement = call(
        &mut rt,
        24,
        json!({"operation":"subscribe_task","task_cap":cap,"event_types":["output_changed"]}),
    );
    assert_eq!(replacement["status"], "ok");
    assert_eq!(
        call(
            &mut rt,
            25,
            json!({"operation":"revoke_grant","policy_cap":b.policy_cap,"task_cap":cap})
        )["status"],
        "ok"
    );
    let denied = call(
        &mut rt,
        26,
        json!({"operation":"poll_task","task_cap":cap,"subscription_cap":subs[1]}),
    );
    assert_eq!(denied["status"], "denied");
    assert!(denied["result"].is_null());
    assert_eq!(rt.mapping_count(), 0);
    drop(rt);
    let mut rt = RtAdapter::launch(
        dir.path(),
        development_fixture(),
        std::env::var_os("RAMEN_TASK_VALIDATOR_WORKER")
            .unwrap()
            .into(),
        7,
    )
    .unwrap();
    let b = rt.bootstrap();
    let g = call(
        &mut rt,
        27,
        json!({"operation":"request_grant","policy_cap":b.policy_cap,"task_id":b.task_id,"resource":b.resources[0].resource,"rights":["observe"],"lifetime_ms":60000}),
    );
    assert_eq!(
        call(
            &mut rt,
            28,
            json!({"operation":"poll_task","task_cap":g["result"]["task_cap"],"subscription_cap":subs[1]})
        )["status"],
        "not_found"
    );
}
