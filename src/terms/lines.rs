//! The machine-written `terms:` line of an entry or question, and its validation.

use super::TermsFile;

/// Inserts, replaces, or removes each term's `terms:` line right after its
/// `id:` line, in `lines` (a working copy of `tf.lines`). `pairs_per_entry`
/// gives each term's pairs in the same order as `tf.terms`; an empty list
/// omits the line. Callers write `lines` to disk.
///
/// Used by approve's write step.
pub fn write_terms_lines(
    lines: &mut Vec<String>,
    tf: &TermsFile,
    pairs_per_entry: &[Vec<(String, String)>],
) {
    enum Op {
        Remove,
        Insert(String),
    }

    let mut ops: Vec<(usize, u8, Op)> = Vec::new();
    for (i, t) in tf.terms.iter().enumerate() {
        let pairs = pairs_per_entry.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
        let new_line = format_terms_line(pairs);

        if let Some(idx) = t.terms_line_idx {
            // 1-based line number -> 0-based index.
            ops.push((idx - 1, 0, Op::Remove));
        }
        if let (Some(new_line), Some(id_line)) = (new_line, t.id_line) {
            // 1-based id line number is also the 0-based "insert right after" index.
            ops.push((id_line, 1, Op::Insert(new_line)));
        }
    }

    // Bottom-to-top (position descending) so earlier edits don't shift the
    // indices later edits still need; a remove is applied before an insert
    // at the same position so a "replace in place" lands correctly.
    ops.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (pos, _, op) in ops {
        match op {
            Op::Remove => {
                if pos < lines.len() {
                    lines.remove(pos);
                }
            }
            Op::Insert(s) => {
                let pos = pos.min(lines.len());
                lines.insert(pos, s);
            }
        }
    }
}

pub fn format_terms_line(pairs: &[(String, String)]) -> Option<String> {
    if pairs.is_empty() {
        return None;
    }
    let body = pairs
        .iter()
        .map(|(w, id)| format!("{}={}", w, id))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("terms: {}", body))
}

pub fn parse_terms_line(line: &str) -> Option<Vec<(String, String)>> {
    let rest = line.strip_prefix("terms: ")?;
    if rest.trim().is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for part in rest.split(", ") {
        let (w, id) = part.split_once('=')?;
        out.push((w.to_string(), id.to_string()));
    }
    Some(out)
}

/// A single uuid on a `terms:` line that no longer names an existing,
/// non-retired term.
pub enum ProblemKind {
    /// The uuid names no term in the file at all.
    Gone,
    /// The uuid names a term, but it is retired; carries the term's name.
    Retired(String),
}

/// One `Problem` per uuid `validate_pairs` finds wrong on a line.
pub struct Problem {
    pub uuid: String,
    /// The word the line names the term by.
    pub word: String,
    pub kind: ProblemKind,
}

/// The line does not parse as a `terms:` line at all (per
/// `parse_terms_line`: missing the `terms: ` prefix, or nothing after it).
#[derive(Debug)]
pub struct Malformed;

/// Validates every uuid on one `terms:` line (`line`, exactly as it stands
/// in the file) against `tf`: `Err(Malformed)` when the line itself doesn't
/// parse, otherwise `Ok` with one `Problem`, in the line's own order, for
/// every uuid that no longer names an existing, non-retired term. Shared by
/// every reader that judges a single terms line this way: `actions::do_answer`
/// (refuses on the first problem it finds), and `check` (reports every one,
/// for both a section's terms line and a term's own).
pub fn validate_pairs(line: &str, tf: &TermsFile) -> Result<Vec<Problem>, Malformed> {
    let pairs = parse_terms_line(line).ok_or(Malformed)?;
    let mut problems = Vec::new();
    for (word, uuid) in pairs {
        match tf
            .terms
            .iter()
            .find(|t| t.id.as_deref() == Some(uuid.as_str()))
        {
            None => problems.push(Problem {
                uuid,
                word,
                kind: ProblemKind::Gone,
            }),
            Some(t) if t.retired => problems.push(Problem {
                uuid,
                word,
                kind: ProblemKind::Retired(t.name.clone()),
            }),
            _ => {}
        }
    }
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terms::parse;
    use crate::terms::tests::{lines_of, well_formed};

    // the terms list, like
    // depends and supersedes, is one line of comma-separated entries: two
    // pairs round-trip through one `terms:` line, not two.
    #[test]
    fn format_and_parse_terms_line_round_trip() {
        let pairs = vec![
            (
                "decision".to_string(),
                "11111111-1111-1111-1111-111111111111".to_string(),
            ),
            (
                "question".to_string(),
                "22222222-2222-2222-2222-222222222222".to_string(),
            ),
        ];
        let line = format_terms_line(&pairs).unwrap();
        assert_eq!(
            line,
            "terms: decision=11111111-1111-1111-1111-111111111111, question=22222222-2222-2222-2222-222222222222"
        );
        let parsed = parse_terms_line(&line).unwrap();
        assert_eq!(parsed, pairs);
    }

    #[test]
    fn format_terms_line_is_none_when_empty() {
        assert!(format_terms_line(&[]).is_none());
    }

    #[test]
    fn parse_terms_line_rejects_non_terms_lines() {
        assert!(parse_terms_line("id: abc").is_none());
    }

    #[test]
    fn validate_pairs_reports_malformed_on_a_line_that_does_not_parse() {
        let tf = parse(&lines_of(&well_formed()));
        assert!(validate_pairs("id: abc", &tf).is_err());
        assert!(validate_pairs("terms: ", &tf).is_err());
    }

    #[test]
    fn validate_pairs_reports_a_gone_uuid() {
        let tf = parse(&lines_of(&well_formed()));
        let problems =
            validate_pairs("terms: decision=99999999-9999-9999-9999-999999999999", &tf).unwrap();
        assert_eq!(problems.len(), 1);
        assert!(matches!(problems[0].kind, ProblemKind::Gone));
        assert_eq!(problems[0].uuid, "99999999-9999-9999-9999-999999999999");
    }

    #[test]
    fn validate_pairs_reports_a_retired_term() {
        let mut s = well_formed();
        s = s.replacen("aliases: choice\n", "aliases: choice\nretired: yes\n", 1);
        let tf = parse(&lines_of(&s));
        let problems =
            validate_pairs("terms: decision=11111111-1111-1111-1111-111111111111", &tf).unwrap();
        assert_eq!(problems.len(), 1);
        match &problems[0].kind {
            ProblemKind::Retired(name) => assert_eq!(name, "decision"),
            ProblemKind::Gone => panic!("expected Retired"),
        }
    }

    #[test]
    fn validate_pairs_is_empty_when_every_pair_still_resolves() {
        let tf = parse(&lines_of(&well_formed()));
        let problems =
            validate_pairs("terms: decision=11111111-1111-1111-1111-111111111111", &tf).unwrap();
        assert!(problems.is_empty());
    }
}
