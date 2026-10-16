//! The hashes of questions, files and sections, and the short version of a
//! section.

use super::*;

// --- hashes ---

/// sha256 of one question's own text (per "Does approve hash each question
/// on its own?"): its heading line, its `depends:` line with the answer sets
/// as written, its `supersedes:` line, an `unrelated:` line, a `blame:` line,
/// and its body (Problem, Standard, options, Why, Costs), each line trimmed
/// at its end, joined with `\n`. The `id:` line is part of it (swapping ids
/// would re-point every depends entry naming them); the lines the tool writes
/// after that (`terms:`, the `**Answer:**` line) and blank lines are left
/// out, so asking or answering a question never moves its hash (approve has
/// already written the id). Read on the text as it stands, so call it on the
/// file after approve turned names into ids.
pub fn question_hash(rf: &RecordFile, sec: &Section) -> String {
    let (s, e) = hash_range(&rf.lines, sec);
    let text = rf.lines[s..e]
        .iter()
        .map(|l| l.trim_end())
        .filter(|l| {
            !l.trim().is_empty() && !l.starts_with("terms: ") && !l.starts_with("**Answer:**")
        })
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&text)
}

/// Every question's `question_hash`, in file order.
pub fn question_hashes(rf: &RecordFile) -> Vec<String> {
    rf.sections.iter().map(|s| question_hash(rf, s)).collect()
}

/// sha256 of a question's heading text alone: what the planning steps key
/// the neighbours of a heading by ("What does the agent do to plan a
/// topic file?").
pub fn heading_hash(sec: &Section) -> String {
    sha256_hex(&sec.heading_text)
}

/// A question's heading and its body, as the reader is given them: the
/// heading text, then the Problem, Standard, options, Why and Costs lines,
/// each trimmed at its end, joined with `\n`. The `depends:`,
/// `supersedes:`, `unrelated:`, `blame:`, `id:` and `terms:` lines, the
/// `**Answer:**` line and blank lines are left out, so a line the agent
/// writes after the reader ran never moves it.
pub fn body_text(rf: &RecordFile, sec: &Section) -> String {
    let (s, e) = hash_range(&rf.lines, sec);
    let mut parts: Vec<&str> = vec![sec.heading_text.as_str()];
    for l in &rf.lines[(s + 1).min(e)..e] {
        let t = l.trim_end();
        if t.trim().is_empty()
            || t.starts_with("depends: ")
            || t.trim() == "depends:"
            || t.starts_with("supersedes: ")
            || t.starts_with("unrelated: ")
            || t.starts_with("blame: ")
            || t.starts_with("id: ")
            || t.starts_with("terms: ")
            || t.starts_with("**Answer:**")
        {
            continue;
        }
        parts.push(t);
    }
    parts.join("\n")
}

/// sha256 of `body_text`.
pub fn body_hash(rf: &RecordFile, sec: &Section) -> String {
    sha256_hex(&body_text(rf, sec))
}

/// The file's hash derived from its parts (per "Does approve hash each
/// question on its own?"): sha256 of the question hashes sorted and joined
/// with `\n`. The order of the questions carries no meaning, so
/// reordering headings moves nothing.
pub fn derive_file_hash(question_hashes: &[String]) -> String {
    let mut sorted: Vec<&str> = question_hashes.iter().map(|h| h.as_str()).collect();
    sorted.sort_unstable();
    sha256_hex(&sorted.join("\n"))
}

/// The file's derived hash over its current questions.
pub fn plan_hash(rf: &RecordFile) -> String {
    derive_file_hash(&question_hashes(rf))
}

/// sha256 of the context prose lines with leading/trailing blank lines
/// trimmed, joined with `\n`; the empty string hashes when there is no
/// prose.
pub fn context_hash(rf: &RecordFile) -> String {
    let (s, e) = rf.context_range;
    let e = e.min(rf.lines.len());
    let s = s.min(e);
    let slice = &rf.lines[s..e];

    let mut from = 0;
    while from < slice.len() && slice[from].trim().is_empty() {
        from += 1;
    }
    let mut to = slice.len();
    while to > from && slice[to - 1].trim().is_empty() {
        to -= 1;
    }
    let text = slice[from..to]
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&text)
}

/// Returns [start, hash_end) — the line range whose joined bytes are hashed.
pub fn hash_range(lines: &[String], sec: &Section) -> (usize, usize) {
    // The boundary to search backward from for the last non-blank line:
    // just before the Answer line when there is one, otherwise the end of
    // the section. Trailing blank lines are never hashed, so inserting the
    // "blank line before the Answer line" (as `adr answer` does) can never
    // change a section's hash out from under itself.
    let boundary = sec.answer_line.unwrap_or(sec.end);
    let mut last = sec.start;
    for i in (sec.start..boundary).rev() {
        if !lines[i].trim().is_empty() {
            last = i;
            break;
        }
    }
    (sec.start, last + 1)
}

/// The set on a `depends:` entry, canonicalized for hashing and printing:
/// `*` for `Any`, or its letters sorted and joined with `|` for `Letters`
/// (so `=B|A` and `=A|B` hash and print the same). The one place both
/// `ask` (writing an `assumes` row) and `check` (recomputing it to compare)
/// turn a set into text.
pub fn canonical_set(set: &AnswerSet) -> String {
    match set {
        AnswerSet::Any => "*".to_string(),
        AnswerSet::Letters(letters) => {
            let mut sorted = letters.clone();
            sorted.sort_unstable();
            sorted
                .into_iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join("|")
        }
    }
}

/// The `=<set>` suffix a `depends:` entry prints after its id/text: `=*`,
/// `=A`, `=A|B` (`canonical_set`, prefixed with `=`), or an empty string
/// for `None`. Shared by every place that rewrites or prints a depends
/// entry's answer set back into text.
pub fn set_suffix(set: &Option<AnswerSet>) -> String {
    set.as_ref()
        .map(|s| format!("={}", canonical_set(s)))
        .unwrap_or_default()
}

/// sha256 of `<dependent key>=<canonical set>` (per "Does a depends entry
/// carry the answers it holds under?", updated by "What does a reference in a topic file carry?"):
/// an `assumes` row's hash, and what `check`'s frozen-set comparison recomputes against the depends
/// line as currently written. `dependent_key` is the dependent section's own heading id (`assumes`
/// rows key on heading ids, not text).
pub fn assumes_hash(dependent_key: &str, set: &AnswerSet) -> String {
    sha256_hex(&format!("{}={}", dependent_key, canonical_set(set)))
}

pub fn section_hash(lines: &[String], sec: &Section) -> String {
    let (s, e) = hash_range(lines, sec);
    let text = lines[s..e]
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&text)
}

/// How many hex characters of `section_hash` the tool prints and checks as
/// a question's "version" (per "Does an answer carry the version of the
/// question the owner read?"): short enough to type on the `adr answer
/// <id>@<version>` command, long enough that two different texts for the
/// same question never collide by chance.
pub const VERSION_LEN: usize = 8;

/// The first `VERSION_LEN` hex characters of `section_hash(lines, sec)`:
/// what `show` and the page print next to an asked question, and what
/// `answer`/the page's POST handler check the owner's `<id>@<version>`
/// against.
pub fn version_of(lines: &[String], sec: &Section) -> String {
    section_hash(lines, sec)[..VERSION_LEN].to_string()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex(&hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::tests::{rf_from, SHAPED};

    #[test]
    fn context_hash_stable_when_a_question_body_changes() {
        let rf1 = rf_from(SHAPED);
        let h1 = context_hash(&rf1);

        let edited = SHAPED.replace("Problem: p.", "Problem: a totally different problem.");
        let rf2 = rf_from(&edited);
        let h2 = context_hash(&rf2);

        assert_eq!(
            h1, h2,
            "context hash must not move when only a body changes"
        );
        assert_ne!(
            h1,
            sha256_hex(""),
            "sanity: this record does have context prose"
        );
    }

    #[test]
    fn plan_hash_changes_on_heading_add_rename_and_body_edit_not_on_reorder() {
        let base = rf_from(SHAPED);
        let base_hash = plan_hash(&base);

        // Body edit: moves the hash (the body is part of the question).
        let body_edited = SHAPED.replace("Problem: p.", "Problem: a very different problem.");
        assert_ne!(
            base_hash,
            plan_hash(&rf_from(&body_edited)),
            "a body edit must move the hash"
        );

        // Heading added.
        let added = format!("{}\n### 1.2 A second question?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) y.\n  Why: w.\n  Costs: c.\n- B) z.\n  Why: w.\n  Costs: c.\n", SHAPED);
        assert_ne!(
            base_hash,
            plan_hash(&rf_from(&added)),
            "adding a heading must move the hash"
        );

        // Heading renamed.
        let renamed = SHAPED.replace(
            "### 1.1 First question?",
            "### 1.1 First question, renamed?",
        );
        assert_ne!(
            base_hash,
            plan_hash(&rf_from(&renamed)),
            "renaming a heading must move the hash"
        );

        // Headings reordered: two questions, swapped.
        let two_questions = "# Topic\n\n### 1.1 Question A?\n\nProblem: pa.\n\nStandard: sa.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) x2.\n  Why: w.\n  Costs: c.\n\n### 1.2 Question B?\n\nProblem: pb.\n\nStandard: sb.\n\n- A) y.\n  Why: w.\n  Costs: c.\n- B) y2.\n  Why: w.\n  Costs: c.\n";
        let swapped = "# Topic\n\n### 1.2 Question B?\n\nProblem: pb.\n\nStandard: sb.\n\n- A) y.\n  Why: w.\n  Costs: c.\n- B) y2.\n  Why: w.\n  Costs: c.\n\n### 1.1 Question A?\n\nProblem: pa.\n\nStandard: sa.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) x2.\n  Why: w.\n  Costs: c.\n";
        assert_eq!(
            plan_hash(&rf_from(two_questions)),
            plan_hash(&rf_from(swapped)),
            "reordering headings must not move the derived hash"
        );
    }

    #[test]
    fn question_hash_covers_depends_with_sets_and_skips_tool_lines() {
        let make = |set_suffix: &str| {
            format!(
                "# Topic\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n### 1.2 R?\ndepends: 1.1 Q?{}\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
                set_suffix
            )
        };
        let with_a = rf_from(&make("=A"));
        let with_b = rf_from(&make("=B"));
        assert_ne!(
            question_hashes(&with_a)[1],
            question_hashes(&with_b)[1],
            "the answer set on a depends entry is part of the question hash"
        );
        assert_eq!(question_hashes(&with_a)[0], question_hashes(&with_b)[0]);

        let base = "# Topic\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let tooled = "# Topic\n\n### 1.1 Q?\nterms: a=b\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n";
        assert_eq!(
            question_hashes(&rf_from(base)),
            question_hashes(&rf_from(tooled)),
            "terms and Answer lines are not part of the question hash"
        );
    }

    #[test]
    fn section_hash_ignores_trailing_whitespace_on_a_line() {
        let clean = rf_from(SHAPED);
        let clean_hash = section_hash(&clean.lines, &clean.sections[0]);

        let trailing = SHAPED.replace("- A) Do the thing.", "- A) Do the thing.  ");
        let dirty = rf_from(&trailing);
        let dirty_hash = section_hash(&dirty.lines, &dirty.sections[0]);

        assert_eq!(
            clean_hash, dirty_hash,
            "trailing whitespace on an option line must not move the section hash"
        );
    }

    #[test]
    fn unrelated_line_parses_is_in_the_question_hash_and_not_in_the_body_hash() {
        let text = "# Topic\n\n### Q?\ndepends: widget\nunrelated: a, b\nid: new-id\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let rf = rf_from(text);
        assert_eq!(
            rf.sections[0].unrelated,
            vec!["a".to_string(), "b".to_string()]
        );
        assert!(rf.shape_errors.is_empty() && rf.sections[0].shape_errors.is_empty());
        let other = rf_from(&text.replace("unrelated: a, b", "unrelated: a, c"));
        assert_ne!(
            question_hash(&rf, &rf.sections[0]),
            question_hash(&other, &other.sections[0])
        );
        assert_eq!(
            body_hash(&rf, &rf.sections[0]),
            body_hash(&other, &other.sections[0])
        );
        assert_eq!(heading_hash(&rf.sections[0]), sha256_hex("Q?"));
        assert!(
            body_text(&rf, &rf.sections[0]).starts_with("Q?\nProblem: p.\nStandard: s.\n- A) x.")
        );
        let twice = rf_from(&text.replace("id: new-id", "unrelated: c\nid: new-id"));
        assert!(twice.sections[0]
            .shape_errors
            .iter()
            .any(|e| e.contains("second unrelated line")));
    }

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    // The line reads in file order, comma separated; the question hash covers
    // it like the depends line, the body hash and the reader never see it.
    #[test]
    fn blame_line_parses_is_in_the_question_hash_and_not_in_the_body_hash() {
        let text = "# Topic\n\n### Q?\ndepends: widget\nblame: old-file 10-14 31-35, sub/other 3-9\nid: new-id\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let rf = rf_from(text);
        let sec = &rf.sections[0];
        assert_eq!(
            sec.blame,
            vec![
                BlameEntry {
                    file: "old-file".to_string(),
                    ranges: vec![(10, 14), (31, 35)]
                },
                BlameEntry {
                    file: "sub/other".to_string(),
                    ranges: vec![(3, 9)]
                },
            ]
        );
        assert!(
            rf.shape_errors.is_empty() && sec.shape_errors.is_empty(),
            "{:?}",
            sec.shape_errors
        );
        let other = rf_from(&text.replace("sub/other 3-9", "sub/other 3-8"));
        assert_ne!(
            question_hash(&rf, sec),
            question_hash(&other, &other.sections[0])
        );
        let without = rf_from(&text.replace("blame: old-file 10-14 31-35, sub/other 3-9\n", ""));
        assert_ne!(
            question_hash(&rf, sec),
            question_hash(&without, &without.sections[0])
        );
        assert_eq!(
            body_hash(&rf, sec),
            body_hash(&without, &without.sections[0])
        );
        assert_eq!(
            body_text(&rf, sec),
            body_text(&without, &without.sections[0])
        );
        assert!(!body_text(&rf, sec).contains("blame"));
        let twice = rf_from(&text.replace("id: new-id", "blame: other 1-1\nid: new-id"));
        assert_eq!(
            twice.sections[0].blame, sec.blame,
            "the first line is the one read"
        );
        assert!(
            twice.sections[0]
                .shape_errors
                .iter()
                .any(|e| e == "blame: second blame line, line 6"),
            "{:?}",
            twice.sections[0].shape_errors
        );
    }

    #[test]
    fn question_hash_changes_when_an_id_line_changes() {
        let base = "# Topic\n\n### 1.1 Q?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let with_id = rf_from(base);
        let no_id = rf_from(&base.replace("id: 11111111-1111-1111-1111-111111111111\n\n", ""));
        assert_ne!(plan_hash(&with_id), plan_hash(&no_id));
        let swapped = base.replace(
            "11111111-1111-1111-1111-111111111111",
            "22222222-2222-2222-2222-222222222222",
        );
        assert_ne!(
            plan_hash(&with_id),
            plan_hash(&rf_from(&swapped)),
            "swapping an id must move the hash"
        );
    }
}
