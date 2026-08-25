//! Indent / dedent actions: Vim `>>` / `<<` and visual `>` / `<`.
//!
//! One "shiftwidth" is the editor's tab width. Blank lines are left untouched
//! when indenting; dedent removes up to a shiftwidth of leading whitespace.

use jagged::Index2;

use super::Execute;
use crate::helper::skip_whitespace;
use crate::{EditorMode, EditorState};

fn shiftwidth(state: &EditorState) -> usize {
    state.view.tab_width.max(1)
}

fn indent_row(state: &mut EditorState, row: usize, sw: usize) {
    // Leave blank lines unchanged (matching Vim).
    if state.lines.len_col(row).unwrap_or(0) == 0 {
        return;
    }
    for _ in 0..sw {
        state.lines.insert(Index2::new(row, 0), ' ');
    }
}

fn dedent_row(state: &mut EditorState, row: usize, sw: usize) {
    for _ in 0..sw {
        match state.lines.get(Index2::new(row, 0)) {
            Some(&c) if c == ' ' || c == '\t' => {
                let _ = state.lines.remove(Index2::new(row, 0));
            }
            _ => break,
        }
    }
}

fn selection_rows(state: &EditorState) -> Option<(usize, usize)> {
    state.selection.as_ref().map(|s| {
        let (a, b) = (s.start.row, s.end.row);
        (a.min(b), a.max(b))
    })
}

/// After a visual indent/dedent, drop the selection, return to normal mode, and
/// park the cursor on the first non-blank of the top row.
fn finish_visual(state: &mut EditorState, top_row: usize) {
    state.selection = None;
    state.mode = EditorMode::Normal;
    state.cursor.row = top_row;
    state.cursor.col = 0;
    skip_whitespace(&state.lines, &mut state.cursor);
    state.clamp_column();
}

/// Indent `count` lines from the cursor by one shiftwidth. Vim `>>`.
#[derive(Clone, Debug, Copy)]
pub struct IndentLine(pub usize);

impl Execute for IndentLine {
    fn set_count(&mut self, count: usize) {
        self.0 = count;
    }

    fn execute(&mut self, state: &mut EditorState) {
        state.capture();
        let sw = shiftwidth(state);
        let start = state.cursor.row;
        for i in 0..self.0.max(1) {
            let row = start + i;
            if row >= state.lines.len() {
                break;
            }
            indent_row(state, row, sw);
        }
        state.cursor.col = 0;
        skip_whitespace(&state.lines, &mut state.cursor);
    }

    fn is_repeatable(&self) -> bool {
        true
    }
}

/// Dedent `count` lines from the cursor by one shiftwidth. Vim `<<`.
#[derive(Clone, Debug, Copy)]
pub struct DedentLine(pub usize);

impl Execute for DedentLine {
    fn set_count(&mut self, count: usize) {
        self.0 = count;
    }

    fn execute(&mut self, state: &mut EditorState) {
        state.capture();
        let sw = shiftwidth(state);
        let start = state.cursor.row;
        for i in 0..self.0.max(1) {
            let row = start + i;
            if row >= state.lines.len() {
                break;
            }
            dedent_row(state, row, sw);
        }
        state.cursor.col = 0;
        skip_whitespace(&state.lines, &mut state.cursor);
        state.clamp_column();
    }

    fn is_repeatable(&self) -> bool {
        true
    }
}

/// Indent every line spanned by the visual selection. Vim visual `>`.
#[derive(Clone, Debug, Copy)]
pub struct IndentSelection;

impl Execute for IndentSelection {
    fn execute(&mut self, state: &mut EditorState) {
        let Some((r0, r1)) = selection_rows(state) else {
            return;
        };
        state.capture();
        let sw = shiftwidth(state);
        for row in r0..=r1 {
            indent_row(state, row, sw);
        }
        finish_visual(state, r0);
    }

    fn is_repeatable(&self) -> bool {
        true
    }
}

/// Dedent every line spanned by the visual selection. Vim visual `<`.
#[derive(Clone, Debug, Copy)]
pub struct DedentSelection;

impl Execute for DedentSelection {
    fn execute(&mut self, state: &mut EditorState) {
        let Some((r0, r1)) = selection_rows(state) else {
            return;
        };
        state.capture();
        let sw = shiftwidth(state);
        for row in r0..=r1 {
            dedent_row(state, row, sw);
        }
        finish_visual(state, r0);
    }

    fn is_repeatable(&self) -> bool {
        true
    }
}
