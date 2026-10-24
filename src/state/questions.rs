//! What the ledger says about a question: current, frozen, dead, open again, the tier and cap walks, and what
//! holds an answer back.
use crate::ledger::{Ledger, Row};
use crate::record::{self, RecordFile, Section};

/// True when `id`'s latest state row in the ledger is `answered`
/// (current). `superseded`/`withdrawn`/`asked`/never-asked are all false.
pub fn is_current(led: &Ledger, id: &str) -> bool {
    matches!(led.latest_state(id), Some(row) if row.event == "answered")
}

/// True when `sec`'s latest ledger state is `answered`, `refused`, or
/// `superseded`: its decision is closed, one way or another, so its text --
/// heading included -- is never edited again (per "Does a decision change
/// after it is answered?" and "Does the tool stop loaded phrases in a
/// question?"). A
/// section with no id yet, or none of those three states, is unfrozen.
/// The one shared "is this section frozen" test `approve` and `check` both
/// use.
pub fn is_frozen(led: &Ledger, sec: &Section) -> bool {
    use crate::status::QuestionStatus;
    matches!(
        crate::status::question_status(led, sec),
        QuestionStatus::Answered | QuestionStatus::Refused | QuestionStatus::Superseded
    )
}

/// True when `sec`'s latest ledger state is `refused`, `superseded`, or
/// `withdrawn`: the question is closed and out of play -- a refused
/// question is done, a superseded decision has been replaced, a withdrawn
/// question no longer stands in the file -- so it is no member the cap counts
/// (per "What bounds the questions standing on one question?").
/// `answered` is still live for this purpose (only
/// `is_frozen`'s edit-lock treats it as closed); a section with no id yet
/// is live. A `dropped` question is out of play too
/// (`QuestionStatus::is_dead`).
pub fn is_dead(led: &Ledger, sec: &Section) -> bool {
    crate::status::question_status(led, sec).is_dead()
}

/// True when `sec` is named on the supersedes line of a question not
/// answered yet (`depends::replaced_ids`): it is left out of the cap's count
/// (per "Does the cap count a decision an open question supersedes?"), so
/// the cap holds by what stands once that question is answered.
fn is_replaced(sec: &Section, replaced: &std::collections::HashSet<String>) -> bool {
    sec.id.as_ref().is_some_and(|id| replaced.contains(id))
}

/// True when the latest ledger state of `id` is `asked` and that row holds
/// `cur_hash`, the hash of the section's text now: the text stands as it was
/// asked. `answer` takes a question only then, `show` prints its version
/// only then, and `next` names the owner's answer only then; an edited
/// question is the agent's to ask again (per "Does an answer carry the
/// version of the question the owner read?").
pub fn asked_text_stands(led: &Ledger, id: &str, cur_hash: &str) -> bool {
    matches!(led.latest_state(id), Some(row) if row.event == "asked" && row.hash == cur_hash)
}

/// True when the latest ledger state of `id` is `asked` although an earlier
/// row answered or refused it: the question was asked again (per "Does
/// check fail a question answered with two different options?") and waits
/// for the owner.
pub fn is_open_again(led: &Ledger, id: &str) -> bool {
    matches!(led.latest_state(id), Some(row) if row.event == "asked")
        && led
            .rows
            .iter()
            .any(|r| r.id == id && matches!(r.event.as_str(), "answered" | "refused"))
}

/// The first group of live questions over the cap (per "What bounds the
/// questions standing on one question?"), `None` when none is. The groups
/// are the questions standing on terms alone, then the questions standing on
/// each question in file order; a question naming two in-file questions
/// stands in the group of each. Only live questions count: a refused,
/// superseded or withdrawn one does not, nor a decision an open question
/// supersedes (`replaced`; per "Does the cap count a decision an open
/// question supersedes?"). What a question stands on is its depends line, as
/// for the tiers (per "What are the tiers derived from?"), whatever state the
/// question it names is in. A group whose members are all answered is not
/// judged (per "Does the cap stop a plan whose questions over it are all
/// answered?"); one that holds an open question is, and the refusal names
/// that group's parent heading (`None` for terms alone), the headings of the
/// open members in file order, and the count of the answered ones. Answered
/// members hold their place but their depends line is never edited again
/// (`is_frozen`), so the refusal (per "Does a refusal name the command that
/// clears it?") names only the ones the agent can still move. Read by
/// `approve` and `check` through `record_refusals`.
pub fn over_cap_headings(
    led: &Ledger,
    rf: &RecordFile,
    replaced: &std::collections::HashSet<String>,
) -> Option<(Option<String>, Vec<String>, usize)> {
    // Depends lines that do not resolve are a derive error, reported on its
    // own; the groups are read only off lines that resolve.
    rf.tiers.as_ref()?;
    let live: Vec<bool> = rf
        .sections
        .iter()
        .map(|sec| !is_dead(led, sec) && !is_replaced(sec, replaced))
        .collect();
    let parents = crate::record::in_file_parents(rf);
    let n = rf.sections.len();
    // groups[0] is terms alone; groups[p + 1] holds what stands on section p.
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); n + 1];
    for c in (0..n).filter(|&c| live[c]) {
        // What a question stands on is read off its depends line alone,
        // whatever state the question it names is in.
        if parents[c].is_empty() {
            groups[0].push(c);
        } else {
            for &p in &parents[c] {
                groups[p + 1].push(c);
            }
        }
    }
    let is_answered = |c: &usize| {
        rf.sections[*c]
            .id
            .as_deref()
            .is_some_and(|id| is_current(led, id))
    };
    let (g, kids) = groups
        .into_iter()
        .enumerate()
        .find(|(_, kids)| kids.len() > crate::record::STAND_CAP && !kids.iter().all(is_answered))?;
    let (answered, open): (Vec<usize>, Vec<usize>) = kids.into_iter().partition(is_answered);
    Some((
        g.checked_sub(1)
            .map(|p| rf.sections[p].heading_text.clone()),
        open.into_iter()
            .map(|c| rf.sections[c].heading_text.clone())
            .collect(),
        answered.len(),
    ))
}

/// True when `sec` has no letter answer yet: no `**Answer:** <letter>`
/// line at all (per "Does a topic file close?" and "Does the owner answer
/// a question with a refusal?") -- a skeleton with nothing written, an
/// unasked or asked-but-open question, and a refused question (whose
/// `**Answer:** refused` line never parses as a letter) are all
/// unresolved; only a genuine letter answer, or a superseded section's old
/// letter still sitting in the file, resolves it. Read straight off the
/// file text, no ledger needed: the one "is this section unresolved" test
/// the tier rule reads.
pub fn tier_unresolved(sec: &Section) -> bool {
    sec.answer_letter().is_none()
}

/// The section beside section `i` in its tier that is not answered: the
/// first one still waiting for an answer, else the first refused one (a
/// refused question counts as not answered until a plan change). What
/// "Does the agent build on a decision before its tier is fully answered?"
/// waits on; `None` when the tier is fully answered or has no derived tier.
pub fn unresolved_sibling(rf: &RecordFile, i: usize) -> Option<&Section> {
    let tiers = rf.tiers.as_ref()?;
    let find = |refused: bool| {
        rf.sections
            .iter()
            .enumerate()
            .find(|(j, s)| {
                *j != i && tiers[*j] == tiers[i] && tier_unresolved(s) && s.is_refused() == refused
            })
            .map(|(_, s)| s)
    };
    find(false).or_else(|| find(true))
}

/// What holds question `i` of a file back from `answer` (per "Does answer take a question standing
/// on a question that has no decision?" and "Does a refused question hold the tiers above it?"):
/// the questions of the same file it names on its depends line that have no letter answer, split
/// into those still waiting for one (`unanswered`) and refused ones (`refused_target`), and the
/// refused questions of a lower tier it does not stand on (`refused_tier`). Read off the file text
/// alone, like `tier_unresolved`. An unanswered question that is not a target holds nothing back. A
/// question open to ask names no decision of another file, and an old entry that does, on a closed
/// question, never waits.
#[derive(Debug, Default, PartialEq)]
pub struct AnswerWaits {
    pub unanswered: Vec<usize>,
    pub refused_target: Vec<usize>,
    pub refused_tier: Vec<usize>,
}

impl AnswerWaits {
    pub fn is_empty(&self) -> bool {
        self.unanswered.is_empty() && self.refused_target.is_empty() && self.refused_tier.is_empty()
    }
}

/// The one test `answer`, `next` and the page read to tell whether `answer`
/// takes question `i` now: it is empty exactly when it does.
pub fn answer_waits(rf: &RecordFile, i: usize) -> AnswerWaits {
    let mut w = AnswerWaits::default();
    for d in &rf.sections[i].depends {
        let Some(t) = rf.dep_target(d) else { continue };
        if t == i || !tier_unresolved(&rf.sections[t]) {
            continue;
        }
        let list = if rf.sections[t].is_refused() {
            &mut w.refused_target
        } else {
            &mut w.unanswered
        };
        if !list.contains(&t) {
            list.push(t);
        }
    }
    if let Some(mine) = rf.tier_of(i) {
        for (j, s) in rf.sections.iter().enumerate() {
            if s.is_refused()
                && rf.tier_of(j).is_some_and(|t| t < mine)
                && !w.refused_target.contains(&j)
            {
                w.refused_tier.push(j);
            }
        }
    }
    w
}

/// What comparing a closed (answered or refused) section's current text
/// against the ledger rows that back it finds (per "Does check fail an
/// answered question whose text changed?" and "Does the owner answer a
/// question with a refusal?"). `check`'s per-section answered/refused
/// comparison and `approve`'s "edited after it closed" gate share this instead
/// of each keeping its own copy, so a genuine re-answer -- a harmless edit
/// followed by a real re-answer at the same letter, which adds a second
/// `answered` row rather than rewriting the first -- never desyncs the
/// two: every row for the id at the relevant event (and, for "answered",
/// at the letter the file currently names) is checked, not only the
/// latest.
pub enum ClosedMatch {
    /// No `**Answer:**` line at all, or one with nothing behind it in the
    /// ledger, ever, at this event.
    NotClosed,
    /// A genuine row backs the current text exactly.
    Current,
    /// The section was genuinely closed at some point (a real row for this
    /// id at this event), but none of those rows' hash matches the current
    /// text: it moved since.
    Edited,
}

/// `sec`'s `ClosedMatch`, from `id`'s ledger rows and `cur_hash` (the
/// caller's already-computed `record::section_hash(lines, sec)`, passed in
/// rather than recomputed here since both callers already have it).
pub fn closed_match(led: &Ledger, cur_hash: &str, sec: &Section, id: &str) -> ClosedMatch {
    if sec.is_refused() {
        let rows_for_id: Vec<&Row> = led
            .rows
            .iter()
            .filter(|r| r.event == "refused" && r.id == id)
            .collect();
        if rows_for_id.iter().any(|r| r.hash == cur_hash) {
            ClosedMatch::Current
        } else if !rows_for_id.is_empty() {
            ClosedMatch::Edited
        } else {
            ClosedMatch::NotClosed
        }
    } else if sec.answer_line.is_some() {
        let rows_for_id: Vec<&Row> = led
            .rows
            .iter()
            .filter(|r| r.event == "answered" && r.id == id)
            .collect();
        let letter_matches = sec.answer_letter().is_some_and(|l| {
            rows_for_id
                .iter()
                .any(|r| r.detail == l.to_string() && r.hash == cur_hash)
        });
        if letter_matches {
            ClosedMatch::Current
        } else if !rows_for_id.is_empty() {
            ClosedMatch::Edited
        } else {
            ClosedMatch::NotClosed
        }
    } else {
        ClosedMatch::NotClosed
    }
}

/// True when `sec` is frozen (`is_frozen`) and its text as it stands in
/// `lines` is not the text the ledger holds for it: the same judgment
/// `check` passes in `check_answers` (`closed_match` for a section with an
/// answer or refusal line, else the latest `asked` row's hash), shared so
/// `approve` refuses before writing what `check` would fail right after.
pub fn frozen_text_edited(led: &Ledger, lines: &[String], sec: &Section) -> bool {
    let Some(id) = &sec.id else { return false };
    if !is_frozen(led, sec) {
        return false;
    }
    let cur = crate::record::section_hash(lines, sec);
    if sec.answer_line.is_some() || sec.is_refused() {
        matches!(closed_match(led, &cur, sec, id), ClosedMatch::Edited)
    } else {
        led.latest_asked(id).is_some_and(|r| r.hash != cur)
    }
}

/// The text a section's `terms:` line is (re)computed from, and what the
/// banned-word scan reads (per "What does a reference in a topic file carry?"): the section's
/// hashed range (heading through the hash end, see `record::hash_range`) with the `terms:` line
/// itself left out (so re-resolving never matches against a term's own previous resolution rather
/// than the prose that produced it), and with `id:`, `depends:`, `supersedes:`, `blame:` and
/// `unrelated:` (of an earlier form) lines left out too, since those hold ids and references, not
/// prose, once a plan is
/// approved.
pub fn section_text_excluding_terms_line(lines: &[String], sec: &Section) -> String {
    let (s, e) = record::hash_range(lines, sec);
    lines[s..e]
        .iter()
        .enumerate()
        .filter(|&(j, l)| {
            let idx = s + j;
            if Some(idx) == sec.terms_line {
                return false;
            }
            !crate::tells::starts_with_any(
                l,
                &[
                    crate::tells::ID_PREFIX,
                    crate::tells::DEPENDS_PREFIX,
                    crate::tells::SUPERSEDES_PREFIX,
                    crate::tells::UNRELATED_PREFIX,
                    crate::tells::BLAME_PREFIX,
                ],
            )
        })
        .map(|(_, l)| l.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    //
    // The text a question is paired to terms from, and the banned-word scan
    // reads, holds the question's words: the reference lines (a blame file's
    // name among them) and the `terms:` line are left out.
    #[test]
    fn the_text_read_for_terms_leaves_out_every_reference_line() {
        let content = "# T\n\n### Does the widget hold?\ndepends: widget\nunrelated: a, b\nblame: widgetnotes 1-2\nid: 11111111-1111-4111-8111-111111111111\nterms: widget=22222222-2222-4222-8222-222222222222\n\nProblem: p.\n\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n";
        let rf = record::build(
            PathBuf::from("t.md"),
            "t.md".to_string(),
            record::split_lines(content),
        );
        let text = section_text_excluding_terms_line(&rf.lines, &rf.sections[0]);
        assert!(
            text.starts_with("### Does the widget hold?\n\nProblem: p."),
            "{}",
            text
        );
        for left_out in [
            "depends:",
            "unrelated:",
            "blame:",
            "widgetnotes",
            "id:",
            "terms:",
        ] {
            assert!(!text.contains(left_out), "{} in {}", left_out, text);
        }
    }
}
