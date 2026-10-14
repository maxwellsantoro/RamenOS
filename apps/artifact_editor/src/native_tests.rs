//! Private consumer orchestration assertion: genuine intent expires before entry.
#[allow(dead_code)]
#[path = "../tests/native_task_support/mod.rs"]
mod support;
use super::*;
use support::*;

#[test]
fn reliability_expired_intent_does_not_poison_app_chain() {
    let name = "reliability_expired_intent_does_not_poison_app_chain";
    let recording = ok(recording::CaseRecorder::open(name));
    let mut spec = scenario(
        "ui1_1_clock_capacity_and_counter_limits_fail_closed",
        "repair",
        0,
        recording::ScenarioScope::DirectNative,
    );
    spec.case = name;
    spec.requirements = &["supplemental_desktop_reliability"];
    let mut f = Fixture::recorded(&recording, spec, 1);
    let mut app = f.app(0, 101, 1);
    for (usage, phase) in [(224, 1), (22, 1), (22, 2), (224, 2)] {
        f.drivers[0].send(&f.h, usage, phase, 2);
        ok(app.desktop.step_native_editor(&app.editor, 2));
    }
    let intent = ok(app.desktop.take_save_intent(&app.editor));
    std::thread::sleep(std::time::Duration::from_millis(1050));
    assert_eq!(
        app.start_intent(intent, 2),
        Err(NativeEditorError::Desktop(Status::Timeout))
    );
    assert!(
        app.pending.is_empty(),
        "pre-entry timeout owns no reconciliation chain"
    );
    assert_eq!(
        ok(app.observe(2)).phase,
        desktop::NativeEditorPhase::Unsaved
    );
    assert_eq!(f.counts(101), (0, 0, 0));
    f.save(0, &mut app, 2);
    assert_eq!(ok(app.observe(2)).bytes, OLD);
    assert_eq!(f.counts(101), (1, 1, 1));
    assert!(app.pending.is_empty());
    drop(app);
    f.finish();
}
