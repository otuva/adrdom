//! The rules on one topic file, section by section.
use super::*;
use crate::depends;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state;
use crate::status;
use crate::tells;
use crate::terms;
use std::collections::{HashMap, HashSet};

fn length_found(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<(usize, msg::Said)> {
    let mut out = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if !status::open_to_ask_text(led, rf, sec) {
            continue;
        }
        let (s, e) = record::hash_range(&rf.lines, sec);
        for (line, words, kind) in tells::length_tells(&rf.lines, s, e, &sec.heading_text) {
            out.push((i, msg::too_long(rel, line, &sec.heading_text, kind, words)));
        }
    }
    out
}

/// `found` as findings of `class`, each about its question.
pub(super) fn in_sections(class: Class, found: Vec<(usize, msg::Said)>) -> Vec<Finding> {
    found
        .into_iter()
        .map(|(i, m)| Finding::said(class, m).in_section(i))
        .collect()
}

fn body_tell_found(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<(usize, msg::Said)> {
    let mut out = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if !status::open_to_ask_text(led, rf, sec) {
            continue;
        }
        let (s, e) = record::hash_range(&rf.lines, sec);
        for (line, phrase, what) in tells::prose_tells_in(&rf.lines, s + 1, e) {
            out.push((i, msg::tell(rel, line, &sec.heading_text, &phrase, what)));
        }
    }
    out
}

fn no_term_found(
    rel: &str,
    rf: &RecordFile,
    terms_list: &[terms::Term],
    led: &Ledger,
) -> Vec<(usize, msg::Said)> {
    let mut out = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if !status::open_to_ask(led, rf, sec) {
            continue;
        }
        let text = state::section_text_excluding_terms_line(&rf.lines, sec);
        if terms::resolve(&text, terms_list).is_empty() {
            out.push((i, msg::pairs_no_term(rel, &sec.heading_text)));
        }
    }
    out
}

/// One `bad_letters` message per answer-set letter of a depends entry on a
/// question of `rf` that has no such option, for every question of `rf` not
/// yet asked (per "Does a depends entry carry the answers it holds under?"):
/// the one function `check`, `approve` and `plan` read, found before approval
/// what `approve` refuses. Cross-file targets carry no set before they are
/// answered, so only a same-file target is read.
pub fn letter_refusals(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<Finding> {
    in_sections(Class::Plan, letter_found(rel, rf, led))
}

/// A pair is judged when the dependent is open to ask, or when its target
/// has no letter answer yet: a target edited and asked again can lose the
/// option an asked dependent's set names, which `approve` refuses.
pub(super) fn letter_found(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<(usize, msg::Said)> {
    let mut out = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        let open = status::open_to_ask_text(led, rf, sec);
        for d in &sec.depends {
            let Some(record::AnswerSet::Letters(letters)) = d.set() else {
                continue;
            };
            let target = match d {
                record::Dep::Id { id, .. } => rf
                    .sections
                    .iter()
                    .find(|s| s.id.as_deref() == Some(id.as_str())),
                record::Dep::Text { text, .. } => {
                    let t = text.trim();
                    if t.ends_with('?') {
                        rf.sections.iter().find(|s| s.heading_text.trim() == t)
                    } else {
                        None
                    }
                }
            };
            if let Some(ts) = target.filter(|ts| open || state::tier_unresolved(ts)) {
                out.extend(
                    depends::bad_letters(rel, &sec.heading_text, ts, letters)
                        .into_iter()
                        .map(|m| (i, m)),
                );
            }
        }
    }
    out
}

/// The rules on what the ledger holds of each question's answer or refusal
/// against what its topic file holds (per "Does check fail an answered
/// question whose text changed?", "Does the owner answer a question with a
/// refusal?" and "Does check fail a question answered with two different
/// options?"), one judgement `check` fails, `approve` refuses on,
/// and `show` and `plan` print: two standing answers; an answer or refusal
/// line with no row behind it; a closed question whose text moved since it
/// was answered or refused; a row whose line the file no longer holds.
pub fn closed_findings(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<Finding> {
    let mut out = Vec::new();
    for (si, sec) in rf.sections.iter().enumerate() {
        let Some(id) = &sec.id else { continue };
        let cur = record::section_hash(&rf.lines, sec);
        let mut push = |m: msg::Said| out.push(Finding::said(Class::Answers, m).in_section(si));
        // The file's own `**Answer:**` line shows one side only: two
        // standing rulings for different options are read off the ledger.
        if let Some((a, b)) = state::conflicting_rulings(led, id) {
            push(msg::answer_conflict(
                rel,
                &sec.heading_text,
                &a.label(),
                &b.label(),
            ));
        }
        let open_again = state::is_open_again(led, id);
        if sec.answer_line.is_some() && open_again {
            // Asked again and not ruled on since: no answer stands, so a
            // line in the file is hand-written.
            push(msg::answer_without_row(rel, &sec.heading_text));
        } else if sec.answer_line.is_some() {
            match state::closed_match(led, &cur, sec, id) {
                state::ClosedMatch::Current => {}
                // A genuine row exists: the ruling is real and the
                // question's text moved since.
                state::ClosedMatch::Edited if sec.is_refused() => {
                    push(msg::refused_text_changed(rel, &sec.heading_text))
                }
                state::ClosedMatch::Edited => {
                    push(msg::answered_text_changed(rel, &sec.heading_text))
                }
                state::ClosedMatch::NotClosed if sec.is_refused() => {
                    push(msg::refusal_without_row(rel, &sec.heading_text))
                }
                state::ClosedMatch::NotClosed => {
                    push(msg::answer_without_row(rel, &sec.heading_text))
                }
            }
        } else if !open_again {
            if led
                .rows
                .iter()
                .any(|r| r.event == "answered" && &r.id == id)
            {
                push(msg::answered_row_without_line(rel, &sec.heading_text));
            } else if led.rows.iter().any(|r| r.event == "refused" && &r.id == id) {
                push(msg::refused_row_without_line(rel, &sec.heading_text));
            }
        }
    }
    out
}

/// Every refusal one topic file (`rf`, at `rel`) earns on its own: shape
/// errors and parse errors -- an option with no decision sentence or more
/// than one, a duplicate id line, an answer naming a missing option --
/// (both `parse`), a heading/context-prose tell on text not yet frozen
/// (`tells`), a missing `depends:` line, an entry that fails to resolve, a
/// frozen section's edited depends entry, the derive error, and the tier
/// cap (all `plan`) -- every category `do_approve_record` refuses on
/// except the cross-file "answered id missing from every topic file"
/// check, which needs every topic file, not just this one
/// (`missing_answered_refusals`, below). Shared with `check` and `approve`
/// (178a232e A point 1, one function for all three, mirroring
/// `terms_refusals`): `approve` prefixes each message `approve: `,
/// `check` prefixes it `<category>: `, `approve` prefixes the "parse" slice
/// `ask: `, so the same line reads identically wherever it is printed.
///
/// `terms_list`/`phrase_lookup` are supplied by the caller rather than
/// loaded here: `approve` passes its own already-approved terms (gated by
/// `require_terms` before this is ever called), `check` a copy with every
/// id minted in memory so a real term in a terms.md that has never been
/// approved still resolves instead of drowning this in "which is no
/// term" refusals. `cross_file_headings` (`depends::cross_file_heading_index`,
/// built once per command run) is read only by the `supersedes:` and
/// `unrelated:` rules; a `depends:` entry is resolved against the file and
/// the terms alone.
///
/// `names` is every heading by its id, of any state and any topic file
/// (`view::heading_texts`), built once per command run by the caller:
/// a refusal that names an identifier names the question beside it.
/// `all` is every topic file loaded: a heading-shaped entry that is the
/// heading of a current decision of another topic file is refused as naming
/// a decision of another file (`derive_finding`).
///
/// The `parse` findings of a topic file: its shape, a `###` line that is no
/// question, a section's shape, and what its parse found (178a232e A point
/// 1: an option with no decision sentence or more than one, a duplicate id
/// line, an answer naming a missing option). The one list every reader of
/// the `parse` rule prints, in this order.
pub fn parse_findings(rel: &str, rf: &RecordFile) -> Vec<Finding> {
    let mut out: Vec<Finding> = rf
        .shape_errors
        .iter()
        .map(|e| Finding::said(Class::Parse, msg::shape_error(rel, e)))
        .collect();
    for i in record::non_question_heading_lines(&rf.lines) {
        out.push(Finding::said(
            Class::Parse,
            msg::heading_not_a_question(rel, i + 1),
        ));
    }
    for (si, sec) in rf.sections.iter().enumerate() {
        for e in &sec.shape_errors {
            out.push(
                Finding::said(
                    Class::Parse,
                    msg::section_shape_error(rel, &sec.heading_text, e),
                )
                .in_section(si),
            );
        }
    }
    for (si, sec) in rf.sections.iter().enumerate() {
        for e in &sec.parse_errors {
            out.push(
                Finding::said(
                    Class::Parse,
                    msg::check_parse_error(rel, &sec.heading_text, e),
                )
                .in_section(si),
            );
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub fn record_refusals(
    rel: &str,
    rf: &RecordFile,
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    names: &HashMap<String, String>,
    all: &[RecordFile],
    replaced: &std::collections::HashSet<String>,
    blame: &crate::blame::Index,
    led: &Ledger,
) -> Vec<Finding> {
    let mut out: Vec<Finding> = parse_findings(rel, rf);

    // Per "Does the tool stop loaded phrases in a question?": a loaded
    // phrase refuses before any write -- a heading with any
    // tell (loaded opening, "and"/"or", provenance, code state, or a date),
    // or a context prose line with a provenance, code-state, or date tell.
    // A heading with any ledger state is skipped (asked text is held by
    // hash, answered text is frozen): it can never be edited, so refusing
    // on it would block this file forever once the list grows to include a
    // word it already carried.
    for (si, sec) in rf.sections.iter().enumerate() {
        if !status::open_to_ask_text(led, rf, sec) {
            continue;
        }
        for (line, phrase, what) in tells::heading_tells(sec.start, &sec.heading_text) {
            out.push(
                Finding::said(
                    Class::Tells,
                    msg::tell(rel, line, &sec.heading_text, &phrase, what),
                )
                .in_section(si),
            );
        }
    }
    // Per "Does ask refuse an option over a word limit?" and "Does an option
    // under a heading that opens with Does hold one word?": the same length
    // rules `approve` enforces, found before approval in a question not yet asked.
    out.extend(in_sections(Class::Tells, length_found(rel, rf, led)));
    // The other two body rules `approve` enforces, found the same way: a loaded
    // phrase in a body, and a question that pairs no term.
    out.extend(in_sections(Class::Tells, body_tell_found(rel, rf, led)));
    out.extend(in_sections(
        Class::Tells,
        no_term_found(rel, rf, terms_list, led),
    ));
    // The three shapes of a depends entry approve stops, found the same way.
    for said in depends_shape_refusals(rel, rf, terms_list, phrase_lookup, led) {
        out.push(Finding::said(Class::Plan, said));
    }
    {
        let (cs, ce) = rf.context_range;
        for (line, phrase, what) in tells::prose_tells_in(&rf.lines, cs, ce) {
            out.push(Finding::said(
                Class::Tells,
                msg::tell(rel, line, "context", &phrase, what),
            ));
        }
    }

    // Per "What stops a question that has no depends line?": every section, skeleton or
    // not, needs at least one `depends:` line naming what the question
    // stands on, before anything about it is resolved or written.
    for (si, sec) in rf.sections.iter().enumerate() {
        if sec.depends.is_empty() {
            out.push(
                Finding::said(Class::Plan, msg::no_depends_line(rel, &sec.heading_text))
                    .in_section(si),
            );
        }
    }

    // Resolve every entry (per "What stops a depends entry that resolves to nothing?" and "What
    // does a reference in a topic file carry?"). An `Id` entry is kept as written and must already
    // be a heading id in this file, a non-retired term id, or, on a closed question, a current
    // decision. A `Text` entry ending in `?` is a heading reference, whose dangling/cycle/ self-dep
    // is `derive_tiers`'s single error (computed at parse time into `rf.derive_error`); any other
    // `Text` entry must resolve to exactly one non-retired term. A `Text` entry (of either shape)
    // found inside a section whose latest ledger state is `answered`/`refused`/`superseded` refuses
    // instead of being silently rewritten (per "Does a decision change after it is answered?"):
    // that section is frozen -- its decision is already closed, one way or another -- and an edit
    // there is the agent's to remove, not approve's to carry forward.
    for (si, sec) in rf.sections.iter().enumerate() {
        let frozen = state::is_frozen(led, sec);
        for d in &sec.depends {
            if let (true, record::Dep::Text { text, .. }) = (frozen, d) {
                let st = status::question_status(led, sec);
                out.push(
                    Finding::said(
                        Class::PlanFrozen,
                        msg::frozen_depends_text(rel, &sec.heading_text, st, text),
                    )
                    .in_section(si),
                );
                continue;
            }
            if let record::Dep::Text { text, .. } = d {
                if text.trim().ends_with('?') {
                    // Dangling/cycle/self-dep is reported once, below, via
                    // `rf.derive_error`.
                    continue;
                }
            }
            // A dead section (refused, superseded, withdrawn) is history:
            // what its line stood on when it was live is not resolved
            // against the current decisions again, the same exemption
            // `resolve_dep` gives a retired term.
            if state::is_dead(led, sec) {
                continue;
            }
            if let Err(e) = depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, led) {
                // What the identifier names, for the refusal only: a term
                // (a retired one too), else the heading of a question in
                // any state.
                let name = match &e {
                    depends::DepError::NotCurrentDecision(id)
                    | depends::DepError::OtherFile(id)
                    | depends::DepError::RetiredTermId(id) => terms_list
                        .iter()
                        .find(|t| t.id.as_deref() == Some(id.as_str()))
                        .map(|t| t.name.as_str())
                        .or_else(|| names.get(id).map(String::as_str)),
                    _ => None,
                };
                out.push(
                    Finding::said(
                        Class::Plan,
                        depends::dep_refusal_text(rel, &sec.heading_text, &e, name),
                    )
                    .in_section(si),
                );
            }
        }
    }
    // The rules on a question's answer or refusal (`closed_findings`):
    // category "answers", which `check` prints from `check_answers` and
    // forwards no line of from here, so it prints once.
    out.extend(closed_findings(rel, rf, led));
    out.extend(derive_finding(rel, rf, cross_file_headings, led));
    out.extend(cap_finding(rel, rf, replaced, led));
    out.extend(depends_line_cap_findings(rel, rf, led));

    out.extend(supersedes_refusals(
        rel,
        rf,
        cross_file_headings,
        names,
        all,
        led,
        None,
    ));

    // Per "What does a reference in a topic file carry?": an
    // `unrelated:` entry names a current decision, a term, another question
    // of the file or a dropped question.
    for said in crate::plan::unrelated_refusals(rel, rf, terms_list, cross_file_headings, led) {
        out.push(Finding::said(Class::Plan, said));
    }

    out.extend(blame_line_refusals(rel, rf, blame, led));

    out
}

/// Every refusal the `blame:` lines of `rf` earn (per "What does a question
/// carry of the blame file it came from?", 672201dd A), category `plan`,
/// judged for each question whose text can still change
/// (`status::open_to_ask_text`): an asked question that stands as asked, an
/// answered one and the rest are not judged again when their blame file
/// changes. For each entry, in line order: it names a blame file and row
/// ranges; the name or identifier is a blame file of `blame`; that file is
/// approved; each range is the range of a row of it, a `decision` row or a
/// `detail` row alike. A fault of the first two stops the judgement of its
/// entry, and the ranges of a file not yet approved wait for it. A repeated
/// entry or range earns its finding once, and is no fault of its own.
pub fn blame_line_refusals(
    rel: &str,
    rf: &RecordFile,
    blame: &crate::blame::Index,
    led: &Ledger,
) -> Vec<Finding> {
    let mut out = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if sec.blame.is_empty() || !status::open_to_ask_text(led, rf, sec) {
            continue;
        }
        let heading = sec.heading_text.as_str();
        let mut found: Vec<msg::Said> = Vec::new();
        for entry in &sec.blame {
            let written = entry.written();
            let faults: Vec<msg::Said> = if entry.ranges.is_empty() {
                vec![msg::blame_entry_without_range(rel, heading, &written)]
            } else {
                match blame.named(&entry.file) {
                    None => vec![msg::blame_names_no_file(rel, heading, &entry.file)],
                    Some(file) if file.status != status::FileStatus::Approved => {
                        vec![msg::blame_file_awaits_approval(
                            rel,
                            heading,
                            file.rel(),
                            file.status,
                        )]
                    }
                    Some(file) => entry
                        .ranges
                        .iter()
                        .filter(|&&(from, to)| {
                            !file.rows().iter().any(|r| r.start == from && r.end == to)
                        })
                        .map(|&(from, to)| {
                            msg::blame_range_of_no_row(rel, heading, file.rel(), from, to)
                        })
                        .collect(),
                }
            };
            for fault in faults {
                if !found.iter().any(|f| f.found == fault.found) {
                    found.push(fault);
                }
            }
        }
        out.extend(
            found
                .into_iter()
                .map(|said| Finding::said(Class::Plan, said).in_section(i)),
        );
    }
    out
}

/// Refusals about a topic file's own name and its own headings, one
/// function for `approve` and `check` so the two print the same lines
/// (per "What names a topic file?"): a name
/// that is not lowercase words joined by hyphens, a
/// topic no name can carry, a name that is not the topic, and a heading
/// text used twice in the file. `topic` is the file's `# ` title when it
/// has one.
pub fn own_name_and_heading_refusals(
    rel: &str,
    topic: Option<&str>,
    sections: &[record::Section],
) -> Vec<Finding> {
    let mut out = own_name_refusals(rel, topic);
    out.extend(duplicate_heading_refusals(rel, sections));
    out
}

/// The naming half of `own_name_and_heading_refusals`.
pub fn own_name_refusals(rel: &str, topic: Option<&str>) -> Vec<Finding> {
    let mut out = Vec::new();
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let here = |said: msg::Said| {
        Finding::said(Class::Plan, said).about(Subject::Directory(rel.to_string()))
    };
    if !repo::valid_topic_file_name(name) {
        // The approve the move names is of the file under the name its topic
        // gives, when the topic gives one.
        let dir = rel
            .rsplit_once('/')
            .map(|(d, _)| format!("{}/", d))
            .unwrap_or_default();
        let named = topic
            .map(|t| format!("{}{}.md", dir, repo::expected_topic_file_stem(t)))
            .filter(|p| repo::valid_topic_file_name(p.rsplit('/').next().unwrap_or(p)))
            .unwrap_or_else(|| rel.to_string());
        out.push(here(msg::bad_topic_file_name(rel, &named)));
    } else if let Some(topic) = topic {
        let expected = repo::expected_topic_file_stem(topic);
        if !repo::valid_topic_file_name(&format!("{}.md", expected)) {
            out.push(here(msg::topic_unnameable(rel, topic)));
        } else if name.strip_suffix(".md") != Some(expected.as_str()) {
            out.push(here(msg::topic_file_name_mismatch(rel, topic)));
        }
    }
    out
}

/// The in-file duplicate heading half of `own_name_and_heading_refusals`.
pub fn duplicate_heading_refusals(rel: &str, sections: &[record::Section]) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for sec in sections {
        *counts.entry(sec.heading_text.as_str()).or_insert(0) += 1;
    }
    let mut dups: Vec<&str> = counts
        .into_iter()
        .filter(|(_, c)| *c > 1)
        .map(|(h, _)| h)
        .collect();
    dups.sort();
    for h in dups {
        out.push(
            Finding::said(Class::Plan, msg::duplicate_heading_in_file(h, rel))
                .about(Subject::Directory(rel.to_string())),
        );
    }
    out
}

/// Whether a question has force: its text can no longer change because it
/// is answered, refused, superseded, or asked and unedited since.
pub fn has_force(rf: &RecordFile, s: &record::Section, led: &Ledger) -> bool {
    state::is_frozen(led, s) || crate::view::asked_text_stands(led, rf, s)
}

/// The questions of `rf` bearing `heading` that are the copy (05141f49 A):
/// the ones with no force that `approve` would newly approve (no id, or no
/// `approved` row for the file holding the id with the question's current
/// hash). A fault of a duplicate heading belongs to them, for `check` in a
/// file and across files and for `approve` alike; an unchanged question is
/// not the copy.
pub fn copying_sections<'a>(
    rf: &'a RecordFile,
    heading: &str,
    led: &Ledger,
) -> Vec<&'a record::Section> {
    rf.sections
        .iter()
        .filter(|s| s.heading_text == heading && !has_force(rf, s, led))
        .filter(|s| {
            let hash = record::question_hash(rf, s);
            !s.id.as_ref().is_some_and(|id| {
                led.rows
                    .iter()
                    .rfind(|r| r.event == "approved" && &r.id == id)
                    .is_some_and(|r| r.hash == hash && r.detail == rf.rel)
            })
        })
        .collect()
}

/// A heading of `rf` that also stands in another topic file of `all`
/// (23f21671 A), one line per heading with the message `check` prints. Every
/// section of every other file counts, answered and superseded ones too,
/// the same set `check` compares. Only a copying question (`copying_sections`)
/// is refused for it; an unchanged one is not.
pub fn cross_file_heading_refusals(
    rel: &str,
    rf: &RecordFile,
    all: &[RecordFile],
    led: &Ledger,
) -> Vec<Finding> {
    let mut heads: Vec<&str> = rf
        .sections
        .iter()
        .map(|s| s.heading_text.as_str())
        .collect();
    heads.sort();
    heads.dedup();
    let mut out = Vec::new();
    for h in heads {
        let mut paths: Vec<&str> = all
            .iter()
            .filter(|o| o.rel != rel && o.sections.iter().any(|s| s.heading_text == h))
            .map(|o| o.rel.as_str())
            .collect();
        if paths.is_empty() || copying_sections(rf, h, led).is_empty() {
            continue;
        }
        paths.push(rel);
        paths.sort();
        paths.dedup();
        out.push(
            Finding::said(
                Class::Plan,
                msg::duplicate_heading_across_files(h, paths[0], paths[1]),
            )
            .about(Subject::Directory(rel.to_string())),
        );
    }
    out
}

/// A current (answered, not superseded) decision id now named by no topic
/// file's heading at all, across every topic file in the repo (`records`):
/// `approve` refuses on it rather than silently withdrawing it the way an
/// asked/refused id missing from every file is (that withdrawal stays
/// `do_approve_record`'s own bookkeeping, not a refusal, since striking a
/// question that was never answered loses nothing). Global, so computed
/// once per run rather than once per file -- `do_approve_record` calls it
/// once regardless of which file was named; `check` likewise once for the
/// whole repo. Shared with `check`'s `ids` category.
pub fn missing_answered_refusals(records: &[RecordFile], led: &Ledger) -> Vec<Finding> {
    missing_answered_refusals_except(records, led, &HashSet::new())
}

/// `missing_answered_refusals` leaving out the ids in `skip`: `check` skips
/// the ids of a removed file awaiting removal, which its `ids` category
/// names with the file's removal move instead. `approve` skips none.
pub fn missing_answered_refusals_except(
    records: &[RecordFile],
    led: &Ledger,
    skip: &HashSet<String>,
) -> Vec<Finding> {
    let mut present_ids: HashSet<String> = HashSet::new();
    for rf in records {
        for sec in &rf.sections {
            if let Some(id) = &sec.id {
                present_ids.insert(id.clone());
            }
        }
    }

    let state_ids = led.state_ids();

    let mut out = Vec::new();
    for id in &state_ids {
        if present_ids.contains(id) || skip.contains(id) {
            continue;
        }
        if let Some(r) = led.latest_state(id) {
            if r.event == "answered" {
                let heading = led
                    .latest_asked(id)
                    .map(|a| a.detail.clone())
                    .unwrap_or_else(|| id.clone());
                out.push(Finding::said(
                    Class::Ids,
                    msg::answered_id_missing(id, &heading),
                ));
            }
        }
    }
    out
}
