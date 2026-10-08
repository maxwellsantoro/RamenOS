//! Proposed supplemental artifact_editor integration target; fixture extraction pending review.
//! NativeFixture is TEST-BINARY-PRIVATE and is not an existing product API.
//! See fixture-proposal.md before any registration or compilation.
mod adapter_native_fixture;
use adapter_native_fixture::NativeFixture;
use desktop_service::dev::Status;
use desktop_service::editor_dev::{NativeEditorPhase, NativeSaveCounter};

const BODY: &[u8] = b"abcd\nx\nabcdef";
fn event(f: &mut NativeFixture, usage: u32, phase: u32) {
    f.send(usage, phase, 2);
    let step =
        f.h.step_native_editor(&f.editor, 2)
            .expect("actual owner input delivery");
    assert!(step.consumed_sequence.is_some());
}
fn stroke(f: &mut NativeFixture, usage: u32) {
    event(f, usage, 1);
    event(f, usage, 2);
}
fn selected_checkpoint(f: &mut NativeFixture) {
    stroke(f, 74);
    for _ in 0..4 {
        stroke(f, 79);
    }
    event(f, 225, 1);
    stroke(f, 82);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!(s.bytes, BODY);
    assert_eq!(
        (s.cursor, s.selection, s.first_visible_line),
        (6, Some((6, 11)), 0)
    );
    assert_eq!((s.text_generation, s.view_version), (1, 7));
    assert!(matches!(s.phase, NativeEditorPhase::Loaded));
}
fn scrolled_selected_checkpoint(f: &mut NativeFixture, body: &[u8]) {
    stroke(f, 74);
    for _ in 0..4 {
        stroke(f, 79);
    }
    event(f, 225, 1);
    stroke(f, 82);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!(s.bytes, body);
    // 31 leading LF shift the selected indices; anchor42/preferred4 must survive.
    assert_eq!(
        (s.cursor, s.selection, s.first_visible_line),
        (37, Some((37, 42)), 4)
    );
    assert_eq!((s.text_generation, s.view_version), (1, 7));
    assert!(matches!(s.phase, NativeEditorPhase::Loaded));
}

#[test]
fn adapter_native_selected_snapshot_continuation_and_policies() {
    let mut f = NativeFixture::new(BODY);
    let s = f.h.editor_observation(&f.editor, 1).unwrap();
    assert_eq!(
        (
            s.cursor,
            s.first_visible_line,
            s.text_generation,
            s.view_version
        ),
        (13, 0, 1, 1)
    );
    selected_checkpoint(&mut f);
    stroke(&mut f, 81);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!((s.cursor, s.selection, s.view_version), (11, None, 8));
    stroke(&mut f, 82);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!(
        (s.cursor, s.selection, s.view_version),
        (6, Some((6, 11)), 9)
    );
    event(&mut f, 225, 2);
    stroke(&mut f, 80);
    stroke(&mut f, 42);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!(s.bytes, b"abcd\n\nabcdef");
    assert_eq!(
        (s.cursor, s.selection, s.text_generation, s.view_version),
        (5, None, 2, 11)
    );
    assert!(matches!(s.phase, NativeEditorPhase::Unsaved));
    f.finish();

    let mut f = NativeFixture::new(BODY);
    selected_checkpoint(&mut f);
    event(&mut f, 0, 3); // Native owner preserves preferred column on actual Reset.
    stroke(&mut f, 81);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!((s.cursor, s.selection), (11, None));
    f.finish();

    let mut f = NativeFixture::new(&[b'\n'; 4096]);
    let before = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!((before.first_visible_line, before.text_generation), (0, 1)); // KeepTop.
    f.send(4, 1, 2);
    let step = f.h.step_native_editor(&f.editor, 2).unwrap();
    assert!(step.consumed_sequence.is_some());
    assert!(!step.text_changed && !step.render_required);
    assert_eq!(f.h.editor_observation(&f.editor, 2).unwrap(), before);
    event(&mut f, 4, 2);
    f.finish();

    let mut body = vec![b'\n'; 31];
    body.extend_from_slice(BODY);
    let mut expected = vec![b'\n'; 31];
    expected.extend_from_slice(b"abcd\n\nabcdef");
    let mut f = NativeFixture::new(&body);
    let s = f.h.editor_observation(&f.editor, 1).unwrap();
    assert_eq!((s.cursor, s.first_visible_line), (44, 0)); // Native KeepTop before real navigation.
    scrolled_selected_checkpoint(&mut f, &body);
    stroke(&mut f, 81);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!(
        (s.cursor, s.selection, s.first_visible_line, s.view_version),
        (42, None, 4, 8)
    );
    stroke(&mut f, 82);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!(
        (s.cursor, s.selection, s.first_visible_line, s.view_version),
        (37, Some((37, 42)), 4, 9)
    );
    event(&mut f, 225, 2);
    stroke(&mut f, 80);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!(
        (s.cursor, s.selection, s.first_visible_line, s.view_version),
        (37, None, 4, 10)
    );
    stroke(&mut f, 42);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!(s.bytes, expected);
    assert_eq!(
        (
            s.cursor,
            s.selection,
            s.first_visible_line,
            s.text_generation,
            s.view_version
        ),
        (36, None, 4, 2, 11)
    );
    assert!(matches!(s.phase, NativeEditorPhase::Unsaved));
    f.finish();

    let mut f = NativeFixture::new(&body);
    scrolled_selected_checkpoint(&mut f, &body);
    event(&mut f, 0, 3);
    stroke(&mut f, 81);
    let s = f.h.editor_observation(&f.editor, 2).unwrap();
    assert_eq!(s.bytes, body);
    assert_eq!((s.cursor, s.selection, s.first_visible_line), (42, None, 4));
    f.finish();
}

fn counter_leg(counter: NativeSaveCounter) {
    // Positive control independently proves the same key actually changes text and view.
    let mut positive = NativeFixture::new(BODY);
    selected_checkpoint(&mut positive);
    event(&mut positive, 225, 2);
    stroke(&mut positive, 4);
    let s = positive.h.editor_observation(&positive.editor, 2).unwrap();
    assert_eq!(s.bytes, b"abcd\nxaef");
    assert_eq!(
        (s.cursor, s.selection, s.text_generation, s.view_version),
        (7, None, 2, 8)
    );
    assert!(matches!(s.phase, NativeEditorPhase::Unsaved));
    positive.finish();

    let mut f = NativeFixture::new(BODY);
    selected_checkpoint(&mut f);
    event(&mut f, 225, 2);
    // Existing public fixture method ONLY; proposed feature changes these two seed arms.
    f.dc.seed_native_save_counter(counter).unwrap();
    let before = f.h.editor_observation(&f.editor, 2).unwrap();
    match counter {
        NativeSaveCounter::TextGeneration => {
            assert_eq!((before.text_generation, before.view_version), (u64::MAX, 7))
        }
        NativeSaveCounter::ViewVersion => {
            assert_eq!((before.text_generation, before.view_version), (1, u64::MAX))
        }
        _ => unreachable!("finite two-counter corpus"),
    }
    // This helper must genuinely render AND compose, then retain the returned frame observation.
    let frame = f.render_and_compose(2);
    assert_eq!(
        (frame.text_generation, frame.view_version),
        (before.text_generation, before.view_version)
    );
    f.send(4, 1, 2);
    assert!(matches!(
        f.h.step_native_editor(&f.editor, 2),
        Err(Status::Exhausted)
    ));
    assert_eq!(f.h.editor_observation(&f.editor, 2).unwrap(), before);
    // Private owner assertions must independently compare frame/anchor/preferred/phase/lifetimes.
    // No rerender before that comparison: rerender would replace the evidence being checked.
    event(&mut f, 4, 2);
    f.finish();
}
#[test]
fn adapter_native_actual_text_max_rejection_is_atomic() {
    counter_leg(NativeSaveCounter::TextGeneration);
}
#[test]
fn adapter_native_actual_view_max_rejection_is_atomic() {
    counter_leg(NativeSaveCounter::ViewVersion);
}
