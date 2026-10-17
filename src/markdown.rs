//! Markdown line prefixes (quotes and list markers) for Enter-key continuation.

/// The block prefix of a markdown line: indent, block quotes and an optional
/// list marker, e.g. `  > - [x] `.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Marker {
    /// Number of chars the whole marker spans, including trailing spaces.
    pub(crate) len: usize,
    /// Indent and quote markers, copied verbatim to the next line.
    head: Vec<char>,
    list: Option<List>,
}

#[derive(Debug, PartialEq, Eq)]
struct List {
    kind: Kind,
    /// Whitespace after the marker (and after the checkbox, if any).
    gap: Vec<char>,
    task: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Kind {
    Bullet(char),
    Ordered { number: usize, delim: char },
}

impl Marker {
    /// Parses the marker at the start of `line`, if there is one.
    pub(crate) fn parse(line: &[char]) -> Option<Marker> {
        let mut i = 0;
        while matches!(line.get(i), Some(' ' | '\t')) {
            i += 1;
        }
        // Block quotes: `>` with an optional single space, possibly nested.
        while line.get(i) == Some(&'>') {
            i += 1;
            if line.get(i) == Some(&' ') {
                i += 1;
            }
        }
        let head = line[..i].to_vec();

        let list = Self::parse_list(line, &mut i);
        if list.is_none() && head.iter().all(|c| matches!(c, ' ' | '\t')) {
            return None;
        }
        Some(Marker {
            len: i,
            head,
            list,
        })
    }

    fn parse_list(line: &[char], i: &mut usize) -> Option<List> {
        let mut j = *i;
        let kind = match line.get(j)? {
            c @ ('-' | '*' | '+') => {
                j += 1;
                Kind::Bullet(*c)
            }
            c if c.is_ascii_digit() => {
                let start = j;
                while line.get(j).is_some_and(char::is_ascii_digit) {
                    j += 1;
                }
                if j - start > 9 {
                    return None;
                }
                let number = line[start..j].iter().collect::<String>().parse().ok()?;
                let delim = *line.get(j).filter(|c| matches!(c, '.' | ')'))?;
                j += 1;
                Kind::Ordered { number, delim }
            }
            _ => return None,
        };
        let mut gap = take_spaces(line, &mut j);
        if gap.is_empty() {
            return None;
        }
        let mut task = false;
        if matches!(line.get(j..j + 3), Some(['[', ' ' | 'x' | 'X', ']']))
            && line.get(j + 3) == Some(&' ')
        {
            j += 3;
            gap = take_spaces(line, &mut j);
            task = true;
        }
        *i = j;
        Some(List { kind, gap, task })
    }

    /// The prefix for the `k`-th line after the current one (`k >= 1`).
    pub(crate) fn continuation(&self, k: usize) -> Vec<char> {
        let mut out = self.head.clone();
        if let Some(list) = &self.list {
            match list.kind {
                Kind::Bullet(c) => out.push(c),
                Kind::Ordered { number, delim } => {
                    out.extend(number.saturating_add(k).to_string().chars());
                    out.push(delim);
                }
            }
            if list.task {
                // The gap before the checkbox is not stored, so it is a single space.
                out.extend([' ', '[', ' ', ']']);
            }
            out.extend(&list.gap);
        }
        out
    }
}

/// Consumes spaces and tabs from `*j`, returning them.
fn take_spaces(line: &[char], j: &mut usize) -> Vec<char> {
    let start = *j;
    while matches!(line.get(*j), Some(' ' | '\t')) {
        *j += 1;
    }
    line[start..*j].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cont(s: &str) -> Option<String> {
        let chars: Vec<char> = s.chars().collect();
        Marker::parse(&chars).map(|m| m.continuation(1).into_iter().collect())
    }

    #[test]
    fn continuations() {
        assert_eq!(cont("> q").as_deref(), Some("> "));
        assert_eq!(cont("- a").as_deref(), Some("- "));
        assert_eq!(cont("* a").as_deref(), Some("* "));
        assert_eq!(cont("+ a").as_deref(), Some("+ "));
        assert_eq!(cont("1. a").as_deref(), Some("2. "));
        assert_eq!(cont("9) a").as_deref(), Some("10) "));
        assert_eq!(cont("- [ ] a").as_deref(), Some("- [ ] "));
        assert_eq!(cont("- [x] a").as_deref(), Some("- [ ] "));
        assert_eq!(cont("  - a").as_deref(), Some("  - "));
        assert_eq!(cont("\t1.  a").as_deref(), Some("\t2.  "));
        assert_eq!(cont("> > q").as_deref(), Some("> > "));
        assert_eq!(cont("> - a").as_deref(), Some("> - "));
        assert_eq!(cont(">q").as_deref(), Some(">"));
    }

    #[test]
    fn not_markers() {
        assert_eq!(cont(""), None);
        assert_eq!(cont("plain"), None);
        assert_eq!(cont("  indented"), None);
        assert_eq!(cont("-a"), None);
        assert_eq!(cont("**bold**"), None);
        assert_eq!(cont("1.5"), None);
        assert_eq!(cont("12345678901. a"), None);
    }

    #[test]
    fn marker_len() {
        let m = |s: &str| Marker::parse(&s.chars().collect::<Vec<_>>()).unwrap().len;
        assert_eq!(m("- a"), 2);
        assert_eq!(m("- [ ] a"), 6);
        assert_eq!(m("- [ ] "), 6);
        assert_eq!(m("  1. "), 5);
        assert_eq!(m("> - "), 4);
    }
}
