use crate::helper::char_width;
use ratatui_core::style::Style;
use ratatui_core::text::Span;

#[derive(Default)]
pub(crate) struct LineWrapper;

impl LineWrapper {
    /// Word-aware wrap: returns the char-index ranges `[start, end)` of each
    /// visual row for a logical line at `max_width` display columns.
    ///
    /// A row is broken at the last space that fits (the space stays on the
    /// current row); a word longer than `max_width` is hard-broken. The ranges
    /// are contiguous and cover the whole line, so every char belongs to exactly
    /// one row. This is the single source of truth for wrapping, shared by the
    /// renderer, viewport row-counting, mouse mapping and the `gj`/`gk` motions.
    pub(crate) fn wrap_ranges(
        chars: &[char],
        max_width: usize,
        tab_width: usize,
    ) -> Vec<(usize, usize)> {
        if max_width == 0 || chars.is_empty() {
            return vec![(0, chars.len())];
        }
        let n = chars.len();
        let mut rows = Vec::new();
        let mut start = 0;
        while start < n {
            let mut w = 0;
            let mut end = start;
            while end < n {
                let cw = char_width(chars[end], tab_width);
                if w + cw > max_width && end > start {
                    break;
                }
                w += cw;
                end += 1;
            }
            if end >= n {
                rows.push((start, n));
                break;
            }
            // Prefer a word break: the last space strictly before `end`.
            match (start + 1..end).rev().find(|&k| chars[k] == ' ') {
                Some(sp) => {
                    rows.push((start, sp + 1)); // keep the space on this row
                    start = sp + 1;
                }
                None => {
                    rows.push((start, end)); // hard break mid-word
                    start = end;
                }
            }
        }
        rows
    }

    pub(crate) fn wrap_line(line: &[char], max_width: usize, tab_width: usize) -> Vec<Vec<char>> {
        Self::wrap_ranges(line, max_width, tab_width)
            .into_iter()
            .map(|(s, e)| line[s..e].to_vec())
            .collect()
    }

    pub(crate) fn wrap_spans(
        spans: Vec<Span<'_>>,
        max_width: usize,
        tab_width: usize,
    ) -> Vec<Vec<Span<'_>>> {
        // Flatten to (char, style), wrap on the char sequence, then regroup each
        // visual row back into runs of equal style.
        let flat: Vec<(char, Style)> = spans
            .iter()
            .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
            .collect();
        let chars: Vec<char> = flat.iter().map(|(c, _)| *c).collect();

        let mut out = Vec::new();
        for (s, e) in Self::wrap_ranges(&chars, max_width, tab_width) {
            let mut row: Vec<Span> = Vec::new();
            let mut i = s;
            while i < e {
                let style = flat[i].1;
                let mut text = String::new();
                while i < e && flat[i].1 == style {
                    text.push(flat[i].0);
                    i += 1;
                }
                row.push(Span::styled(text, style));
            }
            out.push(row);
        }
        if out.is_empty() {
            out.push(Vec::new());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_ranges_breaks_on_words() {
        let line: Vec<char> = "aaaa bbbb cccc".chars().collect();
        // width 6: "aaaa " | "bbbb " | "cccc"
        assert_eq!(
            LineWrapper::wrap_ranges(&line, 6, 2),
            vec![(0, 5), (5, 10), (10, 14)]
        );
    }

    #[test]
    fn test_wrap_ranges_hard_breaks_long_word() {
        let line: Vec<char> = "abcdefghij".chars().collect();
        assert_eq!(
            LineWrapper::wrap_ranges(&line, 4, 2),
            vec![(0, 4), (4, 8), (8, 10)]
        );
    }

    #[test]
    fn test_wrap_spans_regroups_by_style() {
        let spans = vec![Span::raw("aaaa bbbb")];
        let wrapped = LineWrapper::wrap_spans(spans, 6, 2);
        assert_eq!(wrapped[0], vec![Span::raw("aaaa ")]);
        assert_eq!(wrapped[1], vec![Span::raw("bbbb")]);
    }
}
