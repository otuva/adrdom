//! The rules on the depends and supersedes lines of a question.
use super::*;
use crate::depends::{self, DepTarget};
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::state;
use crate::status;
use crate::terms;
use std::collections::{HashMap, HashSet};

/// The three shapes of a `depends:` entry `approve` has stopped, for every
/// question of `rf` not yet asked and with a body (per "Does a depends entry
/// on a question in its file carry an answer set?"): a set on an entry that
/// names a term, an entry on a question of `rf` with no answer that carries
/// no set, and a set on an answered question that `approve` did not freeze. The
/// one function `check`, `approve` and `plan` read, found before approval in
/// text that can still change.
pub fn depends_shape_refusals(
    rel: &str,
    rf: &RecordFile,
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    led: &Ledger,
) -> Vec<msg::Said> {
    let mut out = Vec::new();
    for sec in &rf.sections {
        if !status::open_to_ask(led, rf, sec) {
            continue;
        }
        // Two entries that name one thing (a term and its alias, a name and
        // its id) would be written as one id twice by approve, a line parse
        // refuses: the second is a finding here, before approval.
        let mut seen: Vec<String> = Vec::new();
        for d in &sec.depends {
            let Ok(Some(target)) = depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, led)
            else {
                continue;
            };
            let key = match &target {
                DepTarget::Heading(idx) => rf.sections[*idx]
                    .id
                    .clone()
                    .unwrap_or_else(|| format!("heading {}", idx)),
                DepTarget::Term(id) | DepTarget::Decision(id) | DepTarget::Struck(id) => id.clone(),
            };
            if seen.contains(&key) {
                let written = match d {
                    record::Dep::Id { id, .. } => id.as_str(),
                    record::Dep::Text { text, .. } => text.as_str(),
                };
                out.push(msg::depends_names_twice(rel, &sec.heading_text, written));
                continue;
            }
            seen.push(key);
            let target_section = match &target {
                DepTarget::Heading(idx) => Some(&rf.sections[*idx]),
                DepTarget::Decision(_) | DepTarget::Term(_) | DepTarget::Struck(_) => None,
            };
            // An answer set names the letters of another question: a term has
            // no options (per "Does a depends entry on a question in its file
            // carry an answer set?").
            if matches!(target, DepTarget::Term(_)) && d.set().is_some() {
                out.push(msg::set_on_term(rel, &sec.heading_text));
                continue;
            }
            // A question struck from the file is answered and was a question of
            // the file: its entry is judged as before the strike (per "What
            // strikes the section of a superseded question from its topic
            // file?"): a set stays only when frozen at ask.
            if let (DepTarget::Struck(tid), Some(set)) = (&target, d.set()) {
                if !sec
                    .id
                    .as_deref()
                    .is_some_and(|sid| set_frozen_ids(led, sid, tid, set))
                {
                    let name = led
                        .latest_for_id("asked", tid)
                        .map(|r| r.detail.clone())
                        .unwrap_or_else(|| tid.clone());
                    out.push(msg::set_on_answered_question(rel, &sec.heading_text, &name));
                }
                continue;
            }
            let Some(ts) = target_section else { continue };
            match d.set() {
                None if ts.answer_letter().is_none() => out.push(msg::missing_answer_set(
                    rel,
                    &sec.heading_text,
                    &ts.heading_text,
                )),
                // A set approved before its target was answered stays (per
                // "Does a depends entry on a question in its file carry an
                // answer set?"): a set with no `assumes` row of the target
                // for this entry, written when the target was asked, is
                // refused.
                Some(_) if ts.answer_letter().is_some() && !set_frozen_at_ask(led, sec, d, ts) => {
                    out.push(msg::set_on_answered_question(
                        rel,
                        &sec.heading_text,
                        &ts.heading_text,
                    ))
                }
                _ => {}
            }
        }
    }
    out
}

/// Every refusal the `supersedes:` lines of `rf` earn, the one place for the
/// supersede rules, read by `record_refusals` (so `approve`, `check` and
/// `show`), by `approve` for one candidate (`only`) and by `answer` for its own
/// question. Category `plan`. Judged: each live section with a `supersedes:`
/// line that is not frozen (an answered, refused or superseded section is
/// history); an asked section whose text still stands is judged too, since
/// `approve` froze its line and `answer` reads it. `only` narrows the judged
/// sections to that index. The rules, in this order: every entry names a
/// current decision (an asked section also accepts a target superseded after
/// the section's latest `asked` row, a sibling replacer of the same decision
/// answered first; compared by ledger position, never by time); the line
/// leaves out no decision standing on a target; the section does not stand on
/// a target; no other live, unanswered section of the file stands on a target.
pub fn supersedes_refusals(
    rel: &str,
    rf: &RecordFile,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    names: &HashMap<String, String>,
    all: &[RecordFile],
    led: &Ledger,
    only: Option<usize>,
) -> Vec<Finding> {
    let refuse = |index: usize, said: msg::Said| Finding::said(Class::Plan, said).in_section(index);
    let mut out: Vec<Finding> = Vec::new();
    // (section index, asked, target ids) of each judged section.
    let mut judged: Vec<(usize, bool, Vec<String>)> = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if sec.supersedes.is_empty()
            || only.is_some_and(|o| o != i)
            || state::is_frozen(led, sec)
            || state::is_dead(led, sec)
        {
            continue;
        }
        let asked = crate::view::asked_text_stands(led, rf, sec);
        let mut ids = Vec::new();
        for old in &depends::supersedes_entries(&sec.supersedes, cross_file_headings, led) {
            match depends::resolve_supersedes(old, cross_file_headings, led) {
                // One decision named twice (by heading and by identifier, or
                // twice alike): found before approval, never written twice.
                Ok(id) if ids.contains(&id) => {
                    out.push(refuse(
                        i,
                        msg::supersedes_names_twice(rel, &sec.heading_text, old),
                    ));
                }
                Ok(id) => ids.push(id),
                Err(depends::SupersedesError::Ambiguous(files)) => {
                    out.push(refuse(
                        i,
                        msg::ask_supersedes_ambiguous(rel, &sec.heading_text, old, &files),
                    ));
                }
                Err(depends::SupersedesError::NotCurrent) => {
                    let split_sibling = asked
                        && sec.id.as_deref().is_some_and(|id| {
                            matches!(led.latest_state(old), Some(r) if r.event == "superseded")
                                && matches!(
                                    (led.latest_asked_pos(id), led.latest_superseded_pos(old)),
                                    (Some(a), Some(s)) if s > a
                                )
                        });
                    if split_sibling {
                        ids.push(old.trim().to_string());
                    } else {
                        out.push(refuse(
                            i,
                            msg::ask_supersedes_not_current(
                                rel,
                                &sec.heading_text,
                                old,
                                names.get(old).map(String::as_str),
                            ),
                        ));
                    }
                }
            }
        }
        judged.push((i, asked, ids));
    }

    // Only sections of this file: an open question of another file that
    // stands on a replaced decision is left to check's not-current refusal.
    let stands_on = |d: &record::Dep, target: &str| -> bool {
        let at = rf
            .sections
            .iter()
            .find(|s| s.id.as_deref() == Some(target))
            .map(|s| (rf.rel.as_str(), s.heading_text.trim()));
        depends::entry_names_target(d, &rf.rel, target, at)
    };
    for (i, asked, ids) in &judged {
        let sec = &rf.sections[*i];
        let missing = depends::left_out_of_supersedes(all, led, ids);
        if !missing.is_empty() {
            let moment = if *asked {
                msg::SupersedesMoment::Asked
            } else {
                msg::SupersedesMoment::Approve
            };
            out.push(refuse(
                *i,
                msg::supersedes_leaves_out(rel, &sec.heading_text, &missing, moment),
            ));
        }
        for target in ids {
            let target_name = names.get(target).map(String::as_str).unwrap_or("");
            if sec.depends.iter().any(|d| stands_on(d, target)) {
                out.push(refuse(
                    *i,
                    msg::stands_on_what_it_supersedes(rel, &sec.heading_text, target_name),
                ));
            }
            for (j, other) in rf.sections.iter().enumerate() {
                let open = other
                    .id
                    .as_deref()
                    .is_none_or(|id| !state::is_current(led, id))
                    && !state::is_dead(led, other);
                if j != *i && open && other.depends.iter().any(|d| stands_on(d, target)) {
                    out.push(refuse(
                        j,
                        msg::stands_on_what_the_plan_supersedes(
                            rel,
                            &other.heading_text,
                            target_name,
                            &sec.heading_text,
                        ),
                    ));
                }
            }
        }
    }
    out
}

/// True when the set on `d` is the one `approve` froze on `target` for `sec`:
/// an `assumes` row of the target for this dependent carries the hash of
/// exactly this entry. A set written after the target was answered has no
/// such row, so it is a shape error.
pub(crate) fn set_frozen_at_ask(
    led: &Ledger,
    sec: &record::Section,
    d: &record::Dep,
    target: &record::Section,
) -> bool {
    let (Some(id), Some(tid), Some(set)) = (&sec.id, &target.id, d.set()) else {
        return false;
    };
    set_frozen_ids(led, id, tid, set)
}

fn set_frozen_ids(led: &Ledger, id: &str, tid: &str, set: &record::AnswerSet) -> bool {
    let hash = record::assumes_hash(id, set);
    led.assumes_for(tid)
        .iter()
        .any(|r| r.detail == id && r.hash == hash)
}

/// The rule on the tiers of `rf` (per "What are the tiers derived from?"):
/// the one finding its derive error earns. A heading-shaped entry that names
/// no heading of the file but a current decision of another topic file, on a
/// question being judged, earns the finding on what a depends entry names
/// instead (per "What does a depends entry name?"). About the questions the
/// error names.
pub fn derive_finding(
    rel: &str,
    rf: &RecordFile,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    led: &Ledger,
) -> Option<Finding> {
    let err = rf.derive_error.as_ref()?;
    let other_file = match err {
        record::DeriveError::Dangling { section, entry } => rf
            .sections
            .iter()
            .find(|s| s.heading_text.trim() == section.trim())
            .filter(|s| status::open_to_ask_text(led, rf, s))
            .filter(|_| names_decision_of_other_file(rel, entry, cross_file_headings))
            .map(|_| {
                Finding::said(
                    Class::Plan,
                    msg::dep_names_heading_of_other_file(rel, section, entry.trim()),
                )
            }),
        _ => None,
    };
    // The sections the error is about (`check` reads them to tell a
    // question that can still change from one that cannot): the one
    // holding the entry, the self-dependent one, or every one in the cycle.
    let named: Vec<&str> = match err {
        record::DeriveError::Dangling { section, .. } => vec![section.as_str()],
        record::DeriveError::SelfDep(h) => vec![h.as_str()],
        record::DeriveError::Cycle(hs) => hs.iter().map(String::as_str).collect(),
    };
    let at: Vec<usize> = rf
        .sections
        .iter()
        .enumerate()
        .filter(|(_, s)| named.contains(&s.heading_text.as_str()))
        .map(|(i, _)| i)
        .collect();
    let f = other_file.unwrap_or_else(|| Finding::said(Class::Plan, msg::derive_error(rel, err)));
    Some(f.in_sections(at))
}

/// Whether `entry` is the heading of a current decision of a topic file other
/// than `rel` (`depends::cross_file_heading_index`).
fn names_decision_of_other_file(
    rel: &str,
    entry: &str,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
) -> bool {
    cross_file_headings
        .get(entry.trim())
        .is_some_and(|at| at.iter().any(|(file, _)| file != rel))
}

/// The cap on a depends line (per "What bounds the questions a depends line
/// names?"): a question being judged (`status::open_to_ask_text`) whose
/// depends line names more than `DEPENDS_LINE_CAP` questions of its file, once
/// each, by heading or by identifier. A term does not count.
pub fn depends_line_cap_findings(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<Finding> {
    let mut out = Vec::new();
    for (si, sec) in rf.sections.iter().enumerate() {
        if !status::open_to_ask_text(led, rf, sec) {
            continue;
        }
        // A question struck from the file still counts as one it names.
        let mut targets: Vec<(String, String)> = Vec::new();
        for d in &sec.depends {
            let named = match (rf.dep_target(d), d) {
                (Some(idx), _) => Some((
                    format!("#{}", idx),
                    rf.sections[idx].heading_text.trim().to_string(),
                )),
                (None, record::Dep::Id { id, .. }) if state::is_struck_from(led, id, rel) => {
                    let name = led
                        .latest_for_id("asked", id)
                        .map(|r| r.detail.clone())
                        .unwrap_or_else(|| id.clone());
                    Some((id.clone(), name))
                }
                _ => None,
            };
            if let Some(n) = named {
                if !targets.iter().any(|t| t.0 == n.0) {
                    targets.push(n);
                }
            }
        }
        if targets.len() > record::DEPENDS_LINE_CAP {
            let names: Vec<String> = targets.into_iter().map(|t| t.1).collect();
            out.push(
                Finding::said(
                    Class::Plan,
                    msg::depends_line_over_limit(
                        rel,
                        &sec.heading_text,
                        &names,
                        record::DEPENDS_LINE_CAP,
                    ),
                )
                .in_section(si),
            );
        }
    }
    out
}

/// The cap (per "What bounds the questions standing on one question?"): no
/// more than `STAND_CAP` live questions stand on one question, and no more
/// than `STAND_CAP` stand on terms alone. Only judged when tiers were derived
/// (a derive error is reported first); a refused, superseded or withdrawn
/// question does not count, nor a decision an open question supersedes
/// (`replaced`).
pub fn cap_finding(
    rel: &str,
    rf: &RecordFile,
    replaced: &HashSet<String>,
    led: &Ledger,
) -> Option<Finding> {
    let (parent, open, answered) = state::over_cap_headings(led, rf, replaced)?;
    Some(Finding::said(
        Class::Plan,
        msg::stand_cap_exceeded(rel, parent.as_deref(), &open, answered, record::STAND_CAP),
    ))
}
