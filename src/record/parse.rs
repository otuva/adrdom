//! Reading a topic file line by line: headings, options, detail lines, and the
//! sections and shape errors a file yields.

use super::body::{is_skeleton_range, scan_body};
use super::*;

// --- headings, options and sections ---

fn is_question_heading(line: &str) -> bool {
    line.starts_with("### ") && line.trim_end().ends_with('?')
}

/// The indices of the `### ` lines that do not end in `?`: a heading is a
/// question (per "What does a topic file hold?"), so such a line is
/// refused rather than read as prose.
pub fn non_question_heading_lines(lines: &[String]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("### ") && !is_question_heading(l))
        .map(|(i, _)| i)
        .collect()
}

fn is_h1(line: &str) -> bool {
    line.starts_with("# ")
}

/// A `## ` line: no longer a valid tier heading (tiers are
/// derived from `depends:` lines), but still recognized so it can be
/// reported as a shape error instead of silently becoming stray text or
/// (worse) being mistaken for something else.
fn is_h2(line: &str) -> bool {
    line.starts_with("## ")
}

pub(crate) fn parse_option_line(line: &str) -> Option<(char, String)> {
    if !line.starts_with("- ") {
        return None;
    }
    let after_dash = &line[2..];
    let mut chars = after_dash.chars();
    let letter = chars.next()?;
    if !letter.is_ascii_uppercase() {
        return None;
    }
    let rest1 = chars.as_str();
    let rest = rest1.strip_prefix(") ")?;
    Some((letter, rest.to_string()))
}

pub(super) fn extract_sentence(rest: &str) -> Option<String> {
    let chars: Vec<char> = rest.chars().collect();
    for (idx, c) in chars.iter().enumerate() {
        if *c == '.' || *c == '!' || *c == '?' {
            let next = chars.get(idx + 1);
            if next.is_none() || *next.unwrap() == ' ' {
                let sentence: String = chars[..=idx].iter().collect();
                return Some(sentence);
            }
        }
    }
    None
}

/// Which of the two option-detail lines a body line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Detail {
    Why,
    Costs,
}

/// Recognizes a `  Why: ` or `  Costs: ` line and returns which kind it is
/// plus the text after the prefix. The one place both `scan_body` (which
/// counts these under each option) and `option_details` (which extracts
/// their text) recognize the prefix, so the two stay in sync.
pub(crate) fn detail_line(line: &str) -> Option<(Detail, &str)> {
    if let Some(v) = line.strip_prefix("  Why: ") {
        return Some((Detail::Why, v));
    }
    if let Some(v) = line.strip_prefix("  Costs: ") {
        return Some((Detail::Costs, v));
    }
    None
}

/// `"key: value"`, trimmed: `None` when `line` doesn't start with `key`
/// followed by a colon and a space. `terms::parse` keeps its own version of
/// this (`terms::kv`) rather than sharing it: a real `adrdom/terms.md`
/// field left empty is written with no space after the colon at all (a
/// bare `aliases:`), which this stricter form would treat as not naming
/// the field.
pub(super) fn kv(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{}: ", key);
    line.strip_prefix(&prefix).map(|v| v.trim().to_string())
}

pub(super) fn answer_value(line: &str) -> Option<String> {
    line.strip_prefix("**Answer:** ")
        .map(|v| v.trim().to_string())
}
/// `=A`, `=A|B`, `=*` after the `=`. `None` on anything else (bad set
/// syntax): an empty string, a part longer than one character, a
/// lowercase letter, or an empty part from `||` or a trailing `|`.
pub fn parse_sections(lines: &[String]) -> Vec<Section> {
    // Every heading's text, gathered once before any body is scanned: a
    // `depends:` line's comma-rejoin (see `parse_depends_line`) may need to
    // match a heading that comes later in the file, or the section's own.
    let headings: Vec<String> = lines
        .iter()
        .filter(|l| is_question_heading(l))
        .map(|l| l["### ".len()..].trim().to_string())
        .collect();

    let mut sections = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if is_question_heading(line) {
            let start = i;
            let end = find_end(lines, i + 1);
            sections.push(parse_question(lines, start, end, &headings));
            i = end;
        } else {
            i += 1;
        }
    }
    sections
}

fn find_end(lines: &[String], from: usize) -> usize {
    for (j, line) in lines.iter().enumerate().skip(from) {
        if is_question_heading(line) || is_h2(line) {
            return j;
        }
    }
    lines.len()
}

/// Everything a heading's body scan produces: the pre-existing bits
/// (id/supersedes/options/answer) plus the shape-rule bits (terms line,
/// depends entries, stray lines, Problem/Standard counts, and each
/// option's Why/Costs completeness).
fn parse_question(lines: &[String], start: usize, end: usize, headings: &[String]) -> Section {
    // Fully trimmed (not just `trim_end`): every consumer that compares or
    // hashes `heading_text` (a depends line's target text, an `assumes`
    // row's detail, `check`'s frozen-set comparison) must agree on the same
    // string regardless of extra whitespace right after `### `.
    let heading_text = lines[start]["### ".len()..].trim().to_string();
    let is_skeleton = is_skeleton_range(lines, start, end);
    let scan = scan_body(lines, start, end, headings);

    let mut parse_errors = scan.option_errors;
    if scan.id_count > 1 {
        parse_errors.push("duplicate id line".to_string());
    }
    // An identifier is a value the tool assigns (a uuid, per the term
    // "identifier"): an `id:` line holding anything else was written by
    // hand, and nothing reads it as the question's identifier.
    if let Some(id) = scan.id.as_deref().filter(|id| !is_uuid(id.trim())) {
        parse_errors.push(format!(
            "id line holds {:?}, which is no identifier the tool assigns",
            id
        ));
    }
    if let Some(v) = &scan.answer_value {
        let v = v.trim();
        let valid = v == "refused"
            || (v.chars().count() == 1
                && scan
                    .options
                    .iter()
                    .any(|o| o.letter == v.chars().next().unwrap()));
        if !valid {
            parse_errors.push(format!("answer names missing option {}", v));
        }
    }

    let mut shape_errors: Vec<String> = Vec::new();
    let mut add = |errs: Vec<String>| shape_errors.extend(errs);
    add(scan.stray_errors);
    add(scan.depends_errors);
    add(scan.reference_line_errors);
    add(scan.terms_errors);
    if !is_skeleton {
        // A missing field (count 0) and a repeated one (count > 1) are
        // different mistakes and get different messages: a zero count is
        // the field never being written, a count over one is the same
        // field written twice (178a232e A point 4).
        match scan.problem_count {
            0 => add(vec!["missing Problem line".to_string()]),
            1 => {}
            _ => add(vec!["duplicate Problem line".to_string()]),
        }
        match scan.standard_count {
            0 => add(vec!["missing Standard line".to_string()]),
            1 => {}
            _ => add(vec!["duplicate Standard line".to_string()]),
        }
        // A skeleton heading (empty body) is exempt; a real body needs two
        // or three options.
        if scan.options.len() < 2 {
            add(vec![format!(
                "{} options, at least two",
                scan.options.len()
            )]);
        }
    }
    add(scan.option_shape_errors);
    add(scan.answer_errors);
    // 178a232e A's "the answer line comes last": any non-blank line after
    // it (Problem/Standard/an option/anything else) means it isn't.
    if let Some(al) = scan.answer_line {
        if (al + 1..end).any(|j| !lines[j].trim().is_empty()) {
            add(vec![format!("answer line not last, line {}", al + 1)]);
        }
    }

    Section {
        start,
        end,
        heading_text,
        id: scan.id,
        supersedes: scan.supersedes,
        blame: scan.blame,
        depends: scan.depends,
        options: scan.options,
        answer_line: scan.answer_line,
        answer_value: scan.answer_value,
        parse_errors,
        shape_errors,
        is_skeleton,
        terms_line: scan.terms_line,
    }
}

/// Computes topic, context range, and file-level shape errors, given the
/// lines and the already heading-parsed `sections`.
pub(super) fn analyze_shape(
    lines: &[String],
    sections: &[Section],
) -> (Option<String>, (usize, usize), Vec<String>) {
    let mut shape_errors = Vec::new();

    let h1_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| is_h1(l))
        .map(|(i, _)| i)
        .collect();
    let first_non_blank = lines.iter().position(|l| !l.trim().is_empty());

    let topic;
    let h1_idx: Option<usize>;
    if h1_indices.is_empty() {
        shape_errors.push("missing h1".to_string());
        topic = None;
        h1_idx = None;
    } else {
        if h1_indices.len() > 1 {
            for &idx in &h1_indices[1..] {
                shape_errors.push(format!("duplicate h1, line {}", idx + 1));
            }
        }
        let first_h1 = h1_indices[0];
        if let Some(fnb) = first_non_blank {
            if fnb != first_h1 {
                shape_errors.push(format!("text before h1, line {}", fnb + 1));
            }
        }
        topic = Some(lines[first_h1]["# ".len()..].trim_end().to_string());
        h1_idx = Some(first_h1);
    }

    // Per "Does the tool derive tiers from the depends lines?": tiers are
    // derived from `depends:` lines, never written by
    // hand; a `## ` line left over from the old format is a shape error.
    // Per "What is the shape of a topic file?" (9913568f A): a non-blank line
    // after a `## ` line and before the next `### ` heading sits in no
    // question and is not prose, so it is reported as a stray line (a `### `
    // heading without `?` is reported once already, with its lines).
    let mut after_h2 = false;
    for (i, l) in lines.iter().enumerate() {
        if is_h2(l) {
            shape_errors.push(format!("tier heading, line {}", i + 1));
            after_h2 = true;
        } else if l.starts_with("### ") {
            after_h2 = false;
        } else if after_h2 && !l.trim().is_empty() && !is_h1(l) {
            shape_errors.push(format!("stray line {}", i + 1));
        }
    }

    let context_start = h1_idx.map(|i| i + 1).unwrap_or(0);
    let context_end = sections.first().map(|s| s.start).unwrap_or(lines.len());
    let context_range = (context_start.min(context_end), context_end);

    (topic, context_range, shape_errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::tests::{rf_from, SHAPED};

    // adr: 9913568f-6b85-4d26-946b-c6e108f9aa51
    // "What is the shape of a topic file?": one h1, first in the file.
    #[test]
    fn h1_required_and_first() {
        let rf = rf_from("no h1 here\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        assert!(rf.topic.is_none());
        assert!(rf.shape_errors.iter().any(|e| e.contains("missing h1")));

        let rf2 = rf_from("stray text\n# Topic\n");
        assert_eq!(rf2.topic.as_deref(), Some("Topic"));
        assert!(rf2
            .shape_errors
            .iter()
            .any(|e| e.contains("text before h1")));

        let rf3 = rf_from("# Topic\n\n# Second Topic\n");
        assert!(rf3.shape_errors.iter().any(|e| e.contains("duplicate h1")));

        let rf4 = rf_from(SHAPED);
        assert_eq!(rf4.topic.as_deref(), Some("Topic"));
        assert!(rf4.shape_errors.is_empty(), "{:?}", rf4.shape_errors);
    }

    // adr: 9913568f-6b85-4d26-946b-c6e108f9aa51
    // "What is the shape of a topic file?": no `## ` tier heading.
    #[test]
    fn a_h2_line_is_a_tier_heading_shape_error() {
        let rf = rf_from("# Topic\n\n## Old Group\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        assert!(
            rf.shape_errors
                .iter()
                .any(|e| e.starts_with("tier heading, line 3")),
            "{:?}",
            rf.shape_errors
        );
    }

    #[test]
    fn skeleton_heading_has_no_id_and_is_skeleton() {
        let rf = rf_from("# Topic\n\n### 1.1 Skeleton?\n\n### 1.2 Real question?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        let skeleton = &rf.sections[0];
        assert!(skeleton.is_skeleton);
        assert!(skeleton.id.is_none());
        assert!(
            skeleton.shape_errors.is_empty(),
            "{:?}",
            skeleton.shape_errors
        );

        let real = &rf.sections[1];
        assert!(!real.is_skeleton);
    }

    #[test]
    fn skeleton_with_only_depends_lines_is_skeleton() {
        let rf = rf_from("# Topic\n\n### 1.1 Skeleton?\ndepends: widget, 1.2 Other?\n\n### 1.2 Other?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        let sk = &rf.sections[0];
        assert!(sk.is_skeleton);
        assert!(sk.id.is_none());
        assert_eq!(sk.depends.len(), 2);
        assert!(sk.shape_errors.is_empty(), "{:?}", sk.shape_errors);
    }

    #[test]
    fn depends_typo_missing_the_space_is_not_a_skeleton_line() {
        // `kv` only recognizes "depends: " (with the space); a
        // heading whose only body line is the typo "depends:widget" (no
        // space) must not be silently accepted as a skeleton -- it falls
        // through `scan_body` and is reported as a stray line instead, so
        // the owner sees the typo rather than the line vanishing.
        let content = "# Topic\n\n### 1.1 Q?\ndepends:widget\n";
        let rf = rf_from(content);
        let sec = &rf.sections[0];
        assert!(!sec.is_skeleton, "{:?}", sec);
        assert!(
            sec.shape_errors.iter().any(|e| e == "stray line 4"),
            "{:?}",
            sec.shape_errors
        );
    }

    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    // "Does a reference line hold its list on one line?": the supersedes list, like
    // depends, holds several entries as one line of comma-separated values,
    // not one entry per line.
    #[test]
    fn supersedes_line_parses_multiple_comma_separated_ids() {
        let rf = rf_from("# Topic\n\n### 1.1 Q?\n\nid: new-id\n\nsupersedes: old-1, old-2\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        assert_eq!(
            rf.sections[0].supersedes,
            vec!["old-1".to_string(), "old-2".to_string()]
        );
    }

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // "What stands under a question's heading?", the "each one sentence" clause, at the unit that decides
    // *which* sentence: text with a sentence terminator yields that first
    // sentence -- still the sentence recorded on the option regardless of
    // what follows it, since `chosen_sentence` should show the same text
    // whether or not the option is later refused for holding a second one
    // (that refusal is `scan_body`'s separate `terms::sentence_count` check,
    // covered by `an_option_with_more_than_one_sentence_is_a_parse_error`
    // below). Text with no terminator at all yields `None`, which
    // `scan_body` turns into "option X has no decision sentence".
    #[test]
    fn an_option_decides_exactly_its_first_sentence() {
        assert_eq!(
            extract_sentence("Ties live in code. Because tests move."),
            Some("Ties live in code.".to_string())
        );
        assert_eq!(extract_sentence("Ties live in code"), None);
    }

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    // A heading with a depends line and a blame line still has no body.
    #[test]
    fn a_heading_with_a_depends_and_a_blame_line_is_a_skeleton() {
        let rf = rf_from("# Topic\n\n### Q?\ndepends: widget\nblame: old-file 1-2\n\n### R?\nblame: old-file 3-4\ndepends: widget\nid: 11111111-1111-1111-1111-111111111111\n");
        for sec in &rf.sections {
            assert!(sec.is_skeleton, "{:?}", sec);
            assert!(sec.shape_errors.is_empty(), "{:?}", sec.shape_errors);
        }
        assert_eq!(rf.sections[1].blame.len(), 1);
        let typo = rf_from("# Topic\n\n### Q?\ndepends: widget\nblame:old-file 1-2\n");
        assert!(!typo.sections[0].is_skeleton);
        assert!(typo.sections[0].blame.is_empty());
    }

    #[test]
    fn skeleton_with_an_id_line_is_still_skeleton() {
        // Per "What does a reference in a topic file carry?": `approve` gives every heading an id,
        // skeletons included; a skeleton that has picked one up must not suddenly start requiring
        // Problem/Standard/options.
        let rf = rf_from(
            "# Topic\n\n### 1.1 Skeleton?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: widget\n\n### 1.2 Other?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sk = &rf.sections[0];
        assert!(sk.is_skeleton, "{:?}", sk);
        assert_eq!(
            sk.id.as_deref(),
            Some("11111111-1111-1111-1111-111111111111")
        );
        assert!(sk.shape_errors.is_empty(), "{:?}", sk.shape_errors);
    }
}
