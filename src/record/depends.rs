//! Parsing the `=A|B` answer sets and the entries of a `depends:` line.

use super::*;

// --- answer sets and depends entries ---

fn parse_answer_set(s: &str) -> Option<AnswerSet> {
    if s == "*" {
        return Some(AnswerSet::Any);
    }
    if s.is_empty() {
        return None;
    }
    let mut letters = Vec::new();
    for part in s.split('|') {
        let mut chars = part.chars();
        let c = chars.next()?;
        if chars.next().is_some() || !c.is_ascii_uppercase() {
            return None;
        }
        letters.push(c);
    }
    Some(AnswerSet::Letters(letters))
}

/// Splits a `depends:` line's whole value into its entries (per "Does a
/// reference line hold its whole list on one line?"): entries
/// are comma separated, an answer set stays on its own entry (`|` inside a
/// set, never `,`). Before approve, an entry may itself be heading text that
/// holds a comma: fragments are rejoined, longest run first, against
/// `headings` (every heading text in the file, gathered once up front by
/// the caller so a later heading can be named by an earlier one and vice
/// versa) -- `parse_depends_entry` on the rejoined text tells whether it is
/// heading-shaped (ends in `?`, once any `=set` tail is set aside) so this
/// never needs its own copy of that rule. A fragment run that matches no
/// heading falls back to one entry per raw fragment, same as a plain
/// comma-separated list with no embedded commas.
pub(super) fn parse_depends_line(value: &str, headings: &[String]) -> Vec<Result<Dep, String>> {
    let pieces: Vec<String> = value.split(',').map(|s| s.trim().to_string()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < pieces.len() {
        let mut chosen = i + 1;
        for j in (i + 2..=pieces.len()).rev() {
            let joined = pieces[i..j].join(", ");
            if let Ok(Dep::Text { text, .. }) = parse_depends_entry(&joined) {
                if headings.iter().any(|h| h.trim() == text.trim()) {
                    chosen = j;
                    break;
                }
            }
        }
        let joined = pieces[i..chosen].join(", ");
        out.push(parse_depends_entry(&joined));
        i = chosen;
    }
    out
}

/// Classifies one `depends:` entry (the trimmed text after `depends: `) into
/// `Id` (a uuid, with or without a trailing `=set`) or `Text` (words, with
/// or without a trailing `=set` on a heading-shaped base). `Err` names why
/// the entry is malformed, without the "depends: " prefix or line number
/// (the caller adds those).
fn parse_depends_entry(raw: &str) -> Result<Dep, String> {
    if raw.is_empty() {
        return Err("empty entry".to_string());
    }
    if let Some(eq_idx) = raw.rfind('=') {
        let base = raw[..eq_idx].trim_end();
        if is_uuid(base) || base.ends_with('?') {
            let set_str = &raw[eq_idx + 1..];
            return match parse_answer_set(set_str) {
                Some(set) => Ok(if is_uuid(base) {
                    Dep::Id {
                        id: base.to_string(),
                        set: Some(set),
                    }
                } else {
                    Dep::Text {
                        text: base.to_string(),
                        set: Some(set),
                    }
                }),
                None => Err("bad set syntax".to_string()),
            };
        }
    }
    if is_uuid(raw) {
        return Ok(Dep::Id {
            id: raw.to_string(),
            set: None,
        });
    }
    Ok(Dep::Text {
        text: raw.to_string(),
        set: None,
    })
}

/// Whether two successfully-parsed deps name the same target (ignoring any
/// answer set): used to catch the same entry repeated within one `depends:`
/// line, not a repeated `depends:` line (a second one is its own error).
pub(super) fn dep_identity_eq(a: &Dep, b: &Dep) -> bool {
    match (a, b) {
        (Dep::Id { id: x, .. }, Dep::Id { id: y, .. }) => x == y,
        (Dep::Text { text: x, .. }, Dep::Text { text: y, .. }) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::tests::rf_from;

    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    // "Does a reference line hold its list on one line?": a one-line, comma-
    // separated depends list with three entries parses cleanly (no shape
    // errors) and each entry keeps its own answer set independently: the
    // middle entry carries `=A`, the other two carry none.
    #[test]
    fn depends_parses_all_three_entry_forms_and_sets() {
        let rf = rf_from(
            "# Topic\n\n\
             ### 1.1 First question?\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.2 Second question?\n\
             depends: widget, 1.1 First question?=A, 11111111-1111-1111-1111-111111111111\n\n\
             Problem: p2.\n\n\
             Standard: s2.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[1];
        assert_eq!(
            sec.shape_errors,
            Vec::<String>::new(),
            "{:?}",
            sec.shape_errors
        );
        assert_eq!(sec.depends.len(), 3);
        assert_eq!(
            sec.depends[0],
            Dep::Text {
                text: "widget".to_string(),
                set: None
            }
        );
        assert_eq!(
            sec.depends[1],
            Dep::Text {
                text: "1.1 First question?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A']))
            }
        );
        assert_eq!(
            sec.depends[2],
            Dep::Id {
                id: "11111111-1111-1111-1111-111111111111".to_string(),
                set: None
            }
        );
    }

    #[test]
    fn depends_set_suffixes_parse_letters_and_star() {
        let rf = rf_from(
            "# Topic\n\n\
             ### 1.1 Q?\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.2 R?\n\
             depends: 1.1 Q?=A|B\n\n\
             Problem: p2.\n\n\
             Standard: s2.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.3 S?\n\
             depends: 1.1 Q?=*\n\n\
             Problem: p3.\n\n\
             Standard: s3.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert_eq!(
            rf.sections[1].depends[0],
            Dep::Text {
                text: "1.1 Q?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A', 'B']))
            }
        );
        assert_eq!(
            rf.sections[2].depends[0],
            Dep::Text {
                text: "1.1 Q?".to_string(),
                set: Some(AnswerSet::Any)
            }
        );
    }

    // adr: b0554224-15e4-4e91-a2bb-9ae5667803f4
    // "Does a reference line hold its list on one line?": two heading-shaped
    // entries on the *same* depends line, each with its own distinct
    // answer set: the sets must not bleed into each other or collapse to
    // one set for the whole line.
    #[test]
    fn depends_two_entries_on_one_line_keep_separate_answer_sets() {
        let rf = rf_from(
            "# Topic\n\n\
             ### 1.1 Q?\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.2 Q2?\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.3 R?\n\
             depends: 1.1 Q?=A, 1.2 Q2?=A|B\n\n\
             Problem: p3.\n\n\
             Standard: s3.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[2];
        assert_eq!(
            sec.shape_errors,
            Vec::<String>::new(),
            "{:?}",
            sec.shape_errors
        );
        assert_eq!(
            sec.depends[0],
            Dep::Text {
                text: "1.1 Q?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A']))
            },
            "the first entry's set must be its own, not the second entry's"
        );
        assert_eq!(
            sec.depends[1],
            Dep::Text {
                text: "1.2 Q2?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A', 'B']))
            },
            "the second entry's set must be its own, not the first entry's"
        );
    }

    #[test]
    fn depends_malformed_entries_on_one_line_are_shape_errors() {
        // All four problems (an empty entry, a bad set, and a duplicate) are
        // entries on the same, single `depends:` line -- per "Does a
        // reference line hold its whole list on one line?": a section
        // holds at most one `depends:` line, comma separated -- so every
        // error below names the same line number.
        let rf = rf_from(
            "# Topic\n\n\
             ### 1.1 Q?\n\
             depends: , 1.1 Q?=a, widget, widget\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[0];
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "depends: empty entry, line 4"),
            "{:?}",
            sec.shape_errors
        );
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "depends: bad set syntax, line 4"),
            "{:?}",
            sec.shape_errors
        );
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "depends: duplicate entry, line 4"),
            "{:?}",
            sec.shape_errors
        );
    }

    #[test]
    fn depends_entry_with_a_comma_in_heading_text_rejoins_against_the_file_headings() {
        let rf = rf_from(
            "# Topic\n\n\
             ### First, then second?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### R?\ndepends: First, then second?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[1];
        assert_eq!(
            sec.shape_errors,
            Vec::<String>::new(),
            "{:?}",
            sec.shape_errors
        );
        assert_eq!(
            sec.depends.len(),
            1,
            "the comma must not split into two entries"
        );
        assert_eq!(
            sec.depends[0],
            Dep::Text {
                text: "First, then second?".to_string(),
                set: None
            }
        );
        assert_eq!(rf.dep_target(&sec.depends[0]), Some(0));
    }

    #[test]
    fn depends_comma_rejoin_prefers_the_longest_heading_match() {
        // "Alpha, Beta?" is a real heading in this file too; the rejoin must
        // still prefer the longer, full match "Alpha, Beta, Gamma?" over the
        // shorter one it could stop at ("longest match wins").
        let rf = rf_from(
            "# Topic\n\n\
             ### Alpha, Beta?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### Alpha, Beta, Gamma?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### R?\ndepends: Alpha, Beta, Gamma?=A\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = rf.sections.iter().find(|s| s.heading_text == "R?").unwrap();
        assert_eq!(sec.depends.len(), 1, "{:?}", sec.depends);
        assert_eq!(
            sec.depends[0],
            Dep::Text {
                text: "Alpha, Beta, Gamma?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A']))
            }
        );
    }
}
