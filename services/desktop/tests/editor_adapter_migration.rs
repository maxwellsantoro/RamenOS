//! Proposed supplemental target services/desktop/tests/editor_adapter_migration.rs.
//! Reuses existing genuine volatile launch/input fixture; no recorder registration.
#![cfg(feature = "desktop_v0_dev")]
// This shared fixture intentionally serves more assertions than this target.
#[allow(dead_code)]
#[path = "support/editor_host_assertions.rs"]
mod support;
use desktop_service::dev::Status;
use desktop_service::editor_dev::SaveState;
use support::{Fixture, Live};

const BODY: &[u8] = b"abcd\nx\nabcdef";

fn event(f: &mut Fixture, l: &mut Live, usage: u32, phase: u32) {
    f.d.send(&f.h, usage, phase, 2);
    let step = l.client.step(2).expect("actual focus/input delivery");
    assert!(step.consumed_sequence.is_some());
}
fn stroke(f: &mut Fixture, l: &mut Live, usage: u32) {
    event(f, l, usage, 1);
    event(f, l, usage, 2);
}
fn selected_checkpoint(f: &mut Fixture, l: &mut Live) {
    stroke(f, l, 74); // Home to logical line start 7.
    for _ in 0..4 {
        stroke(f, l, 79);
    }
    event(f, l, 225, 1);
    stroke(f, l, 82); // Shift+Up, short line: preferred column 4.
    let s = l.client.snapshot();
    assert_eq!(s.bytes, BODY);
    assert_eq!(
        (s.cursor, s.selection, s.first_visible_line),
        (6, Some((6, 11)), 0)
    );
}
fn scrolled_selected_checkpoint(f: &mut Fixture, l: &mut Live, body: &[u8]) {
    stroke(f, l, 74);
    for _ in 0..4 {
        stroke(f, l, 79);
    }
    event(f, l, 225, 1);
    stroke(f, l, 82);
    let s = l.client.snapshot();
    assert_eq!(s.bytes, body);
    // 31 leading LF shift the selected indices; anchor42/preferred4 must survive.
    assert_eq!(
        (s.cursor, s.selection, s.first_visible_line),
        (37, Some((37, 42)), 4)
    );
}

#[test]
fn adapter_legacy_selected_snapshot_continuation_and_policies() {
    let mut f = Fixture::new(BODY);
    let mut l = f.launch(1);
    assert_eq!(
        (
            l.client.snapshot().cursor,
            l.client.snapshot().first_visible_line
        ),
        (13, 0)
    );
    assert!(matches!(
        l.client.snapshot().save,
        SaveState::Saved { revision: 1 }
    ));
    selected_checkpoint(&mut f, &mut l);
    stroke(&mut f, &mut l, 81);
    assert_eq!(
        (l.client.snapshot().cursor, l.client.snapshot().selection),
        (11, None)
    );
    stroke(&mut f, &mut l, 82);
    assert_eq!(
        (l.client.snapshot().cursor, l.client.snapshot().selection),
        (6, Some((6, 11)))
    );
    event(&mut f, &mut l, 225, 2);
    stroke(&mut f, &mut l, 80);
    assert_eq!(
        (l.client.snapshot().cursor, l.client.snapshot().selection),
        (6, None)
    );
    stroke(&mut f, &mut l, 42);
    assert_eq!(l.client.snapshot().bytes, b"abcd\n\nabcdef");
    assert_eq!(
        (l.client.snapshot().cursor, l.client.snapshot().selection),
        (5, None)
    );
    assert!(matches!(l.client.snapshot().save, SaveState::Unsaved));

    let mut f = Fixture::new(BODY);
    let mut l = f.launch(1);
    selected_checkpoint(&mut f, &mut l);
    event(&mut f, &mut l, 0, 3); // Genuine input Reset clears modifiers and legacy preferred column.
    stroke(&mut f, &mut l, 81);
    assert_eq!(
        (l.client.snapshot().cursor, l.client.snapshot().selection),
        (8, None)
    );

    let mut f = Fixture::new(&[b'\n'; 4096]);
    let mut l = f.launch(1);
    assert_eq!(l.client.snapshot().first_visible_line, 4067); // CursorVisible: row4096 - 29.
    let before = l.client.snapshot();
    f.d.send(&f.h, 4, 1, 2);
    assert!(matches!(l.client.step(2), Err(Status::Exhausted)));
    let after = l.client.snapshot();
    assert_eq!(after.bytes, before.bytes);
    assert_eq!(
        (after.cursor, after.selection, after.first_visible_line),
        (before.cursor, before.selection, before.first_visible_line)
    );
    assert_eq!(after.confirmed_revision, before.confirmed_revision);
    assert_eq!(after.save, before.save);
    event(&mut f, &mut l, 4, 2);

    let mut body = vec![b'\n'; 31];
    body.extend_from_slice(BODY);
    let mut expected = vec![b'\n'; 31];
    expected.extend_from_slice(b"abcd\n\nabcdef");
    let mut f = Fixture::new(&body);
    let mut l = f.launch(1);
    assert_eq!(
        (
            l.client.snapshot().cursor,
            l.client.snapshot().first_visible_line
        ),
        (44, 4)
    );
    scrolled_selected_checkpoint(&mut f, &mut l, &body);
    stroke(&mut f, &mut l, 81);
    let s = l.client.snapshot();
    assert_eq!((s.cursor, s.selection, s.first_visible_line), (42, None, 4));
    stroke(&mut f, &mut l, 82);
    let s = l.client.snapshot();
    assert_eq!(
        (s.cursor, s.selection, s.first_visible_line),
        (37, Some((37, 42)), 4)
    );
    event(&mut f, &mut l, 225, 2);
    stroke(&mut f, &mut l, 80);
    let s = l.client.snapshot();
    assert_eq!((s.cursor, s.selection, s.first_visible_line), (37, None, 4));
    stroke(&mut f, &mut l, 42);
    let s = l.client.snapshot();
    assert_eq!(s.bytes, expected);
    assert_eq!((s.cursor, s.selection, s.first_visible_line), (36, None, 4));
    assert!(matches!(s.save, SaveState::Unsaved));

    let mut f = Fixture::new(&body);
    let mut l = f.launch(1);
    scrolled_selected_checkpoint(&mut f, &mut l, &body);
    event(&mut f, &mut l, 0, 3);
    stroke(&mut f, &mut l, 81);
    let s = l.client.snapshot();
    assert_eq!(s.bytes, body);
    assert_eq!((s.cursor, s.selection, s.first_visible_line), (39, None, 4));
}
