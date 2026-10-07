//! Every rule on the records, judged once (per "Does a command refuse what
//! check passes?", fe14991d A): each rule is a function here returning
//! `Finding`s, and every command that acts on a rule reads the same
//! findings. `approve` refuses the findings about the file it approves,
//! `ask` the findings about the questions it asks, `check` prints them all
//! (a finding in a draft as a note, per "What does check do with a fault on
//! a file awaiting approval?"), and `show` and `plan` name their moves. A
//! rule on the input of one command (the words `ban` takes, the letter and
//! version `answer` takes) stays with that command.
use crate::depends::{self, DepTarget};
use crate::ledger::Ledger;
use crate::moves::{self, Move};
use crate::msg::{self, Next};
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state;
use crate::status;
use crate::tells;
use crate::terms;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

/// What a finding is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// A topic file, by its path.
    TopicFile(String),
    TermsFile,
    /// A blame file, by its path.
    BlameFile(String),
    /// The records as a whole: a ledger id no file holds, for one.
    Records,
}

/// One rule a thing in the records breaks, tagged with the rule's name (the
/// `check` category under which the identical text is filed: `parse`,
/// `tells`, `plan`, `plan_frozen`, `ids`, or `answers`, which `check` prints
/// from `check_answers` and does not forward from here). `approve` prefixes
/// each message with `approve: `, `check` with `<rule>: `, so the same line
/// reads either way.
#[derive(Debug, Clone)]
pub struct Finding {
    pub subject: Subject,
    pub rule: &'static str,
    /// The finding whole, with the move that clears it.
    pub message: String,
    /// The questions of the file the finding is about, by index; empty for
    /// a finding about the file as a whole (its prose, headings, tiers).
    /// `check` reads it to tell a finding in text that can still change from
    /// one in an asked or answered question.
    pub sections: Vec<usize>,
    /// What the finding waits for, when its only move is an answer
    /// elsewhere first: the message without its waiting tail.
    waits_for: Option<String>,
}

impl Finding {
    /// A finding about the records; `about` names its subject.
    pub fn new(rule: &'static str, message: String) -> Finding {
        Finding { subject: Subject::Records, rule, message, sections: Vec::new(), waits_for: None }
    }
    pub fn about(mut self, subject: Subject) -> Finding {
        self.subject = subject;
        self
    }
    pub fn in_section(mut self, index: usize) -> Finding {
        self.sections.push(index);
        self
    }
    pub fn in_sections(mut self, indexes: Vec<usize>) -> Finding {
        self.sections = indexes;
        self
    }
    /// A finding nobody moves on until `what` is answered elsewhere.
    pub fn waiting(mut self, what: String) -> Finding {
        self.waits_for = Some(what);
        self
    }
    pub fn waits_for(&self) -> Option<&str> {
        self.waits_for.as_deref()
    }
    /// The move that clears this finding.
    pub fn mv(&self) -> Move {
        moves::of_finding(self)
    }
}

/// Tags each finding of `fs` as about the topic file `rel`, a finding about
/// the records left as it is.
fn about_topic_file(fs: Vec<Finding>, rel: &str) -> Vec<Finding> {
    fs.into_iter()
        .map(|f| if f.subject == Subject::Records { f.about(Subject::TopicFile(rel.to_string())) } else { f })
        .collect()
}

/// Strings of one rule, each a finding about the topic file `rel`.
fn found(rule: &'static str, rel: &str, messages: Vec<String>) -> Vec<Finding> {
    messages.into_iter().map(|m| Finding::new(rule, m).about(Subject::TopicFile(rel.to_string()))).collect()
}

/// A path of the shape of a topic file: `adrdom/decisions/<name>.md`.
pub(crate) fn is_topic_file_path(rel: &str) -> bool {
    rel.strip_prefix(&format!("{}/", repo::DECISIONS_DIR))
        .is_some_and(|name| !name.contains('/') && name.ends_with(".md"))
}

/// The plan hash of a file with no question; the same value the context
/// hash of a file with no prose has. What approving a file's removal writes.
pub(crate) fn empty_plan_hash() -> String {
    record::derive_file_hash(&[])
}

/// Every topic file the ledger holds an approved plan for that is absent
/// from the content judged (`exists`) and whose latest approved plan is not
/// the empty one: a file removed, its removal awaiting the owner's approval
/// (per "What may leave a topic file by a plan change?").
pub fn removal_awaiting(led: &Ledger, exists: &dyn Fn(&str) -> bool) -> Vec<String> {
    let rels: BTreeSet<&str> = led
        .rows
        .iter()
        .filter(|r| r.event == "approved" && r.id == "-" && is_topic_file_path(&r.detail))
        .map(|r| r.detail.as_str())
        .collect();
    let empty = empty_plan_hash();
    rels.into_iter()
        .filter(|rel| led.latest_for_path("approved", rel).is_some_and(|r| r.hash != empty))
        .filter(|rel| !exists(rel))
        .map(|rel| rel.to_string())
        .collect()
}

/// What a removed topic file still holds that no live file does.
pub struct RemovalHolds {
    /// Every id the file held with its heading, in file order.
    pub held: Vec<(String, String)>,
    /// What keeps the removal from being approved: a current decision, an
    /// asked question, a heading of the approved plan never asked.
    pub holds: Vec<moves::RemovalHold>,
}

/// The committed version of `rel` that holds the file's latest approved
/// plan: walks the commits that touched the path newest first, skipping
/// those where it is absent (a deletion), and stops at the first version
/// whose plan matches the latest file-level `approved` row. When no
/// committed version matches (the latest approval was never committed), falls
/// back to the newest committed version that has the file. That fallback can
/// name a heading already struck by an uncommitted approval; the move stays
/// executable: restore the file, strike the heading again, approve.
pub(crate) fn latest_approved_version(root: &Path, rel: &str, led: &Ledger) -> Option<RecordFile> {
    // An approved removal is the empty plan: no version to read.
    if led.latest_for_path("approved", rel).is_some_and(|r| r.hash == empty_plan_hash()) {
        return None;
    }
    // A bound on the versions read; the match is almost always the first few.
    const WALK_LIMIT: usize = 50;
    let log = repo::git(root, &["log", "--format=%H", "--", rel])?;
    let mut newest: Option<RecordFile> = None;
    for commit in log.lines().filter(|l| !l.is_empty()).take(WALK_LIMIT) {
        let Some(text) = repo::git(root, &["show", &format!("{}:{}", commit, rel)]) else {
            continue;
        };
        let old = record::build(root.join(rel), rel.to_string(), record::split_lines(&text));
        if state::plan_approved_hashes(led, rel, &record::question_hashes(&old)) {
            return Some(old);
        }
        if newest.is_none() {
            newest = Some(old);
        }
    }
    newest
}

/// What the topic file `rel`, gone from the content judged, held: the ids
/// of its latest approved committed plan (see `latest_approved_version`) and the ids whose latest approved row
/// names it and that have a ledger state, minus the ids in `present_ids`. Shared by `approve`, `check`
/// and `show`, so the three agree on whether a removal passes.
pub fn removal_holds(root: &Path, rel: &str, led: &Ledger, present_ids: &HashSet<String>) -> RemovalHolds {
    let mut held: Vec<(String, String)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    if let Some(old) = latest_approved_version(root, rel, led) {
        for sec in &old.sections {
            if let Some(id) = &sec.id {
                if seen.insert(id.clone()) {
                    held.push((id.clone(), sec.heading_text.clone()));
                }
            }
        }
    }
    for r in led.rows.iter().filter(|r| r.event == "approved" && r.id != "-" && r.detail == rel) {
        // approve writes a per-question row only when the question's hash
        // changed, so a heading approved, never asked, then struck keeps a
        // row the ledger cannot tell from a live one. Only an id with a
        // ledger state counts from the ledger; an unasked heading counts
        // from the file's latest approved committed plan alone.
        if led.latest_state(&r.id).is_some()
            && seen.insert(r.id.clone())
            && led.latest_for_id("approved", &r.id).is_some_and(|l| l.detail == rel)
        {
            let heading = led.latest_asked(&r.id).map(|a| a.detail.clone()).unwrap_or_else(|| r.id.clone());
            held.push((r.id.clone(), heading));
        }
    }
    held.retain(|(id, _)| !present_ids.contains(id));

    let mut holds = Vec::new();
    for (id, heading) in &held {
        let status = match led.latest_state(id).map(|r| r.event.as_str()) {
            None => status::QuestionStatus::NotAsked,
            Some("asked") => status::QuestionStatus::Asked,
            Some("answered") => status::QuestionStatus::Answered,
            _ => continue,
        };
        holds.push(moves::RemovalHold { id: id.clone(), heading: heading.clone(), status });
    }
    RemovalHolds { held, holds }
}

/// Every refusal `adrdom/terms.md` earns under steps 1, 3, 4 and 5 of
/// `do_approve_terms` (shape errors, circular definitions, a dependent's
/// pairing that no longer holds, banned words in the file's own text),
/// plus a definition running past two sentences
/// (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76, not one of the numbered steps
/// above), collected all at once rather than returned at the first --
/// `do_approve_terms` prints every one and writes nothing on any refusal;
/// `check` reports the same list, each line prefixed `terms: ` instead of
/// `approve: `, minus the approval-state lines that only make sense after a
/// real write (`already approved`/`approved`, the definition-changed
/// notes), which stay `check`'s own.
///
/// `tf` must already have every id minted in memory (step 2, see
/// `terms::mint_ids`/`terms::minted_copy`): `do_approve_terms` mints
/// straight into the file it is about to write before calling this; `check`
/// -- which never writes -- calls this on a `terms::minted_copy` instead.
/// Either way, a cycle or a moved/retired dependent's pairing in a terms
/// file that has never been approved, and so carries no ids on disk, is
/// caught the same way both callers look for it (`terms::cycles` and
/// `terms::phrase_lookup` only see a term that has an id).
///
/// Returns every refusal message (`adrdom/terms.md: ...`, no caller
/// prefix) plus each term's freshly resolved definition pairs
/// (`terms::definition_pairs`, in `tf.terms` order), computed once here
/// and reused by `do_approve_terms`'s write step so it is never resolved
/// twice.
///
/// `records` is every topic file in the repo, already parsed by the
/// caller: `do_approve_terms` parses them once itself and reuses
/// that same parse for its step-7 report; `check` passes the same
/// `RecordFile`s it built for its own record checks. Neither re-reads a
/// topic file from disk just to compute a dependent's terms line here.
///
/// `source` is who the ban list (step 5) is read through: `approve` passes
/// `Source::WorkingTree` (it always reads `adrdom/terms.md` straight off
/// disk), `check` passes whichever source it is judging (per "Does check at
/// a commit read the staged files?"), so a staged edit to the ban list is
/// judged, not whatever is left on disk. `scan_banned` is false only for
/// `check`'s own call: `check_banned` already scans `adrdom/terms.md` as a
/// tracked file, so step 5 would otherwise report the same hit twice, once
/// prefixed `banned:` and once prefixed `terms:`; `approve` (the one caller
/// that never runs `check_banned` itself) always passes `true`, since step 5
/// is its only banned-word gate on the file.
pub fn terms_refusals(
    root: &Path,
    records: &[RecordFile],
    tf: &terms::TermsFile,
    led: &Ledger,
    source: &repo::Source,
    scan_banned: bool,
    approved_plans: &std::collections::HashSet<String>,
) -> (Vec<String>, Vec<Vec<(String, String)>>) {
    let mut refusals: Vec<String> = Vec::new();

    // Per "Does approve accept a terms edit that changes more than one
    // entry?": more than one entry added, changed or removed since the
    // latest approval is refused, here for `approve` and `check` alike.
    let differing = state::terms_edit_entries(led, tf);
    if differing.len() > 1 {
        let approved_before = led.latest_for_path("terms", crate::repo::TERMS_PATH).is_some();
        refusals.push(msg::terms_edit_too_wide(&differing, approved_before));
    }

    // Step 1 (shape): "missing id" cannot appear here, since `tf` is
    // already minted.
    for e in &tf.errors {
        refusals.push(msg::terms_shape_error(e));
    }

    // Step 3: definition pairings; a self-pairing or any cycle refuses.
    // Every cycle is reported, not just the first.
    for cycle in terms::cycles(tf) {
        refusals.push(msg::circular_definition(&msg::cycle_chain(&cycle)));
    }

    // Fresh definition pairs for every term (never re-derived from a stale
    // terms line in the file). Computed once and reused for the dependents
    // check below and, by the caller, for the write step.
    let pairs_per_entry: Vec<Vec<(String, String)>> =
        (0..tf.terms.len()).map(|i| terms::definition_pairs(tf, i)).collect();

    // Every current decision's terms line, every current decision's
    // `depends:` line (once approved, an id -- possibly a term's), and
    // every non-retired term's recorded pairing, is a dependent -- the
    // widest reach of `state::dependent_entries`' three readers.
    // Asked questions and not-yet-asked headings of approved plans (in the
    // files of `approved_plans`, read as written on disk) name terms too.
    let mut dependents: Vec<state::DependentEntry> =
        state::dependent_entries(records, led, tf, true, true);
    dependents.extend(state::unsettled_entries(records, led, tf, approved_plans));

    // Step 4: every pair on a dependent must still hold in the new file.
    // `lookup` answers "what does this exact word/phrase resolve to" for
    // every dependent pair, built once rather than once per pair.
    let lookup = terms::phrase_lookup(&tf.terms);
    for dep in &dependents {
        let label = dep.source.label();
        for (word, uuid) in &dep.pairs {
            let Some(target) = tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str()))
            else {
                let word = dep.from_word.then_some(word.as_str());
                refusals.push(msg::term_removed_but_carried(uuid, word, &label, dep.source.decision_ref()));
                continue;
            };
            if target.retired {
                // A decision dependent: the agent's move is to write a new
                // question that supersedes it, not the owner's to make on
                // the spot. A term dependent (its own definition still
                // carries the retired word) is never superseded (per "What
                // holds a retire of a term?"): the agent rewords the
                // definition and the owner re-approves.
                let next = match &dep.source {
                    state::DependentSource::Decision { id, heading, .. } => {
                        msg::term_retired_decision_next(&target.name, id, heading)
                    }
                    state::DependentSource::Asked { rel, heading } => {
                        msg::term_retired_asked_next(heading, &target.name, rel)
                    }
                    state::DependentSource::PlanHeading { rel, .. } => {
                        msg::term_retired_heading_next(&target.name, rel)
                    }
                    state::DependentSource::Term { .. } => msg::term_reword_and_approve(),
                };
                refusals.push(msg::term_retired_but_carried(&target.name, &label, next));
                continue;
            }
            // A `depends:`-line dependent's pair is an id read straight off
            // the entry, never a resolved word -- there is no word to
            // drift, only the target's removal/retirement above, so this
            // check is skipped for it (`from_word` is false).
            let is_decision_or_term = matches!(
                dep.source,
                state::DependentSource::Decision { .. } | state::DependentSource::Term { .. }
            );
            if dep.from_word && is_decision_or_term {
                let still_resolves = lookup.get(word) == Some(uuid);
                if !still_resolves {
                    // A decision dependent here is always current, since
                    // only `is_current` sections become one above -- so it
                    // can never be re-asked (per "Does a decision change
                    // after it is answered?"; "Does a refusal name the
                    // command that clears it?" means a refusal about an
                    // answered question never tells the agent to re-ask it,
                    // since that can't clear it). What moved is
                    // adrdom/terms.md, not the decision's own text, so the
                    // fix is there: restore the wording so the word
                    // resolves to this term again, or, when the move is
                    // meant, supersede the decision with a new question
                    // that pairs the new wording. A term dependent (its own
                    // definition still carries the drifted word) is
                    // reworded by the agent and the owner re-approves, same
                    // as the retired case above.
                    let next = match dep.source.decision_id() {
                        Some(id) => Next::agent(format!(
                            "restores the wording in adrdom/terms.md so \"{}\" still names \"{}\", or writes a question that supersedes {}",
                            word, target.name, id
                        )),
                        None => msg::term_reword_and_approve(),
                    };
                    refusals.push(msg::word_moved_off_term(word, &target.name, &label, next));
                }
            }
        }
    }

    // Step 5: banned hits in terms.md's own text, from the single ban list
    // (skipped for `check`'s call -- see `scan_banned` above -- since
    // `check_banned` already scans this same file as a tracked file).
    if scan_banned {
        let banned_entries = crate::banned::read_entries_via(root, source);
        if !banned_entries.is_empty() {
            let hash_map: std::collections::HashMap<String, Option<String>> =
                banned_entries.into_iter().collect();
            let text = tf.lines.join("\n");
            for hit in crate::banned::hits(&text, &hash_map, &tf.terms) {
                let suffix = msg::term_suffix(hit.term.as_deref());
                refusals.push(msg::terms_banned_word(hit.line, &suffix));
            }
        }
    }

    // Every term's own definition is one or two sentences
    // (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76; not one of the spec's
    // numbered steps). Checked on every entry, retired or not, the same
    // way the shape checks in `terms::parse` run on every entry regardless
    // of `retired`.
    for t in &tf.terms {
        let count = terms::sentence_count(&t.definition);
        if count > 2 {
            refusals.push(msg::definition_too_long(&t.name, count));
        }
    }

    (refusals, pairs_per_entry)
}

/// The length refusals (`tells::length_tells`, the one function `ask` reads
/// too) of every question of `rf` open to ask (`status::open_to_ask_text`:
/// no state, or asked and edited since). Answered text is never read (per
/// "Does the word limit read the text of an answered question?").
pub fn length_refusals(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<String> {
    messages(length_found(rel, rf, led))
}

fn length_found(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if !status::open_to_ask_text(led, rf, sec) {
            continue;
        }
        let (s, e) = record::hash_range(&rf.lines, sec);
        for (line, found, kind) in tells::length_tells(&rf.lines, s, e, &sec.heading_text) {
            out.push((i, msg::too_long(rel, line, &sec.heading_text, &kind, &found)));
        }
    }
    out
}

/// The messages of `found`, the question each is about left out.
fn messages(found: Vec<(usize, String)>) -> Vec<String> {
    found.into_iter().map(|(_, m)| m).collect()
}

/// `found` as findings of `rule`, each about its question.
fn in_sections(rule: &'static str, found: Vec<(usize, String)>) -> Vec<Finding> {
    found.into_iter().map(|(i, m)| Finding::new(rule, m).in_section(i)).collect()
}

/// The loaded phrases (`tells::prose_tells_in`, the scan `ask` reads too)
/// in the body of every question of `rf` open to ask, below its heading,
/// judged as its heading is (per "Does the tool stop loaded phrases in a
/// question?"): the
/// one function `check`, `approve` and `plan`
/// read, found before approval what `ask` refuses.
pub fn body_tell_refusals(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<String> {
    messages(body_tell_found(rel, rf, led))
}

fn body_tell_found(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<(usize, String)> {
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

/// One `pairs_no_term` message per question of `rf` not yet asked, with its
/// options, whose text pairs no term (`terms::resolve`, the pairing `ask`
/// writes): the one function `check`, `approve` and `plan` read, found
/// before approval what `ask` refuses. A heading with no body is no question
/// yet.
pub fn no_term_refusals(rel: &str, rf: &RecordFile, terms_list: &[terms::Term], led: &Ledger) -> Vec<String> {
    messages(no_term_found(rel, rf, terms_list, led))
}

fn no_term_found(rel: &str, rf: &RecordFile, terms_list: &[terms::Term], led: &Ledger) -> Vec<(usize, String)> {
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
/// what `ask` refuses. Cross-file targets carry no set before they are
/// answered, so only a same-file target is read.
pub fn letter_refusals(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<String> {
    messages(letter_found(rel, rf, led))
}

/// A pair is judged when the dependent is open to ask, or when its target
/// has no letter answer yet: a target edited and asked again can lose the
/// option an asked dependent's set names, which `ask` refuses.
fn letter_found(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        let open = status::open_to_ask_text(led, rf, sec);
        for d in &sec.depends {
            let Some(record::AnswerSet::Letters(letters)) = d.set() else {
                continue;
            };
            let target = match d {
                record::Dep::Id { id, .. } => rf.sections.iter().find(|s| s.id.as_deref() == Some(id.as_str())),
                record::Dep::Text { text, .. } => {
                    let t = text.trim();
                    if t.ends_with('?') { rf.sections.iter().find(|s| s.heading_text.trim() == t) } else { None }
                }
            };
            if let Some(ts) = target.filter(|ts| open || state::tier_unresolved(ts)) {
                out.extend(depends::bad_letters(rel, &sec.heading_text, ts, letters).into_iter().map(|m| (i, m)));
            }
        }
    }
    out
}

/// The three shapes of a `depends:` entry `approve` has stopped, for every
/// question of `rf` not yet asked and with a body (per "Does a depends entry
/// on a question in its file carry an answer set?"): a set on an entry that
/// names a term, an entry on a question of `rf` with no answer that carries
/// no set, and a set on an answered question that `ask` did not freeze. The
/// one function `check`, `approve` and `plan` read, found before approval in
/// text that can still change.
pub fn depends_shape_refusals(
    rel: &str,
    rf: &RecordFile,
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    cross_file_headings: &HashMap<String, Vec<(String, String)>>,
    all: &[RecordFile],
    led: &Ledger,
) -> Vec<String> {
    let mut out = Vec::new();
    for sec in &rf.sections {
        if !status::open_to_ask(led, rf, sec) {
            continue;
        }
        for d in &sec.depends {
            let Ok(Some(target)) =
                depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, cross_file_headings, led)
            else {
                continue;
            };
            let (target_section, in_file) = match &target {
                DepTarget::Heading(idx) => (Some(&rf.sections[*idx]), true),
                DepTarget::Decision(id) => (
                    all.iter().find_map(|r| r.sections.iter().find(|s| s.id.as_deref() == Some(id.as_str()))),
                    false,
                ),
                DepTarget::Term(_) => (None, false),
            };
            // An answer set names the letters of another question: a term has
            // no options (per "Does a depends entry on a question in its file
            // carry an answer set?").
            if matches!(target, DepTarget::Term(_)) && d.set().is_some() {
                out.push(msg::set_on_term(rel, &sec.heading_text));
                continue;
            }
            let Some(ts) = target_section else { continue };
            match d.set() {
                None if in_file && ts.answer_letter().is_none() => {
                    out.push(msg::missing_answer_set(rel, &sec.heading_text, &ts.heading_text))
                }
                // A set approved before its target was answered stays (per
                // "Does a depends entry on a question in its file carry an
                // answer set?"): a set with no `assumes` row of the target
                // for this entry, written when the target was asked, is
                // refused.
                Some(_) if ts.answer_letter().is_some() && !set_frozen_at_ask(led, sec, d, ts) => {
                    out.push(msg::set_on_answered_question(rel, &sec.heading_text, &ts.heading_text))
                }
                _ => {}
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
/// (`missing_answered_refusals`, below). Shared with `check` and `ask`
/// (178a232e A point 1, one function for all three, mirroring
/// `terms_refusals`): `approve` prefixes each message `approve: `,
/// `check` prefixes it `<category>: `, `ask` prefixes the "parse" slice
/// `ask: `, so the same line reads identically wherever it is printed.
///
/// `terms_list`/`phrase_lookup` are supplied by the caller rather than
/// loaded here: `approve` passes its own already-approved terms (gated by
/// `require_terms` before this is ever called), `check` a copy with every
/// id minted in memory so a real term in a terms.md that has never been
/// approved still resolves instead of drowning this in "which is no
/// term" refusals. `cross_file_headings` is likewise supplied once per
/// command run (`depends::cross_file_heading_index`), threaded straight
/// into `resolve_dep`.
///
/// `names` is every heading by its id, of any state and any topic file
/// (`view::all_heading_texts`), built once per command run by the caller:
/// a refusal that names an identifier names the question beside it.
/// `all` is every topic file loaded: a heading-shaped entry that names no
/// current decision but a question of another topic file that is not
/// answered (or is refused) is refused with the state the tool holds.
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
    led: &Ledger,
) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();

    for e in &rf.shape_errors {
        out.push(Finding::new("parse", msg::shape_error(rel, e)));
    }
    for i in record::non_question_heading_lines(&rf.lines) {
        out.push(Finding::new("parse", msg::heading_not_a_question(rel, i + 1)));
    }
    for (si, sec) in rf.sections.iter().enumerate() {
        for e in &sec.shape_errors {
            out.push(Finding::new("parse", msg::section_shape_error(rel, &sec.heading_text, e)).in_section(si));
        }
    }
    // 178a232e A point 1: an option with no decision sentence (or more than
    // one), a duplicate id line, and an answer naming a missing option were
    // parse errors only `check` read; now every reader of the shared "parse"
    // category (`ask`, `approve`, `check`) refuses on them the same way it
    // already refuses on a shape error.
    for (si, sec) in rf.sections.iter().enumerate() {
        for e in &sec.parse_errors {
            out.push(Finding::new("parse", msg::check_parse_error(rel, &sec.heading_text, e)).in_section(si));
        }
    }

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
            out.push(Finding::new("tells", msg::tell(rel, line, &sec.heading_text, &phrase, what)).in_section(si));
        }
    }
    // Per "Does ask refuse an option over a word limit?" and "Does an option
    // under a heading that opens with Does hold one word?": the same length
    // rules `ask` enforces, found before approval in a question not yet asked.
    out.extend(in_sections("tells", length_found(rel, rf, led)));
    // The other two body rules `ask` enforces, found the same way: a loaded
    // phrase in a body, and a question that pairs no term.
    out.extend(in_sections("tells", body_tell_found(rel, rf, led)));
    out.extend(in_sections("tells", no_term_found(rel, rf, terms_list, led)));
    // The three shapes of a depends entry approve stops, found the same way.
    for message in depends_shape_refusals(rel, rf, terms_list, phrase_lookup, cross_file_headings, all, led) {
        out.push(Finding::new("plan", message));
    }
    {
        let (cs, ce) = rf.context_range;
        for (line, phrase, what) in tells::prose_tells_in(&rf.lines, cs, ce) {
            out.push(Finding::new("tells", msg::tell(rel, line, "context", &phrase, what)));
        }
    }

    // Per "What does a depends line name?": every section, skeleton or
    // not, needs at least one `depends:` line naming what the question
    // stands on, before anything about it is resolved or written.
    for (si, sec) in rf.sections.iter().enumerate() {
        if sec.depends.is_empty() {
            out.push(Finding::new("plan", msg::no_depends_line(rel, &sec.heading_text)).in_section(si));
        }
    }

    // Resolve every entry (per "What does a depends line name?" and "Does a
    // reference in a topic file carry the identifier of what it names?").
    // An `Id` entry is kept as written and must already be a heading id in
    // this file, a non-retired term id, or a current decision. A `Text`
    // entry ending in `?` is a heading reference, whose dangling/cycle/
    // self-dep is `derive_tiers`'s single error (computed at parse time
    // into `rf.derive_error`); any other `Text` entry must resolve to
    // exactly one non-retired term. A `Text` entry (of either shape) found
    // inside a section whose latest ledger state is
    // `answered`/`refused`/`superseded` refuses instead of being silently
    // rewritten (per "Does a decision change after it is answered?"): that
    // section is frozen -- its decision is already closed, one way or
    // another -- and an edit there is the agent's to remove, not approve's
    // to carry forward.
    for (si, sec) in rf.sections.iter().enumerate() {
        let frozen = state::is_frozen(led, sec);
        // A frozen section whose text (the depends line included) is no
        // longer what the ledger holds: `check` fails it right after, so
        // approve refuses it before writing anything. A `Text` entry in
        // it keeps its own refusal below.
        if frozen
            && !sec.depends.iter().any(|d| matches!(d, record::Dep::Text { .. }))
            && state::frozen_text_edited(led, &rf.lines, sec)
        {
            // Category "answers": `check` prints this line from
            // `check_answers` and forwards no `answers` refusal from here,
            // so it prints once; `approve` refuses on every category.
            out.push(Finding::new("answers", msg::answered_text_changed(
                    rel,
                    &sec.heading_text,
                    sec.id.as_deref().unwrap_or(""),
                )));
        }
        for d in &sec.depends {
            if frozen && matches!(d, record::Dep::Text { .. }) {
                out.push(Finding::new("plan_frozen", msg::frozen_depends_edited(rel, &sec.heading_text)).in_section(si));
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
            if let Err(e) =
                depends::resolve_dep(d, sec, rf, terms_list, phrase_lookup, cross_file_headings, led)
            {
                // What the identifier names, for the refusal only: a term
                // (a retired one too), else the heading of a question in
                // any state.
                let name = match &e {
                    depends::DepError::NotCurrentDecision(id) | depends::DepError::RetiredTermId(id) => terms_list
                        .iter()
                        .find(|t| t.id.as_deref() == Some(id.as_str()))
                        .map(|t| t.name.as_str())
                        .or_else(|| names.get(id).map(String::as_str)),
                    _ => None,
                };
                out.push(
                    Finding::new("plan", depends::dep_refusal_text(rel, &sec.heading_text, &e, name)).in_section(si),
                );
            }
        }
    }
    out.extend(derive_finding(rel, rf, all, led));
    out.extend(cap_finding(rel, rf, replaced, led));

    out.extend(supersedes_refusals(rel, rf, cross_file_headings, names, all, led, None));

    // Per "Does every neighbour get a line under its heading?": an
    // `unrelated:` entry names a current decision, a term, another question
    // of the file or a dropped question.
    for message in crate::plan::unrelated_refusals(rel, rf, terms_list, cross_file_headings, led) {
        out.push(Finding::new("plan", message));
    }

    out
}

/// Every refusal the `supersedes:` lines of `rf` earn, the one place for the
/// supersede rules, read by `record_refusals` (so `approve`, `check` and
/// `show`), by `ask` for one candidate (`only`) and by `answer` for its own
/// question. Category `plan`. Judged: each live section with a `supersedes:`
/// line that is not frozen (an answered, refused or superseded section is
/// history); an asked section whose text still stands is judged too, since
/// `ask` froze its line and `answer` reads it. `only` narrows the judged
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
    let refuse = |index: usize, message: String| Finding::new("plan", message).in_section(index);
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
        let asked = sec
            .id
            .as_deref()
            .is_some_and(|id| state::asked_text_stands(led, id, &record::section_hash(&rf.lines, sec)));
        let mut ids = Vec::new();
        for old in &depends::supersedes_entries(&sec.supersedes, cross_file_headings, led) {
            match depends::resolve_supersedes(old, cross_file_headings, led) {
                Ok(id) => ids.push(id),
                Err(depends::SupersedesError::Ambiguous(files)) => {
                    out.push(refuse(i, msg::ask_supersedes_ambiguous(rel, &sec.heading_text, old, &files)));
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
                        out.push(refuse(i, msg::ask_supersedes_not_current(
                            rel,
                            &sec.heading_text,
                            old,
                            names.get(old).map(String::as_str),
                        )));
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
            let moment = if *asked { msg::SupersedesMoment::Asked } else { msg::SupersedesMoment::Approve };
            out.push(refuse(*i, msg::supersedes_leaves_out(rel, &sec.heading_text, &missing, moment)));
        }
        for target in ids {
            let target_name = names.get(target).map(String::as_str).unwrap_or("");
            if sec.depends.iter().any(|d| stands_on(d, target)) {
                out.push(refuse(*i, msg::stands_on_what_it_supersedes(rel, &sec.heading_text, target_name)));
            }
            for (j, other) in rf.sections.iter().enumerate() {
                let open = other.id.as_deref().map_or(true, |id| !state::is_current(led, id))
                    && !state::is_dead(led, other);
                if j != *i && open && other.depends.iter().any(|d| stands_on(d, target)) {
                    out.push(refuse(j, msg::stands_on_what_the_plan_supersedes(
                        rel,
                        &other.heading_text,
                        target_name,
                        &sec.heading_text,
                    )));
                }
            }
        }
    }
    out
}

/// The refusal for a heading-shaped `depends:` entry that names a question
/// of another topic file that is not a current decision: not answered yet
/// (unasked or asked), or refused. `None` when no other file holds an
/// unanswered or refused question under that heading.
fn waiting_on_other_file(
    rel: &str,
    section: &str,
    entry: &str,
    all: &[RecordFile],
    led: &Ledger,
) -> Option<Finding> {
    use status::QuestionStatus;
    let mut refused: Option<&str> = None;
    for other in all.iter().filter(|o| o.rel != rel) {
        for sec in other.sections.iter().filter(|s| s.heading_text.trim() == entry.trim()) {
            match status::question_status(led, sec) {
                QuestionStatus::NoBody | QuestionStatus::NotAsked | QuestionStatus::Asked => {
                    let what = msg::dep_waits_on_other_file_what(rel, section, entry, &other.rel);
                    return Some(
                        Finding::new("plan", msg::dep_waits_on_other_file(rel, section, entry, &other.rel))
                            .waiting(what),
                    );
                }
                QuestionStatus::Refused => refused = Some(other.rel.as_str()),
                _ => {}
            }
        }
    }
    refused.map(|r| Finding::new("plan", msg::dep_on_refused_in_other_file(rel, section, entry, r)))
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
) -> Vec<String> {
    let mut out = own_name_refusals(rel, topic);
    out.extend(duplicate_heading_refusals(rel, sections));
    out
}

/// The naming half of `own_name_and_heading_refusals`.
pub fn own_name_refusals(rel: &str, topic: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    let name = rel.rsplit('/').next().unwrap_or(rel);
    if !repo::valid_topic_file_name(name) {
        out.push(msg::bad_topic_file_name(rel));
    } else if let Some(topic) = topic {
        let expected = repo::expected_topic_file_stem(topic);
        if !repo::valid_topic_file_name(&format!("{}.md", expected)) {
            out.push(msg::topic_unnameable(rel, topic));
        } else if name.strip_suffix(".md") != Some(expected.as_str()) {
            out.push(msg::topic_file_name_mismatch(rel, topic, &expected));
        }
    }
    out
}

/// The in-file duplicate heading half of `own_name_and_heading_refusals`.
pub fn duplicate_heading_refusals(rel: &str, sections: &[record::Section]) -> Vec<String> {
    let mut out = Vec::new();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for sec in sections {
        *counts.entry(sec.heading_text.as_str()).or_insert(0) += 1;
    }
    let mut dups: Vec<&str> = counts.into_iter().filter(|(_, c)| *c > 1).map(|(h, _)| h).collect();
    dups.sort();
    for h in dups {
        out.push(msg::duplicate_heading_in_file(h, rel));
    }
    out
}

/// Whether a question has force: its text can no longer change because it
/// is answered, refused, superseded, or asked and unedited since.
pub fn has_force(rf: &RecordFile, s: &record::Section, led: &Ledger) -> bool {
    state::is_frozen(led, s)
        || s.id
            .as_deref()
            .is_some_and(|id| state::asked_text_stands(led, id, &record::section_hash(&rf.lines, s)))
}

/// The questions of `rf` bearing `heading` that are the copy (05141f49 A):
/// the ones with no force that `approve` would newly approve (no id, or no
/// `approved` row for the file holding the id with the question's current
/// hash). A fault of a duplicate heading belongs to them, for `check` in a
/// file and across files and for `approve` alike; an unchanged question is
/// not the copy.
pub fn copying_sections<'a>(rf: &'a RecordFile, heading: &str, led: &Ledger) -> Vec<&'a record::Section> {
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
pub fn cross_file_heading_refusals(rel: &str, rf: &RecordFile, all: &[RecordFile], led: &Ledger) -> Vec<String> {
    let mut heads: Vec<&str> = rf.sections.iter().map(|s| s.heading_text.as_str()).collect();
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
        out.push(msg::duplicate_heading_across_files(h, paths[0], paths[1]));
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
pub fn missing_answered_refusals_except(records: &[RecordFile], led: &Ledger, skip: &HashSet<String>) -> Vec<Finding> {
    let mut present_ids: HashSet<String> = HashSet::new();
    for rf in records {
        for sec in &rf.sections {
            if let Some(id) = &sec.id {
                present_ids.insert(id.clone());
            }
        }
    }

    let mut state_ids: BTreeSet<String> = BTreeSet::new();
    for row in &led.rows {
        if matches!(
            row.event.as_str(),
            "asked" | "answered" | "refused" | "superseded" | "withdrawn"
        ) {
            state_ids.insert(row.id.clone());
        }
    }

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
                out.push(Finding::new("ids", msg::answered_id_missing(id, &heading)));
            }
        }
    }
    out
}

/// True when the set on `d` is the one `ask` froze on `target` for `sec`:
/// an `assumes` row of the target for this dependent carries the hash of
/// exactly this entry. A set written after the target was answered has no
/// such row, so it is a shape error.
pub(crate) fn set_frozen_at_ask(led: &Ledger, sec: &record::Section, d: &record::Dep, target: &record::Section) -> bool {
    let (Some(id), Some(tid), Some(set)) = (&sec.id, &target.id, d.set()) else { return false };
    let hash = record::assumes_hash(id, set);
    led.assumes_for(tid).iter().any(|r| &r.detail == id && r.hash == hash)
}

/// The rule on the tiers of `rf` (per "What does the tool derive the tiers
/// from?"): the one finding its derive error earns, a heading-shaped entry
/// on a question of another topic file not yet answered waiting for that
/// answer. About the questions the error names.
pub fn derive_finding(rel: &str, rf: &RecordFile, all: &[RecordFile], led: &Ledger) -> Option<Finding> {
    let err = rf.derive_error.as_ref()?;
    let waiting = match err {
        record::DeriveError::Dangling { section, entry } => waiting_on_other_file(rel, section, entry, all, led),
        _ => None,
    };
    // The sections the error is about (`check` reads them to tell a
    // question that can still change from one that cannot): the one
    // holding the entry, the self-dependent one, or every one in the cycle.
    let named: Vec<&str> = match err {
        record::DeriveError::Dangling { section, .. } | record::DeriveError::AmbiguousHeading { section, .. } => {
            vec![section.as_str()]
        }
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
    let f = waiting.unwrap_or_else(|| Finding::new("plan", msg::derive_error(rel, err)));
    Some(f.in_sections(at))
}

/// The cap (per "What bounds the questions of a tier?"): no derived tier may
/// hold more than `TIER_CAP` live questions. Only judged when tiers were
/// derived; a refused, superseded or withdrawn question does not count, nor
/// a decision an open question supersedes (`replaced`).
pub fn cap_finding(rel: &str, rf: &RecordFile, replaced: &HashSet<String>, led: &Ledger) -> Option<Finding> {
    let tiers = rf.tiers.as_ref()?;
    let (open, answered) = state::over_cap_headings(led, rf, tiers, replaced)?;
    Some(Finding::new("plan", msg::tier_cap_exceeded(rel, &open, answered, record::TIER_CAP)))
}

// --- the judgements every command reads --------------------------------------

/// What the rules find in the plan of the topic file `rf` (the file as
/// loaded; `all` is every topic file), with `names` every heading by its id:
/// a copy of `rf` is normalized the way `approve` normalizes it, then judged
/// by `record_refusals`. What `show` and the refusal `answer` makes read for
/// the moves of a plan.
pub fn judge_record(
    names: &HashMap<String, String>,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[terms::Term],
    led: &Ledger,
) -> Vec<Finding> {
    let phrase_lookup = terms::phrase_lookup(terms_list);
    let cross_file_headings = depends::cross_file_heading_index(all, led);
    let base = depends::replaced_base(all, &depends::approved_plan_rels(all, led), &cross_file_headings, led);
    let mut rf = rf.clone();
    depends::normalize_cross_file_deps(&mut rf, terms_list, &phrase_lookup, &cross_file_headings, led);
    let replaced = depends::replaced_ids(&base, &rf, &cross_file_headings, led);
    about_topic_file(
        record_refusals(&rf.rel, &rf, terms_list, &phrase_lookup, &cross_file_headings, names, all, &replaced, led),
        &rf.rel,
    )
}

/// What one topic file is judged against: the file normalized
/// (`depends::normalize_cross_file_deps`), every topic file as loaded, the
/// terms, and what the caller built once from them.
pub struct TopicFileJudge<'a> {
    pub root: &'a Path,
    pub rel: &'a str,
    pub rf: &'a RecordFile,
    pub all: &'a [RecordFile],
    pub terms_list: &'a [terms::Term],
    pub phrase_lookup: &'a HashMap<String, String>,
    pub cross_file_headings: &'a HashMap<String, Vec<(String, String)>>,
    pub names: &'a HashMap<String, String>,
    pub replaced: &'a HashSet<String>,
    pub led: &'a Ledger,
    /// Whether the planning rules judge the file (`planning_rules_apply`,
    /// read off the file as written, before the normalize pass).
    pub planning: bool,
}

/// Whether the rules of the planning steps (a question with no body, the
/// reader's rows, every neighbour on a line) judge a topic file: while it
/// awaits approval, its plan on the way to the owner. An approved file is
/// judged by the rules of its record alone, so a heading approved before
/// those rules is no finding. The one scope `approve` and `check` both read
/// (per "Does a command refuse what check passes?", fe14991d A). `rf` is the
/// file as written, before `depends::normalize_cross_file_deps`; `rel` its
/// path from the root, as the ledger names it.
pub fn planning_rules_apply(led: &Ledger, rf: &RecordFile, rel: &str) -> bool {
    status::topic_file_status_at(led, rf, rel).awaits_approval()
}

/// Every rule one topic file breaks on its own, in the order `approve`
/// names them: the rules of the record (`record_refusals`), its own name and
/// headings, a heading in another topic file, an answer-set letter its
/// target has no option for, and, while `planning_rules_apply`, a question
/// not yet asked with no body and the planning records of a question not
/// yet asked (the reader's rows at each step and every neighbour on a
/// line). The one judgement `approve` refuses
/// on (per "Does a command refuse what check passes?", fe14991d A).
pub fn judge_topic_file(j: &TopicFileJudge) -> Vec<Finding> {
    let (rel, rf, led) = (j.rel, j.rf, j.led);
    let mut out = about_topic_file(
        record_refusals(
            rel,
            rf,
            j.terms_list,
            j.phrase_lookup,
            j.cross_file_headings,
            j.names,
            j.all,
            j.replaced,
            led,
        ),
        rel,
    );
    out.extend(found("plan", rel, own_name_and_heading_refusals(rel, rf.topic.as_deref(), &rf.sections)));
    out.extend(found("plan", rel, cross_file_heading_refusals(rel, rf, j.all, led)));
    // Per "Does a depends entry carry the answers it holds under?": a letter
    // the target has no option for is found before `ask`.
    out.extend(about_topic_file(in_sections("plan", letter_found(rel, rf, led)), rel));
    if j.planning {
        out.extend(found("plan", rel, planning_steps(j.root, rel, rf, j.all, j.terms_list, led)));
    }
    out
}

/// The findings of the planning steps on `rf`: a question not yet asked with
/// no body, and its planning records (the reader's rows at each step and
/// every neighbour on a line), judged only while the file awaits approval:
/// they are what the owner's approval of the plan waits on, and an approved
/// file has nothing for approve to take. Per "What does the agent do to plan
/// a topic file?" and "Does a planning step run without the record of the
/// step before?": the rows are read as they stand; neither the model nor
/// the reader is needed. `rf` normalized or as loaded gives one status: an
/// approved file holds identifiers, which normalizing leaves as they are.
fn planning_step_refusals(
    root: &Path,
    rel: &str,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[terms::Term],
    led: &Ledger,
) -> Vec<String> {
    if !planning_rules_apply(led, rf, rel) {
        return Vec::new();
    }
    planning_steps(root, rel, rf, all, terms_list, led)
}

/// A question not yet asked with no body, and the planning records of the
/// questions not yet asked: per "What does the agent do to plan a topic
/// file?" and "Does a planning step run without the record of the step
/// before?", the rows are read as they stand; neither the model nor the
/// reader is needed.
fn planning_steps(
    root: &Path,
    rel: &str,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[terms::Term],
    led: &Ledger,
) -> Vec<String> {
    let mut planning = crate::plan::load(root, rf, all, terms_list, led);
    planning.rel = rel.to_string();
    no_body_refusals(rel, rf, led).into_iter().chain(planning.approve_refusals()).collect()
}


/// `judge_topic_file` of the topic file `rf` as loaded from disk, with what
/// it is judged against built the way `approve` builds it: the file
/// normalized (`depends::normalize_cross_file_deps`), every heading's text,
/// and what the plan's supersedes lines replace. What `adr plan` prints as
/// the findings on its file: the judgement `approve` refuses on.
pub fn judge_topic_file_at(
    root: &Path,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[terms::Term],
    led: &Ledger,
) -> Vec<Finding> {
    let phrase_lookup = terms::phrase_lookup(terms_list);
    let cross_file_headings = depends::cross_file_heading_index(all, led);
    let mut rf = rf.clone();
    depends::normalize_cross_file_deps(&mut rf, terms_list, &phrase_lookup, &cross_file_headings, led);
    let names = crate::view::heading_texts(all);
    let base = depends::replaced_base(all, &depends::approved_plan_rels(all, led), &cross_file_headings, led);
    let replaced = depends::replaced_ids(&base, &rf, &cross_file_headings, led);
    judge_topic_file(&TopicFileJudge {
        root,
        rel: &rf.rel,
        rf: &rf,
        all,
        terms_list,
        phrase_lookup: &phrase_lookup,
        cross_file_headings: &cross_file_headings,
        names: &names,
        replaced: &replaced,
        led,
        planning: planning_rules_apply(led, &rf, &rf.rel),
    })
}

/// `judge_topic_file_at` in two parts: the findings each cleared by a move
/// of its own, and the findings of the planning steps (a question not yet
/// asked with no body, missing reader rows, a neighbour on no line), whose
/// moves are the planning moves (`moves::of_planning`).
pub fn judge_topic_file_split_at(
    root: &Path,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[terms::Term],
    led: &Ledger,
) -> (Vec<Finding>, Vec<Finding>) {
    let steps: HashSet<String> = planning_step_refusals(root, &rf.rel, rf, all, terms_list, led).into_iter().collect();
    judge_topic_file_at(root, rf, all, terms_list, led).into_iter().partition(|f| !steps.contains(&f.message))
}

/// Per "Does the agent write the bodies before approve?": the plan is
/// approved with its bodies, so a question never asked that has no body is
/// a finding.
pub fn no_body_refusals(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<String> {
    rf.sections
        .iter()
        .filter(|sec| sec.is_skeleton)
        .filter(|sec| !sec.id.as_ref().is_some_and(|id| led.latest_asked(id).is_some()))
        .map(|sec| msg::question_without_body(rel, &sec.heading_text))
        .collect()
}

/// The rules on the records as a whole: a current decision whose id no
/// topic file holds (`missing_answered_refusals`).
pub fn judge_records(records: &[RecordFile], led: &Ledger) -> Vec<Finding> {
    missing_answered_refusals(records, led)
}

/// The rules on a removed topic file: each question it held that keeps its
/// removal from being approved (`removal_holds`).
pub fn judge_removed_topic_file(root: &Path, rel: &str, led: &Ledger, present_ids: &HashSet<String>) -> Vec<Finding> {
    removal_holds(root, rel, led, present_ids)
        .holds
        .iter()
        .map(|h| Finding::new("plan", msg::removal_holds_open(rel, h)).about(Subject::TopicFile(rel.to_string())))
        .collect()
}

/// The rules on the terms file (`terms_refusals`), each a finding about it;
/// the second part is what `terms_refusals` returns beside the messages (the
/// pairings `approve` rewrites).
#[allow(clippy::type_complexity)]
pub fn judge_terms_file(
    root: &Path,
    records: &[RecordFile],
    tf: &terms::TermsFile,
    led: &Ledger,
    source: &repo::Source,
    scan_banned: bool,
    approved_plans: &HashSet<String>,
) -> (Vec<Finding>, Vec<Vec<(String, String)>>) {
    let (messages, pairs) = terms_refusals(root, records, tf, led, source, scan_banned, approved_plans);
    (messages.into_iter().map(|m| Finding::new("terms", m).about(Subject::TermsFile)).collect(), pairs)
}

/// The coverage rule of a blame file over an origin of `n` lines (per "Does a
/// blame file cover its whole origin?"): each range past the origin's end and
/// each run of lines no row covers.
pub fn judge_blame_coverage(bf: &crate::blame::BlameFile, n: usize) -> Vec<Finding> {
    let (beyond, gaps) = crate::blame::coverage_errors(&bf.rows, n);
    let about = || Subject::BlameFile(bf.rel.clone());
    let mut out: Vec<Finding> = beyond
        .iter()
        .map(|b| Finding::new("blame", msg::blame_range_beyond(&bf.rel, b.start, b.end, n)).about(about()))
        .collect();
    out.extend(
        gaps.iter().map(|g| Finding::new("blame", msg::blame_coverage_gap(&bf.rel, g.start, g.end)).about(about())),
    );
    out
}

/// The destination rule of a blame file (per "What does a blame row
/// name?"): each row whose destination is not one of the closed set, and,
/// for the rows that resolve, what each resolves to, in row order. `origin`
/// is the file's `origin:` path.
pub fn judge_blame_destinations(
    root: &Path,
    source: &repo::Source,
    bf: &crate::blame::BlameFile,
    origin: &str,
) -> (Vec<Finding>, Vec<crate::blame::Resolved>) {
    use crate::blame;
    // Every name a row resolves against is read through `source`, the same
    // tree the caller judges: `check --staged` resolves against the staged
    // files.
    let (all_records, _) = record::load_all_via(root, source);
    let (any_headings, known_heading_ids) = blame::heading_ids(&all_records);
    let terms_list: Vec<terms::Term> =
        state::load_terms_via(root, source).map(|tf| terms::minted_copy(&tf).terms).unwrap_or_default();
    let phrase_lookup = terms::phrase_lookup(&terms_list);
    let decision_stems: HashSet<String> = repo::list_decision_files_via(root, source)
        .iter()
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .collect();
    let ctx = blame::ResolveCtx {
        any_headings,
        known_heading_ids,
        terms_list: terms_list.as_slice(),
        phrase_lookup: &phrase_lookup,
        source: *source,
        decision_stems,
        known_origins: blame::known_origins_via(root, source),
        own_origin: origin.to_string(),
        root: root.to_path_buf(),
    };
    let mut findings = Vec::new();
    let mut resolved = Vec::new();
    for row in &bf.rows {
        match blame::resolve_destination(&row.dest, &ctx) {
            Ok(r) => resolved.push(r),
            Err(e) => findings.push(
                Finding::new("blame", blame::dest_refusal_text(&bf.rel, row, &e))
                    .about(Subject::BlameFile(bf.rel.clone())),
            ),
        }
    }
    (findings, resolved)
}

/// Every rule one blame file breaks, read off its state against its origin
/// (`blame::file_state`): the failures of the state, then, when the origin
/// could be read, coverage and destinations. What `show` names the moves of
/// a blame file from; `approve` reads the coverage and destination rules
/// through the same two functions.
pub fn judge_blame_file(root: &Path, st: &crate::blame::FileState, bf: &crate::blame::BlameFile) -> Vec<Finding> {
    let about = || Subject::BlameFile(bf.rel.clone());
    let mut out: Vec<Finding> =
        st.shape.iter().chain(&st.failures).map(|f| Finding::new("blame", f.clone()).about(about())).collect();
    if let Some(c) = &st.origin_content {
        out.extend(judge_blame_coverage(bf, c.lines().count()));
        if let Some(origin) = &bf.origin {
            out.extend(judge_blame_destinations(root, &repo::Source::WorkingTree, bf, origin).0);
        }
    }
    out
}
