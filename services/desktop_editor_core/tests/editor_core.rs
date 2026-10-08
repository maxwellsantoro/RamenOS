//! Assertion-only proposed pure-core consumer. Never Focus, process or Save proof.
//! Root registers the future crate/test only after API and assertions freeze.
use desktop_editor_core::{
    EditEffect, EditError, EditorHint, EditorModel, EditorSnapshot, FRAME_BYTES, FontRows, HEIGHT,
    InitialView, ResetPolicy, STRIDE, TEXT_LIMIT, WIDTH,
};
use sha2::{Digest, Sha256};

const FONT: &[u8; 1536] = include_bytes!("fixtures/ascii8x16_v0.bin");
const FONT_SHA: &str = "50ab6522c495f06a69590ccb6958458be15e02a2ca4c6b1024b052e826d21c64";
const OLD_SHA: &str = "69679689bb8a4cc1ce90a4f761525f8820f73056401d116736443a6585fee3c1";
const NEW_SHA: &str = "cfa160298051a6ece5616dc73eb963da3ae6fe55324c59b6370bc6fe1281cf7f";
fn hash(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn font() -> FontRows {
    std::array::from_fn(|i| FONT[i * 16..(i + 1) * 16].try_into().unwrap())
}
fn model(b: &[u8]) -> EditorModel {
    EditorModel::from_selected_ascii(b, 1, InitialView::KeepTop).unwrap()
}
fn draw(m: &EditorModel) -> Vec<u8> {
    let mut p = vec![0xa5; FRAME_BYTES];
    m.rasterize_bgra(&font(), &mut p).unwrap();
    p
}
fn press(m: &mut EditorModel, usage: u32, modifiers: u32) -> EditEffect {
    m.apply_key(usage, modifiers).unwrap()
}
fn unchanged() -> EditEffect {
    EditEffect {
        text_changed: false,
        view_changed: false,
        overflow_rejected: false,
        hint: EditorHint::None,
    }
}
fn pixel(p: &[u8], x: usize, y: usize) -> &[u8] {
    &p[y * STRIDE + x * 4..y * STRIDE + x * 4 + 4]
}
// Independent fixed-cell oracle. It has no positions/scroll/editor algorithm:
// each test supplies its explicit expected visible cells and caret location.
fn expected(
    cells: &[(usize, usize, u8)],
    selection_cells: &[(usize, usize)],
    caret: (usize, usize),
) -> Vec<u8> {
    let mut p = vec![255; 640 * 480 * 4];
    for &(r, c) in selection_cells {
        for y in r * 16..r * 16 + 16 {
            for x in c * 8..c * 8 + 8 {
                p[y * 2560 + x * 4..y * 2560 + x * 4 + 4].copy_from_slice(&[255, 192, 128, 255]);
            }
        }
    }
    for &(r, c, b) in cells {
        assert!((32..=126).contains(&b));
        for y in 0..16 {
            for x in 0..8 {
                if FONT[(b as usize - 32) * 16 + y] & (0x80u8 >> x) != 0 {
                    let o = (r * 16 + y) * 2560 + (c * 8 + x) * 4;
                    p[o..o + 4].copy_from_slice(&[0, 0, 0, 255]);
                }
            }
        }
    }
    for y in caret.0 * 16..caret.0 * 16 + 16 {
        for x in caret.1 * 8..caret.1 * 8 + 2 {
            p[y * 2560 + x * 4..y * 2560 + x * 4 + 4].copy_from_slice(&[0, 0, 0, 255]);
        }
    }
    p
}

#[test]
fn pure_d08_01_initial_policies_preserve_legacy_and_native() {
    assert_eq!(
        (TEXT_LIMIT, WIDTH, HEIGHT, STRIDE, FRAME_BYTES),
        (4096, 640, 480, 2560, 1_228_800)
    );
    let body = vec![b'\n'; 4096];
    let legacy = EditorModel::from_selected_ascii(&body, 0, InitialView::CursorVisible)
        .unwrap()
        .snapshot();
    let native = EditorModel::from_selected_ascii(&body, 1, InitialView::KeepTop)
        .unwrap()
        .snapshot();
    assert_eq!(
        (
            legacy.cursor,
            legacy.first_visible_line,
            legacy.text_generation
        ),
        (4096, 4067, 0)
    );
    assert_eq!(
        (
            native.cursor,
            native.first_visible_line,
            native.text_generation
        ),
        (4096, 0, 1)
    );
    assert_eq!(legacy.bytes, native.bytes);
    assert_eq!(
        (
            native.selection,
            native.selection_anchor,
            native.preferred_column
        ),
        (None, None, None)
    );
    let restored = EditorModel::from_snapshot(legacy.clone()).unwrap();
    assert_eq!(restored.snapshot(), legacy);
}

#[test]
fn pure_d08_02_constructor_denials_and_real_ascii_positive() {
    let mut allowed = vec![9, 10];
    allowed.extend(32..=126);
    assert_eq!(model(&allowed).snapshot().bytes, allowed);
    for b in [
        vec![0],
        vec![13],
        vec![127],
        vec![128],
        vec![255],
        "é".as_bytes().to_vec(),
    ] {
        assert!(matches!(
            EditorModel::from_selected_ascii(&b, 1, InitialView::KeepTop),
            Err(EditError::InvalidAscii)
        ));
    }
    assert!(matches!(
        EditorModel::from_selected_ascii(&vec![b'a'; 4097], 1, InitialView::KeepTop),
        Err(EditError::BodyTooLong)
    ));
    assert_eq!(model(&vec![b'a'; 4096]).snapshot().bytes.len(), 4096);
    let valid: EditorSnapshot = model(b"abc").snapshot();
    let mut cases = Vec::new();
    let mut s = valid.clone();
    s.cursor = usize::MAX;
    cases.push(s);
    let mut s = valid.clone();
    s.selection = Some((2, 1));
    cases.push(s);
    let mut s = valid.clone();
    s.selection = Some((1, 1));
    cases.push(s);
    let mut s = valid.clone();
    s.selection = Some((0, 4));
    cases.push(s);
    let mut s = valid.clone();
    s.selection = Some((0, 2));
    s.selection_anchor = Some(0);
    cases.push(s); // cursor3 inconsistent
    let mut s = valid.clone();
    s.selection_anchor = Some(4);
    cases.push(s);
    let mut s = valid.clone();
    s.preferred_column = Some(4097);
    cases.push(s);
    let mut s = valid.clone();
    s.first_visible_line = usize::MAX;
    cases.push(s);
    let mut s = valid.clone();
    s.first_visible_line = 1;
    cases.push(s); // one-line document maxrow0
    for s in cases {
        assert!(matches!(
            EditorModel::from_snapshot(s),
            Err(EditError::InvalidSnapshot)
        ));
    }
    assert_eq!(
        EditorModel::from_snapshot(valid.clone())
            .unwrap()
            .snapshot(),
        valid
    );
}

#[test]
fn pure_d08_03_exact_us_key_translation() {
    let mut m = model(b"");
    let mut expected_bytes: Vec<u8> = Vec::new();
    for modifiers in [0, 2] {
        for usage in 4..=29 {
            press(&mut m, usage, modifiers);
        }
        expected_bytes.extend(if modifiers == 0 {
            b"abcdefghijklmnopqrstuvwxyz"
        } else {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        });
        for usage in 30..=39 {
            press(&mut m, usage, modifiers);
        }
        expected_bytes.extend(if modifiers == 0 {
            b"1234567890"
        } else {
            b"!@#$%^&*()"
        });
        for usage in [40, 43, 44, 45, 46, 47, 48, 49, 51, 52, 53, 54, 55, 56] {
            press(&mut m, usage, modifiers);
        }
        expected_bytes.extend(if modifiers == 0 {
            b"\n\t -=[]\\;'`,./"
        } else {
            b"\n\t _+{}|:\"~<>?"
        });
    }
    let s = m.snapshot();
    assert_eq!(s.bytes, expected_bytes);
    assert_eq!(s.cursor, expected_bytes.len());
    assert_eq!(s.text_generation, 1 + expected_bytes.len() as u64);
}

#[test]
fn pure_d08_04_select_all_replace_and_exact_generations() {
    let mut m = model(b"note=old\n");
    assert_eq!(
        press(&mut m, 4, 1),
        EditEffect {
            view_changed: true,
            ..unchanged()
        }
    );
    assert_eq!(m.snapshot().selection, Some((0, 9)));
    for usage in [17, 18, 23, 8, 46, 17, 8, 26, 40] {
        assert!(press(&mut m, usage, 0).text_changed);
    }
    let s = m.snapshot();
    assert_eq!(s.bytes, b"note=new\n");
    assert_eq!(s.cursor, 9);
    assert_eq!(
        (
            s.selection,
            s.selection_anchor,
            s.preferred_column,
            s.text_generation
        ),
        (None, None, None, 10)
    );
    assert_eq!(press(&mut m, 4, 3).hint, EditorHint::None); // existing Ctrl+Shift+A selects all
    assert_eq!(m.snapshot().selection, Some((0, 9)));
}

#[test]
fn pure_d08_05_cursor_selection_collapse_home_end_backspace() {
    let mut m = model(b"abcd\nxy");
    press(&mut m, 74, 0);
    assert_eq!(m.snapshot().cursor, 5);
    press(&mut m, 77, 2);
    assert_eq!(m.snapshot().selection, Some((5, 7)));
    press(&mut m, 80, 0);
    assert_eq!((m.snapshot().cursor, m.snapshot().selection), (5, None));
    press(&mut m, 77, 0);
    press(&mut m, 80, 2);
    assert_eq!(m.snapshot().selection, Some((6, 7)));
    press(&mut m, 79, 0);
    assert_eq!((m.snapshot().cursor, m.snapshot().selection), (7, None));
    press(&mut m, 80, 2);
    press(&mut m, 42, 0);
    assert_eq!(m.snapshot().bytes, b"abcd\nx");
    press(&mut m, 4, 1);
    press(&mut m, 42, 0);
    assert_eq!(
        (m.snapshot().bytes, m.snapshot().cursor),
        (Vec::<u8>::new(), 0)
    );
    let s = m.snapshot();
    assert_eq!(press(&mut m, 42, 0), unchanged());
    assert_eq!(m.snapshot(), s);
    for usage in [74, 77, 79, 80, 81, 82] {
        assert_eq!(press(&mut m, usage, 0), unchanged());
    }
}

#[test]
fn pure_d08_06_logical_vertical_preferred_column() {
    let mut m = model(b"abcd\nx\nabcdef");
    press(&mut m, 74, 0);
    for _ in 0..4 {
        press(&mut m, 79, 0);
    }
    assert_eq!(m.snapshot().cursor, 11);
    for (usage, cursor) in [(82, 6), (82, 4), (81, 6), (81, 11)] {
        press(&mut m, usage, 0);
        assert_eq!(m.snapshot().cursor, cursor);
        assert_eq!(m.snapshot().preferred_column, Some(4));
    }
    press(&mut m, 82, 0);
    press(&mut m, 80, 0);
    assert_eq!(
        (m.snapshot().cursor, m.snapshot().preferred_column),
        (5, None)
    );
    press(&mut m, 81, 0);
    assert_eq!(m.snapshot().cursor, 7);
}

#[test]
fn pure_d08_07_wrap_and_scroll_are_visual_not_logical_lines() {
    let mut m =
        EditorModel::from_selected_ascii(&vec![b'a'; 4096], 1, InitialView::CursorVisible).unwrap();
    assert_eq!(m.snapshot().first_visible_line, 22); // caret row51, col16, 30 visible rows
    press(&mut m, 74, 0);
    assert_eq!(
        (m.snapshot().cursor, m.snapshot().first_visible_line),
        (0, 0)
    ); // logical Home
    press(&mut m, 77, 0);
    assert_eq!(
        (m.snapshot().cursor, m.snapshot().first_visible_line),
        (4096, 22)
    );
    let mut newline =
        EditorModel::from_selected_ascii(&vec![b'\n'; 4096], 1, InitialView::CursorVisible)
            .unwrap();
    press(&mut newline, 4, 1);
    press(&mut newline, 80, 0);
    assert_eq!(
        (
            newline.snapshot().cursor,
            newline.snapshot().first_visible_line
        ),
        (0, 0)
    );
}

#[test]
fn pure_d08_08_4097th_press_preserves_all_state_and_raster() {
    let mut m = model(&vec![b'a'; 4095]);
    assert!(press(&mut m, 4, 0).text_changed);
    assert_eq!(
        (m.snapshot().bytes.len(), m.snapshot().text_generation),
        (4096, 2)
    );
    let before = m.snapshot();
    let pixels = draw(&m);
    assert_eq!(
        press(&mut m, 4, 0),
        EditEffect {
            overflow_rejected: true,
            ..unchanged()
        }
    );
    assert_eq!(m.snapshot(), before);
    assert_eq!(draw(&m), pixels);
    press(&mut m, 42, 0);
    assert!(press(&mut m, 5, 0).text_changed);
    assert_eq!(m.snapshot().bytes.len(), 4096);
    press(&mut m, 4, 1);
    assert!(press(&mut m, 4, 2).text_changed);
    assert_eq!(m.snapshot().bytes, b"A");
}

#[test]
fn pure_d08_09_4096_real_local_presses_without_injected_body() {
    let mut m = model(b"");
    for _ in 0..4096 {
        assert!(press(&mut m, 4, 0).text_changed);
    }
    assert_eq!(m.snapshot().bytes, vec![b'a'; 4096]);
    assert_eq!(
        (
            m.snapshot().cursor,
            m.snapshot().text_generation,
            m.snapshot().first_visible_line
        ),
        (4096, 4097, 22)
    );
    let prior = m.snapshot();
    assert!(press(&mut m, 4, 0).overflow_rejected);
    assert_eq!(m.snapshot(), prior);
    // These are4096 pure presses, NOT8192 authenticated process deliveries/acks.
}

#[test]
fn pure_d08_10_counter_max_atomicity_and_same_byte_replace() {
    let mut state = model(b"a").snapshot();
    state.text_generation = u64::MAX;
    state.selection = Some((0, 1));
    state.selection_anchor = Some(0);
    let mut m = EditorModel::from_snapshot(state).unwrap();
    let e = press(&mut m, 4, 0);
    assert!(!e.text_changed);
    assert!(e.view_changed);
    assert_eq!(m.snapshot().text_generation, u64::MAX);
    assert_eq!(m.snapshot().selection, None);
    let before = m.snapshot();
    let p = draw(&m);
    assert_eq!(m.apply_key(5, 0), Err(EditError::CounterExhausted));
    assert_eq!(m.snapshot(), before);
    assert_eq!(draw(&m), p);
    assert!(press(&mut m, 80, 0).view_changed);
    assert_eq!(m.snapshot().text_generation, u64::MAX);
    let mut full = model(&vec![b'a'; 4096]).snapshot();
    full.text_generation = u64::MAX;
    let mut full = EditorModel::from_snapshot(full).unwrap();
    let before = full.snapshot();
    assert_eq!(
        press(&mut full, 4, 0),
        EditEffect {
            overflow_rejected: true,
            ..unchanged()
        }
    );
    assert_eq!(full.snapshot(), before); // length rejection precedes counter addition
}

#[test]
fn pure_d08_11_reset_policies_preserve_adapter_behavior() {
    let mut m = model(b"abcd\nx\nabcdef");
    press(&mut m, 74, 0);
    for _ in 0..4 {
        press(&mut m, 79, 0);
    }
    press(&mut m, 82, 2);
    let before = m.snapshot();
    assert_eq!(
        (
            before.preferred_column,
            before.selection_anchor,
            before.selection
        ),
        (Some(4), Some(11), Some((6, 11)))
    );
    assert_eq!(
        m.reset_navigation(ResetPolicy::PreserveNavigation),
        unchanged()
    );
    assert_eq!(m.snapshot(), before);
    assert_eq!(
        m.reset_navigation(ResetPolicy::ClearPreferredColumn),
        unchanged()
    );
    let mut expected_state = before;
    expected_state.preferred_column = None;
    assert_eq!(m.snapshot(), expected_state); // selection, anchor, bytes, cursor and scroll stay intact
}

#[test]
fn pure_d08_12_hints_and_invalid_presses_have_no_authority_or_mutation() {
    let mut m = model(b"note=old\n");
    let prior = m.snapshot();
    let p = draw(&m);
    assert_eq!(
        press(&mut m, 22, 1),
        EditEffect {
            hint: EditorHint::SaveRequested,
            ..unchanged()
        }
    );
    assert_eq!(m.snapshot(), prior);
    assert_eq!(draw(&m), p);
    for usage in [224, 225, 41] {
        assert_eq!(press(&mut m, usage, 0), unchanged());
        assert_eq!(m.snapshot(), prior);
    }
    for usage in [0, 1, 50, 57, 73, 78, 83, 223, 226, u32::MAX] {
        assert_eq!(m.apply_key(usage, 0), Err(EditError::UnsupportedKey));
        assert_eq!(m.snapshot(), prior);
        assert_eq!(draw(&m), p);
    }
    for modifiers in [4, 8, u32::MAX] {
        assert_eq!(m.apply_key(4, modifiers), Err(EditError::InvalidModifiers));
        assert_eq!(m.snapshot(), prior);
    }
    for usage in [5, 20, 21, 41] {
        assert_eq!(m.apply_key(usage, 1), Err(EditError::UnsupportedKey));
        assert_eq!(m.snapshot(), prior);
    }
    assert!(press(&mut m, 5, 0).text_changed); // independent valid positive after denials
}

#[test]
fn pure_d08_13_existing_old_and_new_full_raster_hashes() {
    assert_eq!(hash(FONT), FONT_SHA);
    assert_eq!(
        hash(include_bytes!("fixtures/ascii8x16_v0.golden.json")),
        "90141218185439b97a6d22a959f1c69bbbf17b6c2408d8eabb2069fb76c1499c"
    );
    for (body, digest) in [
        (b"note=old\n".as_slice(), OLD_SHA),
        (b"note=new\n".as_slice(), NEW_SHA),
    ] {
        let m = model(body);
        let p = draw(&m);
        assert_eq!(p.len(), 1_228_800);
        assert_eq!(hash(&p), digest);
        let cells: Vec<_> = body[..8]
            .iter()
            .enumerate()
            .map(|(c, b)| (0, c, *b))
            .collect();
        assert_eq!(p, expected(&cells, &[], (1, 0)));
        for (x, y, v) in [
            (0, 0, 255),
            (0, 4, 0),
            (4, 4, 0),
            (5, 4, 255),
            (44, 4, 0),
            (45, 4, 255),
            (0, 16, 0),
            (1, 31, 0),
            (2, 16, 255),
            (639, 479, 255),
        ] {
            assert_eq!(pixel(&p, x, y), &[v, v, v, 255]);
        }
        assert!(p.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }
}

#[test]
fn pure_d08_14_exact_tab_selection_and_caret_precedence() {
    let mut m = model(b"a\tZ");
    press(&mut m, 74, 0);
    press(&mut m, 79, 0);
    press(&mut m, 79, 2);
    assert_eq!(
        (m.snapshot().cursor, m.snapshot().selection),
        (2, Some((1, 2)))
    );
    let p = draw(&m);
    assert_eq!(
        p,
        expected(
            &[(0, 0, b'a'), (0, 4, b'Z')],
            &[(0, 1), (0, 2), (0, 3)],
            (0, 4)
        )
    );
    assert_eq!(pixel(&p, 8, 0), &[255, 192, 128, 255]);
    assert_eq!(pixel(&p, 32, 0), &[0, 0, 0, 255]);
    let mut newline = model(b"\n");
    press(&mut newline, 4, 1);
    assert_eq!(draw(&newline), expected(&[], &[(0, 0)], (1, 0))); // selected LF colors exactly one cell
}

#[test]
fn pure_d08_15_exact_last_column_and_viewport_clipping() {
    let mut m = model(&[b'a'; 79]);
    press(&mut m, 43, 0);
    press(&mut m, 5, 0);
    let mut cells: Vec<_> = (0..79).map(|c| (0, c, b'a')).collect();
    cells.push((1, 0, b'b'));
    assert_eq!(draw(&m), expected(&cells, &[], (1, 1))); // tab at79 wraps to next visual row col0
    press(&mut m, 80, 0);
    press(&mut m, 80, 2); // select tab, caret row0 col79
    assert_eq!(m.snapshot().selection, Some((79, 80)));
    assert_eq!(draw(&m), expected(&cells, &[(0, 79)], (0, 79)));
    let bottom =
        EditorModel::from_selected_ascii(&vec![b'\n'; 4096], 1, InitialView::CursorVisible)
            .unwrap();
    assert_eq!(draw(&bottom), expected(&[], &[], (29, 0))); // entire output outside caret stays white
}

#[test]
fn pure_d08_16_raster_output_length_denials_do_not_write() {
    let m = model(b"note=old\n");
    let before = m.snapshot();
    for len in [0, 1, FRAME_BYTES - 1, FRAME_BYTES + 1] {
        let mut p = vec![0xa5; len];
        assert_eq!(
            m.rasterize_bgra(&font(), &mut p),
            Err(EditError::InvalidBufferLength)
        );
        assert!(p.iter().all(|b| *b == 0xa5));
        assert_eq!(m.snapshot(), before);
    }
    assert_eq!(hash(&draw(&m)), OLD_SHA);
}
