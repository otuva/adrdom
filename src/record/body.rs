//! Scanning one question's body: its lines, options and detail lines, and the
//! shape errors they raise.

use super::depends::{dep_identity_eq, parse_depends_line};
use super::parse::{answer_value, extract_sentence, kv};
use super::*;

// --- question body ---

pub(super) struct BodyScan {
    pub(super) id: Option<String>,
    pub(super) id_count: usize,
    pub(super) supersedes: Vec<String>,
    pub(super) blame: Vec<BlameEntry>,
    pub(super) depends: Vec<Dep>,
    pub(super) depends_errors: Vec<String>,
    /// "supersedes: second supersedes line, line N" / "blame: second blame
    /// line, line N":
    /// a82db5b7 holds every reference list on one line, so a second line of
    /// any is refused, the same as a second depends line, rather than
    /// silently ignored.
    pub(super) reference_line_errors: Vec<String>,
    /// "terms: second terms line, line N": the tool writes that line.
    pub(super) terms_errors: Vec<String>,
    pub(super) terms_line: Option<usize>,
    pub(super) options: Vec<OptionRec>,
    pub(super) answer_line: Option<usize>,
    pub(super) answer_value: Option<String>,
    pub(super) option_errors: Vec<String>,
    pub(super) stray_errors: Vec<String>,
    pub(super) problem_count: usize,
    pub(super) standard_count: usize,
    /// "option A missing Why" / "option B has two Costs lines", one entry
    /// per option that lacks exactly one non-empty Why/Costs line under it.
    pub(super) option_shape_errors: Vec<String>,
    /// "second answer line, line N" for each `**Answer:**` line past the
    /// first one in the section: `answer_line`/`answer_val` always keep the
    /// first occurrence, so a later one can never overwrite them and hide
    /// itself from the "answer line comes last" check.
    pub(super) answer_errors: Vec<String>,
}

pub(super) fn scan_body(
    lines: &[String],
    start: usize,
    end: usize,
    headings: &[String],
) -> BodyScan {
    let mut id = None;
    let mut id_count = 0;
    let mut supersedes: Vec<String> = Vec::new();
    let mut depends: Vec<Dep> = Vec::new();
    let mut depends_errors: Vec<String> = Vec::new();
    let mut depends_line_seen = false;
    let mut reference_line_errors: Vec<String> = Vec::new();
    let mut terms_errors: Vec<String> = Vec::new();
    let mut supersedes_line_seen = false;
    // An `unrelated:` line of an earlier record form (`tells::UNRELATED_PREFIX`):
    // left where it stands under an answered question, a stray line under any
    // other. Judged once the answer line is known.
    let mut unrelated_lines: Vec<usize> = Vec::new();
    let mut blame: Vec<BlameEntry> = Vec::new();
    let mut blame_line_seen = false;
    let mut terms_line = None;
    let mut options: Vec<OptionRec> = Vec::new();
    let mut answer_line = None;
    let mut answer_val = None;
    let mut option_errors = Vec::new();
    let mut stray_errors = Vec::new();
    let mut answer_errors = Vec::new();
    let mut problem_count = 0;
    let mut standard_count = 0;
    // Parallel to `options`: how many non-empty `  Why: `/`  Costs: ` lines
    // have been seen directly under each option so far.
    let mut why_counts: Vec<usize> = Vec::new();
    let mut costs_counts: Vec<usize> = Vec::new();
    // True while the previous non-blank line was an option or one of its
    // Why/Costs lines; a Why/Costs anywhere else is stray.
    let mut under_option = false;

    for (j, line) in lines.iter().enumerate().take(end).skip(start + 1) {
        if line.trim().is_empty() {
            continue;
        }
        // A `### ` line without `?` is refused once, by its own line
        // (`non_question_heading_lines`); it is not also a stray line.
        if line.starts_with("### ") {
            continue;
        }
        // A `depends:` with nothing after it names nothing: it counts as a
        // depends line with no entries, so the section is refused as having
        // no depends line (`record_refusals`) and nothing else about it.
        let empty_depends = is_bare_depends_line(line);
        if let Some(v) = kv(line, "depends").or_else(|| empty_depends.then(String::new)) {
            if v.is_empty() {
                depends_line_seen = true;
                continue;
            }
            if depends_line_seen {
                depends_errors.push(format!("depends: second depends line, line {}", j + 1));
                continue;
            }
            depends_line_seen = true;
            for res in parse_depends_line(&v, headings) {
                match res {
                    Ok(d) => {
                        if depends.iter().any(|existing| dep_identity_eq(existing, &d)) {
                            depends_errors
                                .push(format!("depends: duplicate entry, line {}", j + 1));
                        } else {
                            depends.push(d);
                        }
                    }
                    Err(why) => {
                        depends_errors.push(format!("depends: {}, line {}", why, j + 1));
                    }
                }
            }
            continue;
        }
        if let Some(v) = kv(line, "id") {
            if id_count == 0 {
                id = Some(v);
            }
            id_count += 1;
            continue;
        }
        if let Some(v) = kv(line, "supersedes") {
            if supersedes_line_seen {
                reference_line_errors.push(format!(
                    "supersedes: second supersedes line, line {}",
                    j + 1
                ));
                continue;
            }
            supersedes_line_seen = true;
            supersedes = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            continue;
        }
        if line.starts_with(crate::tells::UNRELATED_PREFIX) {
            unrelated_lines.push(j);
            continue;
        }
        if let Some(v) = kv(line, "blame") {
            if blame_line_seen {
                reference_line_errors.push(format!("blame: second blame line, line {}", j + 1));
                continue;
            }
            blame_line_seen = true;
            blame = v
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(BlameEntry::parse)
                .collect();
            continue;
        }
        if kv(line, "terms").is_some() {
            if terms_line.is_some() {
                terms_errors.push(format!("terms: second terms line, line {}", j + 1));
                continue;
            }
            terms_line = Some(j);
            continue;
        }
        if kv(line, "Problem").is_some() {
            problem_count += 1;
            continue;
        }
        if kv(line, "Standard").is_some() {
            standard_count += 1;
            continue;
        }
        if let Some((letter, rest)) = parse_option_line(line) {
            let sentence = extract_sentence(&rest);
            if sentence.is_none() {
                option_errors.push(format!("option {} has no decision sentence", letter));
            } else {
                // 178a232e A's "each one sentence": a decision text holding
                // a second sentence is refused too -- `extract_sentence`
                // above only ever looks at the first one, so a count has to
                // be taken separately to catch the rest of the text.
                let count = terms::sentence_count(rest.trim_end());
                if count > 1 {
                    option_errors.push(format!(
                        "option {} decision text has {} sentences, more than one",
                        letter, count
                    ));
                }
            }
            options.push(OptionRec {
                letter,
                sentence,
                line: j,
            });
            why_counts.push(0);
            costs_counts.push(0);
            under_option = true;
            continue;
        }
        if let Some((kind, v)) = detail_line(line) {
            if under_option {
                if !v.trim().is_empty() {
                    let counts = match kind {
                        Detail::Why => &mut why_counts,
                        Detail::Costs => &mut costs_counts,
                    };
                    if let Some(c) = counts.last_mut() {
                        *c += 1;
                    }
                }
                continue;
            }
            stray_errors.push(format!("stray line {}", j + 1));
            continue;
        }
        under_option = false;
        if let Some(v) = answer_value(line) {
            if answer_line.is_none() {
                answer_line = Some(j);
                answer_val = Some(v);
            } else {
                answer_errors.push(format!("second answer line, line {}", j + 1));
            }
            continue;
        }
        stray_errors.push(format!("stray line {}", j + 1));
    }

    if answer_line.is_none() {
        for j in unrelated_lines {
            stray_errors.push(format!("stray line {}", j + 1));
        }
        // In line order, as the loop above reports them.
        stray_errors.sort_by_key(|e| {
            e.rsplit(' ')
                .next()
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(0)
        });
    }

    // A fourth option is a shape error; the minimum (two, unless the
    // heading is a skeleton) is enforced by the caller, which alone knows
    // whether the body is a skeleton.
    let mut option_shape_errors = Vec::new();
    if options.len() > 3 {
        option_shape_errors.push(format!("{} options, at most three", options.len()));
    }

    // Every option carries exactly one non-empty Why line and exactly one
    // non-empty Costs line.
    for (i, opt) in options.iter().enumerate() {
        match why_counts[i] {
            0 => option_shape_errors.push(format!("option {} missing Why", opt.letter)),
            1 => {}
            _ => option_shape_errors.push(format!("option {} has two Why lines", opt.letter)),
        }
        match costs_counts[i] {
            0 => option_shape_errors.push(format!("option {} missing Costs", opt.letter)),
            1 => {}
            _ => option_shape_errors.push(format!("option {} has two Costs lines", opt.letter)),
        }
    }

    BodyScan {
        id,
        id_count,
        supersedes,
        blame,
        depends,
        depends_errors,
        reference_line_errors,
        terms_errors,
        terms_line,
        options,
        answer_line,
        answer_value: answer_val,
        option_errors,
        stray_errors,
        problem_count,
        standard_count,
        option_shape_errors,
        answer_errors,
    }
}

/// A heading whose body holds only blank lines, `depends:` lines, a
/// `supersedes:` line (part of the plan, per "Does approve turn the
/// supersedes line into identifiers?"), a `blame:` line (per "What does a
/// question carry of the blame file it came from?"), and an `id:` line (per
/// "What stops a question that has no depends line?" and "What does a
/// reference in a topic file carry?"): needs no id of its own to stay a
/// skeleton, but `approve` gives every heading an id, skeletons included, so
/// a skeleton must stay a skeleton once it has one. A `depends:` line with
/// nothing after it, indented or not.
fn is_bare_depends_line(line: &str) -> bool {
    line.trim() == "depends:"
}

pub(super) fn is_skeleton_range(lines: &[String], start: usize, end: usize) -> bool {
    ((start + 1)..end).all(|j| {
        let t = lines[j].trim();
        // Same prefix `kv` requires: "depends:"/"id:" with no
        // space after the colon is a typo, not a real field, so it must not
        // be silently swallowed as skeleton -- it falls through and
        // `scan_body` reports it as a stray line instead.
        t.is_empty()
            || t.starts_with("depends: ")
            || is_bare_depends_line(t)
            || t.starts_with("supersedes: ")
            || t.starts_with("blame: ")
            || t.starts_with("id: ")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::tests::{rf_from, SHAPED};

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // "What stands under a question's heading?": no stray line in the body.
    #[test]
    fn stray_line_in_body_is_a_shape_error_naming_the_line() {
        let content = "# Topic\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\nthis line is not allowed\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let rf = rf_from(content);
        let sec = &rf.sections[0];
        let stray_line_no = content
            .lines()
            .position(|l| l == "this line is not allowed")
            .unwrap()
            + 1;
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == &format!("stray line {}", stray_line_no)),
            "{:?}",
            sec.shape_errors
        );
    }

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // The body under a question's heading ("What stands under a question's heading?"):
    // a Problem/Standard presence clause, plus (via `SHAPED`
    // below) a well-formed body -- one-line Problem, one-line Standard,
    // two options each with a Why and a Costs line -- parsing with zero
    // shape errors.
    #[test]
    fn missing_problem_or_standard_only_fails_non_skeleton_questions() {
        let rf = rf_from("# Topic\n\n### 1.1 Skeleton?\n\n### 1.2 Has a body but no Problem/Standard?\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        let skeleton = &rf.sections[0];
        assert!(skeleton.shape_errors.is_empty());

        let non_skeleton = &rf.sections[1];
        assert!(non_skeleton
            .shape_errors
            .iter()
            .any(|e| e == "missing Problem line"));
        assert!(non_skeleton
            .shape_errors
            .iter()
            .any(|e| e == "missing Standard line"));

        let rf2 = rf_from(SHAPED);
        assert!(
            rf2.sections[0].shape_errors.is_empty(),
            "{:?}",
            rf2.sections[0].shape_errors
        );
    }

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // "What stands under a question's heading?", the "each one sentence" clause, the half
    // `an_option_decides_exactly_its_first_sentence` above does not cover:
    // a decision text holding a second sentence is a parse error (a
    // `terms::sentence_count` > 1 on the option's own text), not silently
    // accepted just because the first sentence parsed fine.
    #[test]
    fn an_option_with_more_than_one_sentence_is_a_parse_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One. And a second sentence.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(
            rf.sections[0]
                .parse_errors
                .iter()
                .any(|e| e == "option A decision text has 2 sentences, more than one"),
            "{:?}",
            rf.sections[0].parse_errors
        );

        let ok = rf_from(SHAPED);
        assert!(
            ok.sections[0].parse_errors.is_empty(),
            "{:?}",
            ok.sections[0].parse_errors
        );
    }

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // "What stands under a question's heading?", a second Problem:/Standard: line: a
    // repeated field is reported as a duplicate, not folded into the same
    // "missing" message a zero count gets.
    #[test]
    fn a_repeated_problem_or_standard_line_is_a_duplicate_not_a_missing_one() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\n\nProblem: p again.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(
            rf.sections[0]
                .shape_errors
                .iter()
                .any(|e| e == "duplicate Problem line"),
            "{:?}",
            rf.sections[0].shape_errors
        );
        assert!(
            !rf.sections[0]
                .shape_errors
                .iter()
                .any(|e| e == "missing Problem line"),
            "{:?}",
            rf.sections[0].shape_errors
        );

        let rf2 = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\n\nStandard: s.\n\nStandard: s again.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(
            rf2.sections[0]
                .shape_errors
                .iter()
                .any(|e| e == "duplicate Standard line"),
            "{:?}",
            rf2.sections[0].shape_errors
        );
    }

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // "What stands under a question's heading?", the answer line comes last: an
    // `**Answer:**` line with real body content after it is refused, at
    // the section-parsing level `approve`/`check` both read.
    #[test]
    fn an_answer_line_before_the_rest_of_the_body_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\n**Answer:** A\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(
            rf.sections[0]
                .shape_errors
                .iter()
                .any(|e| e.starts_with("answer line not last")),
            "{:?}",
            rf.sections[0].shape_errors
        );

        // At the true end (nothing but blank lines after it): accepted.
        let ok = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        );
        assert!(
            !ok.sections[0]
                .shape_errors
                .iter()
                .any(|e| e.starts_with("answer line not last")),
            "{:?}",
            ok.sections[0].shape_errors
        );
    }

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // "What stands under a question's heading?", the answer line comes last: a second
    // `**Answer:**` line is its own shape error, not silently swallowed by
    // `answer_line` being overwritten to point at the later (last) one --
    // which used to let an out-of-place early answer line escape the
    // "answer line not last" check entirely.
    #[test]
    fn a_second_answer_line_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n\n**Answer:** B\n",
        );
        assert!(
            rf.sections[0]
                .shape_errors
                .iter()
                .any(|e| e.starts_with("second answer line")),
            "{:?}",
            rf.sections[0].shape_errors
        );
    }

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // "What stands under a question's heading?": no Lean line in the body.
    #[test]
    fn a_lean_line_inside_a_question_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\nLean: A.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e.starts_with("stray line")));
    }

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // "What stands under a question's heading?": a Why line belongs under an option.
    #[test]
    fn a_why_line_before_any_option_is_stray() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n  Why: early.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e == "stray line 7"));
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n  Costs: late.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e.starts_with("stray line")));
    }

    // adr: 178a232e-b51c-4122-a1a9-9e9ed6970748
    // "What stands under a question's heading?", two or three lettered options: four is
    // refused, three is accepted (the `ok` case below).
    #[test]
    fn a_fourth_option_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n- D) Four.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e == "4 options, at most three"));
        let ok = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(ok.sections[0].shape_errors.is_empty());
    }

    #[test]
    fn one_option_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) Only choice.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e == "1 options, at least two"));
        assert!(
            rf.sections[0].parse_errors.is_empty(),
            "{:?}",
            rf.sections[0].parse_errors
        );
    }

    #[test]
    fn answer_refused_is_recognized() {
        let content = "# Topic\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** refused\n";
        let rf = rf_from(content);
        let sec = &rf.sections[0];
        assert!(sec.is_refused());
        assert_eq!(sec.answer_letter(), None);
        assert!(sec.parse_errors.is_empty(), "{:?}", sec.parse_errors);
    }

    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    // "Does a reference line hold its list on one line?": a depends list split
    // across two lines is refused (a shape error), not read as more
    // entries of the same list.
    #[test]
    fn two_depends_lines_under_one_heading_is_a_shape_error() {
        let rf = rf_from(
            "# Topic\n\n### 1.1 Q?\ndepends: widget\ndepends: gadget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[0];
        // Only the first line's entry is parsed; the second line is refused
        // outright rather than treated as more entries.
        assert_eq!(
            sec.depends,
            vec![Dep::Text {
                text: "widget".to_string(),
                set: None
            }]
        );
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "depends: second depends line, line 5"),
            "{:?}",
            sec.shape_errors
        );
    }

    #[test]
    fn bare_depends_predicate_ignores_indent() {
        assert!(is_bare_depends_line("depends:"));
        assert!(is_bare_depends_line("  depends:  "));
        assert!(!is_bare_depends_line("depends: x"));
        assert!(!is_bare_depends_line("depends:x"));
    }

    #[test]
    fn indented_bare_depends_is_the_same_as_bare_and_a_later_real_one_is_second() {
        for bare in ["depends:", "  depends:"] {
            let rf = rf_from(&format!("# T\n\n### Q?\n{bare}\ndepends: widget\n"));
            let sec = &rf.sections[0];
            assert!(
                sec.shape_errors
                    .iter()
                    .any(|e| e.starts_with("depends: second depends line")),
                "{bare:?}: {:?}",
                sec.shape_errors
            );
            assert!(
                !sec.shape_errors.iter().any(|e| e.contains("stray")),
                "{bare:?}: {:?}",
                sec.shape_errors
            );
        }
    }

    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    // "Does a reference line hold its list on one line?": a supersedes list split
    // across two lines is refused (a shape error), not silently dropped.
    #[test]
    fn two_supersedes_lines_under_one_heading_is_a_shape_error() {
        let rf = rf_from(
            "# Topic\n\n### 1.1 Q?\ndepends: widget\nsupersedes: aaa, bbb\nsupersedes: ccc\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[0];
        // Only the first line's entries are read.
        assert_eq!(sec.supersedes, vec!["aaa".to_string(), "bbb".to_string()]);
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "supersedes: second supersedes line, line 6"),
            "{:?}",
            sec.shape_errors
        );
    }

    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    // "Does a reference line hold its list on one line?": a terms list split
    // across two lines is refused (a shape error), not silently dropped.
    #[test]
    fn two_terms_lines_under_one_heading_is_a_shape_error() {
        let rf = rf_from(
            "# Topic\n\n### 1.1 Q?\ndepends: widget\nterms: widget=aaa\nterms: gadget=bbb\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[0];
        // The first terms line stays the one the section records.
        assert_eq!(sec.terms_line, Some(4));
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "terms: second terms line, line 6"),
            "{:?}",
            sec.shape_errors
        );
    }
}
