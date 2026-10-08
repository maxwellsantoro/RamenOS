//! Bounded local ASCII edit/raster data shared by host adapters and the editor child.
//! This component authenticates no input, document, process, mapping or Save.

pub const TEXT_LIMIT: usize = 4096;
pub const WIDTH: usize = 640;
pub const HEIGHT: usize = 480;
pub const STRIDE: usize = WIDTH * 4;
pub const FRAME_BYTES: usize = STRIDE * HEIGHT;
const COLS: usize = 80;
const ROWS: usize = 30;
pub type FontRows = [[u8; 16]; 96];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorSnapshot {
    pub bytes: Vec<u8>,
    pub cursor: usize,
    pub selection: Option<(usize, usize)>,
    pub first_visible_line: usize,
    pub selection_anchor: Option<usize>,
    pub preferred_column: Option<usize>,
    pub text_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialView {
    KeepTop,
    CursorVisible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetPolicy {
    PreserveNavigation,
    ClearPreferredColumn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorHint {
    None,
    SaveRequested,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditError {
    BodyTooLong,
    InvalidAscii,
    InvalidSnapshot,
    InvalidModifiers,
    UnsupportedKey,
    CounterExhausted,
    InvalidBufferLength,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditEffect {
    pub text_changed: bool,
    pub view_changed: bool,
    pub overflow_rejected: bool,
    pub hint: EditorHint,
}

pub struct EditorModel {
    state: EditorSnapshot,
}

fn validate_body(bytes: &[u8]) -> Result<(), EditError> {
    if bytes.len() > TEXT_LIMIT {
        return Err(EditError::BodyTooLong);
    }
    if !bytes.iter().all(|b| matches!(*b, 9 | 10 | 32..=126)) {
        return Err(EditError::InvalidAscii);
    }
    Ok(())
}

fn unchanged() -> EditEffect {
    EditEffect {
        text_changed: false,
        view_changed: false,
        overflow_rejected: false,
        hint: EditorHint::None,
    }
}

impl EditorModel {
    pub fn from_selected_ascii(
        bytes: &[u8],
        text_generation: u64,
        initial_view: InitialView,
    ) -> Result<Self, EditError> {
        validate_body(bytes)?;
        let mut model = Self {
            state: EditorSnapshot {
                bytes: bytes.to_vec(),
                cursor: bytes.len(),
                selection: None,
                first_visible_line: 0,
                selection_anchor: None,
                preferred_column: None,
                text_generation,
            },
        };
        if initial_view == InitialView::CursorVisible {
            model.scroll_to_cursor();
        }
        Ok(model)
    }

    pub fn from_snapshot(snapshot: EditorSnapshot) -> Result<Self, EditError> {
        validate_body(&snapshot.bytes)?;
        let length = snapshot.bytes.len();
        if snapshot.cursor > length
            || snapshot.selection_anchor.is_some_and(|a| a > length)
            || snapshot.preferred_column.is_some_and(|c| c > TEXT_LIMIT)
        {
            return Err(EditError::InvalidSnapshot);
        }
        if snapshot.selection.is_some_and(|(start, end)| {
            start >= end
                || end > length
                || !matches!(
                    snapshot.selection_anchor,
                    Some(a) if a == start && snapshot.cursor == end
                        || a == end && snapshot.cursor == start
                )
        }) {
            return Err(EditError::InvalidSnapshot);
        }
        let model = Self { state: snapshot };
        let maximum_row = model.positions()[length].0;
        if model.state.first_visible_line > maximum_row {
            return Err(EditError::InvalidSnapshot);
        }
        Ok(model)
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        self.state.clone()
    }

    pub fn apply_key(&mut self, usage: u32, modifiers: u32) -> Result<EditEffect, EditError> {
        if modifiers & !3 != 0 {
            return Err(EditError::InvalidModifiers);
        }
        if modifiers & 1 != 0 && usage == 22 {
            return Ok(EditEffect {
                hint: EditorHint::SaveRequested,
                ..unchanged()
            });
        }
        let before = (
            self.state.text_generation,
            self.state.cursor,
            self.state.selection,
            self.state.first_visible_line,
        );
        let accepted = self.press(usage, modifiers)?;
        let text_changed = self.state.text_generation != before.0;
        Ok(EditEffect {
            text_changed,
            view_changed: text_changed
                || self.state.cursor != before.1
                || self.state.selection != before.2
                || self.state.first_visible_line != before.3,
            overflow_rejected: !accepted,
            hint: EditorHint::None,
        })
    }

    pub fn reset_navigation(&mut self, policy: ResetPolicy) -> EditEffect {
        if policy == ResetPolicy::ClearPreferredColumn {
            self.state.preferred_column = None;
        }
        unchanged()
    }

    fn press(&mut self, usage: u32, modifiers: u32) -> Result<bool, EditError> {
        if matches!(usage, 224 | 225) {
            return Ok(true);
        }
        if modifiers & 1 != 0 {
            return match usage {
                4 => {
                    self.state.selection_anchor = Some(0);
                    self.state.cursor = self.state.bytes.len();
                    self.state.selection = if self.state.bytes.is_empty() {
                        None
                    } else {
                        Some((0, self.state.bytes.len()))
                    };
                    self.state.preferred_column = None;
                    self.scroll_to_cursor();
                    Ok(true)
                }
                _ => Err(EditError::UnsupportedKey),
            };
        }
        let shift = modifiers & 2 != 0;
        if matches!(usage, 74 | 77 | 79..=82) {
            return self.move_cursor(usage, shift).map(|()| true);
        }
        if usage == 41 {
            return Ok(true);
        }
        if usage == 42 {
            let (start, end) = self
                .state
                .selection
                .unwrap_or((self.state.cursor.saturating_sub(1), self.state.cursor));
            return self.replace(start, end, &[]);
        }
        let byte = match usage {
            4..=29 => b'a' + (usage - 4) as u8 - if shift { 32 } else { 0 },
            30..=39 => {
                if shift {
                    b"!@#$%^&*()"[(usage - 30) as usize]
                } else {
                    b"1234567890"[(usage - 30) as usize]
                }
            }
            40 => b'\n',
            43 => b'\t',
            44 => b' ',
            45 => {
                if shift {
                    b'_'
                } else {
                    b'-'
                }
            }
            46 => {
                if shift {
                    b'+'
                } else {
                    b'='
                }
            }
            47 => {
                if shift {
                    b'{'
                } else {
                    b'['
                }
            }
            48 => {
                if shift {
                    b'}'
                } else {
                    b']'
                }
            }
            49 => {
                if shift {
                    b'|'
                } else {
                    b'\\'
                }
            }
            51 => {
                if shift {
                    b':'
                } else {
                    b';'
                }
            }
            52 => {
                if shift {
                    b'"'
                } else {
                    b'\''
                }
            }
            53 => {
                if shift {
                    b'~'
                } else {
                    b'`'
                }
            }
            54 => {
                if shift {
                    b'<'
                } else {
                    b','
                }
            }
            55 => {
                if shift {
                    b'>'
                } else {
                    b'.'
                }
            }
            56 => {
                if shift {
                    b'?'
                } else {
                    b'/'
                }
            }
            _ => return Err(EditError::UnsupportedKey),
        };
        let (start, end) = self
            .state
            .selection
            .unwrap_or((self.state.cursor, self.state.cursor));
        self.replace(start, end, &[byte])
    }

    fn replace(&mut self, start: usize, end: usize, bytes: &[u8]) -> Result<bool, EditError> {
        let length = self.state.bytes.len() - (end - start) + bytes.len();
        if length > TEXT_LIMIT {
            return Ok(false);
        }
        let changed = self.state.bytes[start..end] != *bytes;
        let next_generation = if changed {
            self.state
                .text_generation
                .checked_add(1)
                .ok_or(EditError::CounterExhausted)?
        } else {
            self.state.text_generation
        };
        self.state.bytes.splice(start..end, bytes.iter().copied());
        self.state.cursor = start + bytes.len();
        self.state.selection = None;
        self.state.selection_anchor = None;
        self.state.preferred_column = None;
        self.state.text_generation = next_generation;
        self.scroll_to_cursor();
        Ok(true)
    }

    fn move_cursor(&mut self, usage: u32, shift: bool) -> Result<(), EditError> {
        let cursor = self.state.cursor;
        let line_start = self.state.bytes[..cursor]
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |i| i + 1);
        let line_end = self.state.bytes[cursor..]
            .iter()
            .position(|b| *b == b'\n')
            .map_or(self.state.bytes.len(), |i| cursor + i);
        let target = match usage {
            74 => line_start,
            77 => line_end,
            79 => {
                if !shift {
                    self.state
                        .selection
                        .map_or((cursor + 1).min(self.state.bytes.len()), |(_, end)| end)
                } else {
                    (cursor + 1).min(self.state.bytes.len())
                }
            }
            80 => {
                if !shift {
                    self.state
                        .selection
                        .map_or(cursor.saturating_sub(1), |(start, _)| start)
                } else {
                    cursor.saturating_sub(1)
                }
            }
            81 | 82 => {
                let column = *self
                    .state
                    .preferred_column
                    .get_or_insert(cursor - line_start);
                if usage == 82 {
                    if line_start == 0 {
                        cursor.min(column)
                    } else {
                        let end = line_start - 1;
                        let start = self.state.bytes[..end]
                            .iter()
                            .rposition(|b| *b == b'\n')
                            .map_or(0, |i| i + 1);
                        start + column.min(end - start)
                    }
                } else if line_end == self.state.bytes.len() {
                    cursor
                } else {
                    let start = line_end + 1;
                    let end = self.state.bytes[start..]
                        .iter()
                        .position(|b| *b == b'\n')
                        .map_or(self.state.bytes.len(), |i| start + i);
                    start + column.min(end - start)
                }
            }
            _ => return Err(EditError::UnsupportedKey),
        };
        if !matches!(usage, 81 | 82) {
            self.state.preferred_column = None;
        }
        if shift {
            let anchor = *self.state.selection_anchor.get_or_insert(cursor);
            self.state.selection = if anchor == target {
                None
            } else {
                Some((anchor.min(target), anchor.max(target)))
            };
        } else {
            self.state.selection_anchor = None;
            self.state.selection = None;
        }
        self.state.cursor = target;
        self.scroll_to_cursor();
        Ok(())
    }

    fn positions(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::with_capacity(self.state.bytes.len() + 1);
        let (mut row, mut column) = (0, 0);
        for b in &self.state.bytes {
            result.push((row, column));
            if *b == b'\n' {
                row += 1;
                column = 0;
            } else {
                column += if *b == b'\t' { 4 - column % 4 } else { 1 };
                row += column / COLS;
                column %= COLS;
            }
        }
        result.push((row, column));
        result
    }

    fn scroll_to_cursor(&mut self) {
        let row = self.positions()[self.state.cursor].0;
        if row < self.state.first_visible_line {
            self.state.first_visible_line = row;
        } else if row >= self.state.first_visible_line + ROWS {
            self.state.first_visible_line = row + 1 - ROWS;
        }
    }

    fn cell(pixels: &mut [u8], row: usize, column: usize, color: [u8; 4]) {
        if row >= ROWS || column >= COLS {
            return;
        }
        for y in row * 16..row * 16 + 16 {
            for x in column * 8..column * 8 + 8 {
                pixels[y * STRIDE + x * 4..y * STRIDE + x * 4 + 4].copy_from_slice(&color);
            }
        }
    }

    pub fn rasterize_bgra(&self, font: &FontRows, pixels: &mut [u8]) -> Result<(), EditError> {
        if pixels.len() != FRAME_BYTES {
            return Err(EditError::InvalidBufferLength);
        }
        pixels.fill(255);
        let positions = self.positions();
        let first = self.state.first_visible_line;
        for (i, b) in self.state.bytes.iter().enumerate() {
            let (row, column) = positions[i];
            if row < first || row >= first + ROWS {
                continue;
            }
            let visible = row - first;
            if self
                .state
                .selection
                .is_some_and(|(start, end)| i >= start && i < end)
            {
                let cells = if *b == b'\t' { 4 - column % 4 } else { 1 };
                for dx in 0..cells {
                    Self::cell(
                        pixels,
                        visible + (column + dx) / COLS,
                        (column + dx) % COLS,
                        [255, 192, 128, 255],
                    );
                }
            }
            if !(32..=126).contains(b) {
                continue;
            }
            for (y, row_bits) in font[*b as usize - 32].iter().enumerate() {
                for x in 0..8 {
                    if row_bits & (128 >> x) != 0 {
                        let o = (visible * 16 + y) * STRIDE + (column * 8 + x) * 4;
                        pixels[o..o + 4].copy_from_slice(&[0, 0, 0, 255]);
                    }
                }
            }
        }
        let (row, column) = positions[self.state.cursor];
        if row >= first && row < first + ROWS {
            for y in (row - first) * 16..(row - first) * 16 + 16 {
                for x in column * 8..column * 8 + 2 {
                    let o = y * STRIDE + x * 4;
                    pixels[o..o + 4].copy_from_slice(&[0, 0, 0, 255]);
                }
            }
        }
        Ok(())
    }
}

pub mod wire;
