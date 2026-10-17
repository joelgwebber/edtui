use jagged::index::RowIndex;

use super::Execute;
use crate::{
    EditorState,
    helper::{insert_char, line_break},
    markdown::Marker,
};

/// Inserts a single character at the current cursor position.
///
/// In single-line mode, newline characters (`\n`, `\r`) are ignored.
#[derive(Clone, Debug, Copy)]
pub struct InsertChar(pub char);

impl Execute for InsertChar {
    fn execute(&mut self, state: &mut EditorState) {
        // Block newline characters in single-line mode
        if state.view.single_line && matches!(self.0, '\n' | '\r') {
            return;
        }
        insert_char(&mut state.lines, &mut state.cursor, self.0, false);

        // Capture insert session for the dot-repeat command
        if let Some(buffer) = &mut state.insert_recording {
            buffer.push(self.0);
        }
    }

    fn is_repeatable(&self) -> bool {
        true
    }
}

/// Inserts a newline at the current cursor position.
///
/// In single-line mode, this action is ignored.
#[derive(Clone, Debug, Copy)]
pub struct LineBreak(pub usize);

impl Execute for LineBreak {
    fn set_count(&mut self, count: usize) {
        self.0 = count;
    }

    fn execute(&mut self, state: &mut EditorState) {
        if state.view.single_line {
            return;
        }
        if state.lines.is_empty() {
            state.lines.push(Vec::new());
        }
        if state.markdown_continuation && markdown_line_break(state, self.0) {
            return;
        }
        for _ in 0..self.0 {
            line_break(&mut state.lines, &mut state.cursor);
        }
    }

    fn is_repeatable(&self) -> bool {
        true
    }
}

/// Handles a line break on a markdown quote or list item. Returns `false` when
/// the line has no marker or the cursor is inside it, leaving a plain break.
fn markdown_line_break(state: &mut EditorState, count: usize) -> bool {
    let row = state.cursor.row;
    let Some(line) = state.lines.get(RowIndex::new(row)) else {
        return false;
    };
    let Some(marker) = Marker::parse(line) else {
        return false;
    };
    let col = state.cursor.col;

    // An item with an empty body: the break removes the marker.
    if col >= line.len() && line[marker.len..].iter().all(|c| c.is_whitespace()) {
        state.lines.get_mut(RowIndex::new(row)).map(Vec::clear);
        state.cursor.col = 0;
        return true;
    }
    if col < marker.len {
        return false;
    }

    for k in 1..=count {
        line_break(&mut state.lines, &mut state.cursor);
        for c in marker.continuation(k) {
            insert_char(&mut state.lines, &mut state.cursor, c, false);
        }
    }
    true
}

/// Appends a newline below the current cursor position.
///
/// In single-line mode, this action is ignored.
#[derive(Clone, Debug, Copy)]
pub struct AppendNewline(pub usize);

impl Execute for AppendNewline {
    fn set_count(&mut self, count: usize) {
        self.0 = count;
    }

    fn execute(&mut self, state: &mut EditorState) {
        if state.view.single_line {
            return;
        }
        state.capture();
        state.cursor.col = 0;
        for _ in 0..self.0 {
            if !state.lines.is_empty() {
                state.cursor.row += 1;
            }
            if state.cursor.row < state.lines.len() {
                state.lines.insert(RowIndex::new(state.cursor.row), vec![]);
            } else {
                state.lines.push(vec![]);
            }
        }
    }

    fn is_repeatable(&self) -> bool {
        true
    }
}

/// Appends a newline at the current cursor position.
///
/// In single-line mode, this action is ignored.
#[derive(Clone, Debug, Copy)]
pub struct InsertNewline(pub usize);

impl Execute for InsertNewline {
    fn set_count(&mut self, count: usize) {
        self.0 = count;
    }

    fn execute(&mut self, state: &mut EditorState) {
        if state.view.single_line {
            return;
        }
        state.cursor.col = 0;
        for _ in 0..self.0 {
            state.lines.insert(RowIndex::new(state.cursor.row), vec![]);
        }
    }

    fn is_repeatable(&self) -> bool {
        true
    }
}

/// Pushes a line to the back of the buffer.
/// Does not affect the cursor position.
#[derive(Clone, Debug, Copy)]
pub struct PushLine<'a>(pub &'a str);

impl Execute for PushLine<'_> {
    fn execute(&mut self, state: &mut EditorState) {
        let chars: Vec<char> = self.0.chars().collect();
        state.lines.push(chars);
    }
}

#[cfg(test)]
mod tests {
    use crate::{Index2, Lines};

    use super::*;
    fn test_state() -> EditorState {
        EditorState::new(Lines::from("Hello World!\n\n123."))
    }

    #[test]
    fn test_insert_char() {
        let mut state = test_state();

        InsertChar('!').execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 1));
        assert_eq!(state.lines, Lines::from("!Hello World!\n\n123."));

        state.cursor = Index2::new(0, 13);
        InsertChar('!').execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 14));
        assert_eq!(state.lines, Lines::from("!Hello World!!\n\n123."));
    }

    #[test]
    fn test_insert_char_into_empty_buffer() {
        let mut state = EditorState::new(Lines::from("\n"));
        state.cursor.row = 1;

        InsertChar('a').execute(&mut state);
        assert_eq!(state.cursor, Index2::new(1, 1));
        assert_eq!(state.lines, Lines::from("\na"));
    }

    #[test]
    fn test_insert_char_out_of_bounds() {
        let mut state = EditorState::new(Lines::from("\nb"));
        state.cursor = Index2::new(0, 1);

        InsertChar('a').execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 1));
        assert_eq!(state.lines, Lines::from("a\nb"));
    }

    fn md_state(text: &str, row: usize, col: usize) -> EditorState {
        let mut state = EditorState::new(Lines::from(text));
        state.set_markdown_continuation(true);
        state.mode = crate::EditorMode::Insert;
        state.cursor = Index2::new(row, col);
        state
    }

    fn text(state: &EditorState) -> String {
        state
            .lines
            .iter_row()
            .map(|r| r.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn test_markdown_off_by_default() {
        let mut state = EditorState::new(Lines::from("- a"));
        state.cursor = Index2::new(0, 3);
        LineBreak(1).execute(&mut state);
        assert_eq!(text(&state), "- a\n");
        assert_eq!(state.cursor, Index2::new(1, 0));
    }

    #[test]
    fn test_markdown_continues_markers() {
        for (before, after) in [
            ("- a", "- a\n- "),
            ("* a", "* a\n* "),
            ("+ a", "+ a\n+ "),
            ("> q", "> q\n> "),
            ("  - a", "  - a\n  - "),
            ("1. a", "1. a\n2. "),
            ("9) a", "9) a\n10) "),
            ("- [ ] a", "- [ ] a\n- [ ] "),
            ("- [x] a", "- [x] a\n- [ ] "),
            ("> - a", "> - a\n> - "),
        ] {
            let mut state = md_state(before, 0, before.chars().count());
            LineBreak(1).execute(&mut state);
            assert_eq!(text(&state), after, "{before:?}");
            assert_eq!(
                state.cursor,
                Index2::new(1, after.rsplit('\n').next().unwrap().chars().count())
            );
        }
    }

    #[test]
    fn test_markdown_empty_item_ends_list() {
        for before in ["- ", "> ", "1. ", "- [ ] ", "  * "] {
            let mut state = md_state(&format!("x\n{before}"), 1, before.chars().count());
            LineBreak(1).execute(&mut state);
            assert_eq!(text(&state), "x\n", "{before:?}");
            assert_eq!(state.cursor, Index2::new(1, 0));
        }
    }

    #[test]
    fn test_markdown_split_in_the_middle() {
        let mut state = md_state("- ab", 0, 3);
        LineBreak(1).execute(&mut state);
        assert_eq!(text(&state), "- a\n- b");
        assert_eq!(state.cursor, Index2::new(1, 2));

        // Inside the marker itself the break is plain.
        let mut state = md_state("1. ab", 0, 1);
        LineBreak(1).execute(&mut state);
        assert_eq!(text(&state), "1\n. ab");
    }

    #[test]
    fn test_markdown_plain_lines_unchanged() {
        let mut state = md_state("plain", 0, 5);
        LineBreak(1).execute(&mut state);
        assert_eq!(text(&state), "plain\n");
    }

    #[test]
    fn test_markdown_count_renumbers() {
        let mut state = md_state("1. a", 0, 4);
        LineBreak(3).execute(&mut state);
        assert_eq!(text(&state), "1. a\n2. \n3. \n4. ");
        assert_eq!(state.cursor, Index2::new(3, 3));
    }

    #[test]
    fn test_markdown_single_line_ignored() {
        let mut state = md_state("- a", 0, 3);
        state.set_single_line(true);
        LineBreak(1).execute(&mut state);
        assert_eq!(text(&state), "- a");
    }

    #[test]
    fn test_line_break() {
        let mut state = test_state();

        LineBreak(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(1, 0));
        assert_eq!(state.lines, Lines::from("\nHello World!\n\n123."));

        state.cursor = Index2::new(1, 5);
        LineBreak(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 0));
        assert_eq!(state.lines, Lines::from("\nHello\n World!\n\n123."));
    }

    #[test]
    fn test_line_break_col_out_of_bounds() {
        let mut state = test_state();
        state.cursor.col = 99;

        LineBreak(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(1, 0));
        assert_eq!(state.lines, Lines::from("Hello World!\n\n\n123."));

        state.cursor.col = 99;
        state.cursor.row = 4;
        LineBreak(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(5, 0));
        assert_eq!(state.lines, Lines::from("Hello World!\n\n\n123.\n"));
    }

    #[test]
    fn test_append_newline() {
        let mut state = test_state();

        AppendNewline(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(1, 0));
        assert_eq!(state.lines, Lines::from("Hello World!\n\n\n123."));

        state.cursor = Index2::new(3, 0);
        AppendNewline(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(4, 0));
        assert_eq!(state.lines, Lines::from("Hello World!\n\n\n123.\n"));
    }

    #[test]
    fn test_insert_newline() {
        let mut state = test_state();

        InsertNewline(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));
        assert_eq!(state.lines, Lines::from("\nHello World!\n\n123."));

        state.cursor = Index2::new(2, 1);
        InsertNewline(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 0));
        assert_eq!(state.lines, Lines::from("\nHello World!\n\n\n123."));
    }

    #[test]
    fn test_push_line() {
        let mut state = test_state();

        PushLine("456.").execute(&mut state);
        assert_eq!(state.lines, Lines::from("Hello World!\n\n123.\n456."));
    }
}
