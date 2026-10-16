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
use crate::moves;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::rule::Rule;
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
    /// The topic files as a directory, by the path of the file the finding
    /// is in: its name, or a heading it holds twice or shares with another
    /// file. `check` reads these off the directory itself, a file that does
    /// not parse included.
    Directory(String),
}

/// The class a finding is filed under inside the tool: what `check` sorts
/// failures from notes by, and what `approve` and `ask` pick the findings
/// they refuse on by. Never printed: a finding prints its found text and
/// its move, with no tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    Parse,
    Plan,
    PlanFrozen,
    Tells,
    Answers,
    Ids,
    Terms,
    Blame,
    Banned,
    Ledger,
    Reader,
    Install,
    Tested,
    Untested,
    Brief,
    /// A finding of no class `check` or a command picks it by.
    Other,
}

/// The command whose own input, environment, files or gate a stop is about.
/// A stop prints `<command>: <what>; <move>`; a finding never carries a
/// command's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Check,
    Show,
    Ask,
    Approve,
    Answer,
    Ban,
    Unban,
    Install,
    Uninstall,
    Serve,
    Read,
    Plan,
}

impl Command {
    pub fn as_str(self) -> &'static str {
        match self {
            Command::Check => "check",
            Command::Show => "show",
            Command::Ask => "ask",
            Command::Approve => "approve",
            Command::Answer => "answer",
            Command::Ban => "ban",
            Command::Unban => "unban",
            Command::Install => "install",
            Command::Uninstall => "uninstall",
            Command::Serve => "serve",
            Command::Read => "read",
            Command::Plan => "plan",
        }
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One rule a thing in the records breaks: the rule, what was found, and
/// the move that clears it. Printed as `<found>; <move>`, with no leading
/// tag, so the same line reads the same under every command.
#[derive(Debug, Clone)]
pub struct Finding {
    pub subject: Subject,
    pub rule: &'static Rule,
    /// The class the tool sorts the finding by; never printed.
    pub class: Class,
    /// What was found, with the place it was found at.
    pub found: String,
    /// The move that clears it, whole (`agent ...`, `owner runs: ...`,
    /// `not yet ...`).
    pub next: msg::Next,
    /// The questions of the file the finding is about, by index; empty for
    /// a finding about the file as a whole (its prose, headings, tiers).
    /// `check` reads it to tell a finding in text that can still change from
    /// one in an asked or answered question.
    pub sections: Vec<usize>,
}

impl Finding {
    /// A finding about the records of no class; `about` names its subject.
    pub fn new(rule: &'static Rule, found: String, next: msg::Next) -> Finding {
        Finding {
            subject: Subject::Records,
            rule,
            class: Class::Other,
            found,
            next,
            sections: Vec::new(),
        }
    }
    /// A finding of `class` from what a message function said.
    pub fn said(class: Class, said: msg::Said) -> Finding {
        Finding::new(said.rule, said.found, said.next).class(class)
    }
    pub fn class(mut self, class: Class) -> Finding {
        self.class = class;
        self
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
    /// The finding as one line: what was found and the move that clears it. The one rendering every command prints.
    pub fn text(&self) -> String {
        msg::finding_line(&self.found, &self.next)
    }
}

/// The one order findings print in, under every command and on the page: by
/// the file the finding is about (the records as a whole first), then the
/// first question of the file it is about (the file as a whole first), then
/// the rule's place in `rule::ALL`, then its text.
pub fn finding_order(a: &Finding, b: &Finding) -> std::cmp::Ordering {
    fn key(f: &Finding) -> (String, usize, usize, String) {
        let file = match &f.subject {
            Subject::Records => String::new(),
            Subject::TermsFile => repo::TERMS_PATH.to_string(),
            Subject::TopicFile(p) | Subject::BlameFile(p) | Subject::Directory(p) => p.clone(),
        };
        let question = f.sections.iter().min().map_or(0, |i| i + 1);
        let rule = crate::rule::ALL
            .iter()
            .position(|r| r.name == f.rule.name)
            .unwrap_or(usize::MAX);
        (file, question, rule, f.text())
    }
    key(a).cmp(&key(b))
}

/// Tags each finding of `fs` as about the topic file `rel`, a finding about
/// the records left as it is.
fn about_topic_file(fs: Vec<Finding>, rel: &str) -> Vec<Finding> {
    fs.into_iter()
        .map(|f| {
            if f.subject == Subject::Records {
                f.about(Subject::TopicFile(rel.to_string()))
            } else {
                f
            }
        })
        .collect()
}

/// Strings of one rule, each a finding about the topic file `rel`.
fn found(class: Class, rel: &str, messages: Vec<msg::Said>) -> Vec<Finding> {
    messages
        .into_iter()
        .map(|m| Finding::said(class, m).about(Subject::TopicFile(rel.to_string())))
        .collect()
}

/// Every identifier the topic files `all` hold: the one set of the
/// questions in the files every reader of "held by no file" reads.
pub fn held_ids(all: &[RecordFile]) -> HashSet<&str> {
    all.iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.as_deref()))
        .collect()
}

/// A path of the shape of a topic file: `adrdom/decisions/<name>.md`.
pub(crate) fn is_topic_file_path(rel: &str) -> bool {
    // Any `.md` under the directory, nested too: the files the loader,
    // show, check, ask and approve take (`repo::list_decision_files_via`).
    rel.strip_prefix(&format!("{}/", repo::DECISIONS_DIR))
        .is_some_and(|name| name.ends_with(".md"))
}

/// A path of the shape of a blame file: `adrdom/blame/<name>.md`, nested
/// paths too (`blame::Index` lists the same files).
pub(crate) fn is_blame_file_path(rel: &str) -> bool {
    rel.strip_prefix(&format!("{}/", repo::BLAME_DIR))
        .is_some_and(|name| name.ends_with(".md"))
}

/// Whether `rel` (a path from the repository root) names a file approve
/// takes: a topic file under adrdom/decisions, adrdom/terms.md, or a blame
/// file under adrdom/blame.
pub fn approve_takes(rel: &str) -> bool {
    rel == repo::TERMS_PATH || is_topic_file_path(rel) || is_blame_file_path(rel)
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
        .filter(|rel| {
            led.latest_for_path("approved", rel)
                .is_some_and(|r| r.hash != empty)
        })
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
pub(crate) fn latest_approved_version(
    root: &Path,
    rel: &str,
    led: &Ledger,
) -> Option<std::sync::Arc<RecordFile>> {
    // An approved removal is the empty plan: no version to read.
    if led
        .latest_for_path("approved", rel)
        .is_some_and(|r| r.hash == empty_plan_hash())
    {
        return None;
    }
    let mut newest: Option<std::sync::Arc<RecordFile>> = None;
    for old in crate::approval::committed_topics(root, rel) {
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
pub fn removal_holds(
    root: &Path,
    rel: &str,
    led: &Ledger,
    present_ids: &HashSet<String>,
) -> RemovalHolds {
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
    for r in led
        .rows
        .iter()
        .filter(|r| r.event == "approved" && r.id != "-" && r.detail == rel)
    {
        // approve writes a per-question row only when the question's hash
        // changed, so a heading approved, never asked, then struck keeps a
        // row the ledger cannot tell from a live one. Only an id with a
        // ledger state counts from the ledger; an unasked heading counts
        // from the file's latest approved committed plan alone.
        if led.latest_state(&r.id).is_some()
            && seen.insert(r.id.clone())
            && led
                .latest_for_id("approved", &r.id)
                .is_some_and(|l| l.detail == rel)
        {
            let heading = led
                .latest_asked(&r.id)
                .map(|a| a.detail.clone())
                .unwrap_or_else(|| r.id.clone());
            held.push((r.id.clone(), heading));
        }
    }
    held.retain(|(id, _)| !present_ids.contains(id));

    let mut holds = Vec::new();
    for (id, heading) in &held {
        // The status the closed list names (`status::question_status_of_id`):
        // a question never asked, asked, or a current decision holds the
        // removal; a closed one does not.
        let status = match status::question_status_of_id(led, id) {
            None => status::QuestionStatus::NotAsked,
            Some(st @ (status::QuestionStatus::Asked | status::QuestionStatus::Answered)) => st,
            Some(_) => continue,
        };
        holds.push(moves::RemovalHold {
            id: id.clone(),
            heading: heading.clone(),
            status,
        });
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
/// A finding about the terms file from what a message function said.
fn terms_finding(said: msg::Said) -> Finding {
    Finding::said(Class::Terms, said).about(Subject::TermsFile)
}

pub fn terms_refusals(
    root: &Path,
    records: &[RecordFile],
    tf: &terms::TermsFile,
    led: &Ledger,
    source: &repo::Source,
    scan_banned: bool,
    approved_plans: &std::collections::HashSet<String>,
) -> (Vec<Finding>, Vec<Vec<(String, String)>>) {
    let mut refusals: Vec<Finding> = Vec::new();

    // Step 1 (shape): "missing id" cannot appear here, since `tf` is
    // already minted.
    for e in &tf.errors {
        refusals.push(terms_finding(msg::terms_shape_error(e)));
    }

    // Step 3: definition pairings; a self-pairing or any cycle refuses.
    // Every cycle is reported, not just the first.
    for cycle in terms::cycles(tf) {
        refusals.push(terms_finding(msg::circular_definition(&cycle)));
    }

    // Fresh definition pairs for every term (never re-derived from a stale
    // terms line in the file). Computed once and reused for the dependents
    // check below and, by the caller, for the write step.
    let pairs_per_entry: Vec<Vec<(String, String)>> = (0..tf.terms.len())
        .map(|i| terms::definition_pairs(tf, i))
        .collect();

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
            let Some(target) = tf
                .terms
                .iter()
                .find(|t| t.id.as_deref() == Some(uuid.as_str()))
            else {
                let word = dep.from_word.then_some(word.as_str());
                refusals.push(terms_finding(msg::term_removed_but_carried(
                    uuid,
                    word,
                    &label,
                    dep.source.decision_ref(),
                )));
                continue;
            };
            if target.retired {
                // A decision dependent: the agent's move is to write a new
                // question that supersedes it, not the owner's to make on
                // the spot. A term dependent (its own definition still
                // carries the retired word) is never superseded (per "What
                // holds a retire of a term?"): the agent rewords the
                // definition and the owner re-approves.
                let user = match &dep.source {
                    state::DependentSource::Decision { id, heading, .. } => {
                        msg::RetiredUser::Decision(id, heading)
                    }
                    state::DependentSource::Asked { rel, .. } => msg::RetiredUser::Asked(rel),
                    state::DependentSource::PlanHeading { rel, .. } => {
                        msg::RetiredUser::PlanHeading(rel)
                    }
                    state::DependentSource::Term { .. } => msg::RetiredUser::Definition,
                };
                refusals.push(terms_finding(msg::term_retired_but_carried(
                    &target.name,
                    &label,
                    user,
                )));
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
                    refusals.push(terms_finding(msg::word_moved_off_term(
                        word,
                        &target.name,
                        &label,
                        dep.source.decision_id(),
                    )));
                }
            }
        }
    }

    // Step 5: banned hits in terms.md's own text, from the single ban list
    // (skipped for `check`'s call -- see `scan_banned` above -- since
    // `check_banned` already scans this same file as a tracked file).
    if scan_banned {
        refusals.extend(terms_banned_hits(root, tf, source));
    }

    // Every term's own definition is one or two sentences
    // (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76; not one of the spec's
    // numbered steps). Checked on every entry, retired or not, the same
    // way the shape checks in `terms::parse` run on every entry regardless
    // of `retired`.
    for t in &tf.terms {
        let count = terms::sentence_count(&t.definition);
        if count > 2 {
            refusals.push(terms_finding(msg::definition_too_long(&t.name, count)));
        }
    }

    (refusals, pairs_per_entry)
}

/// The banned words `adrdom/terms.md` holds, each by the line the file as
/// written holds it on: what `approve` refuses the terms file on, and what
/// `check` reads to know whether `approve` would take it (`check_banned`
/// names the hit itself, as it does in every tracked file).
pub fn terms_banned_hits(
    root: &Path,
    tf: &terms::TermsFile,
    source: &repo::Source,
) -> Vec<Finding> {
    let banned_entries = crate::banned::read_entries_via(root, source);
    if banned_entries.is_empty() {
        return Vec::new();
    }
    let hash_map: std::collections::HashMap<String, Option<String>> =
        banned_entries.into_iter().collect();
    // The file as written, not the minted copy: an entry minted an id gains
    // a line, and a hit names the line the file holds.
    let text = source
        .read(root, crate::repo::TERMS_PATH)
        .unwrap_or_else(|| tf.lines.join("\n"));
    crate::banned::hits(&text, &hash_map, &tf.terms)
        .into_iter()
        .map(|hit| {
            terms_finding(msg::terms_banned_word(
                hit.line,
                &msg::term_suffix(hit.term.as_deref()),
            ))
        })
        .collect()
}

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
fn in_sections(class: Class, found: Vec<(usize, msg::Said)>) -> Vec<Finding> {
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
/// what `ask` refuses. Cross-file targets carry no set before they are
/// answered, so only a same-file target is read.
pub fn letter_refusals(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<Finding> {
    in_sections(Class::Plan, letter_found(rel, rf, led))
}

/// A pair is judged when the dependent is open to ask, or when its target
/// has no letter answer yet: a target edited and asked again can lose the
/// option an asked dependent's set names, which `ask` refuses.
fn letter_found(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<(usize, msg::Said)> {
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
/// options?"), one judgement `check` fails, `approve` and `ask` refuse on,
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
                id,
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
    // rules `ask` enforces, found before approval in a question not yet asked.
    out.extend(in_sections(Class::Tells, length_found(rel, rf, led)));
    // The other two body rules `ask` enforces, found the same way: a loaded
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
            if frozen && matches!(d, record::Dep::Text { .. }) {
                let st = status::question_status(led, sec);
                out.push(
                    Finding::said(
                        Class::PlanFrozen,
                        msg::frozen_depends_edited(rel, &sec.heading_text, st),
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

    // Per "Does every neighbour get a line under its heading?": an
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
                    .map_or(true, |id| !state::is_current(led, id))
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

/// True when the set on `d` is the one `ask` froze on `target` for `sec`:
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

/// The cap (per "What bounds the questions at one tier?"): no derived tier may
/// hold more than `TIER_CAP` live questions. Only judged when tiers were
/// derived; a refused, superseded or withdrawn question does not count, nor
/// a decision an open question supersedes (`replaced`).
pub fn cap_finding(
    rel: &str,
    rf: &RecordFile,
    replaced: &HashSet<String>,
    led: &Ledger,
) -> Option<Finding> {
    let tiers = rf.tiers.as_ref()?;
    let (open, answered) = state::over_cap_headings(led, rf, tiers, replaced)?;
    Some(Finding::said(
        Class::Plan,
        msg::tier_cap_exceeded(rel, &open, answered, record::TIER_CAP),
    ))
}

// --- the judgements every command reads --------------------------------------

/// What one topic file is judged against: the file, every topic file as
/// loaded, the terms, and what the caller built once from them.
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
    /// Every blame file the `blame:` lines are judged against, read through
    /// the same `repo::Source` as the topic files of the run.
    pub blame: &'a crate::blame::Index,
    pub led: &'a Ledger,
    /// Whether the planning rules judge the file (`planning_rules_apply`,
    /// read off the file as written).
    pub planning: bool,
    /// What the planning of every file of the run reads, when the caller
    /// judges many files and has read it once; `None` reads it here.
    pub shared: Option<&'a crate::plan::Shared>,
    /// Whether the rules on the directory (the file's own name, a heading
    /// twice in it, a heading in another file) judge it too: `false` for a
    /// caller that judges the whole directory once on its own (`check`).
    pub directory: bool,
}

/// Whether the rules of the planning steps (a question with no body, the
/// reader's rows, every neighbour on a line) judge a topic file: while it
/// awaits approval, its plan on the way to the owner. An approved file is
/// judged by the rules of its record alone, so a heading approved before
/// those rules is no finding. The one scope `approve` and `check` both read
/// (per "Does a command refuse what check passes?", fe14991d A). `rf` is the
/// file as written; `rel` its
/// path from the root, as the ledger names it.
pub fn planning_rules_apply(led: &Ledger, rf: &RecordFile, rel: &str) -> bool {
    status::topic_file_status_at(led, rf, rel).awaits_approval()
}

/// Every rule one topic file breaks on its own, in the order `approve`
/// names them: the rules of the record (`record_refusals`), its own name and
/// headings, a heading in another topic file, an answer-set letter its
/// target has no option for, and, while `planning_rules_apply`, a question
/// not yet asked with no body and the planning records of a question not
/// yet asked (the record of each step and every neighbour on a
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
            j.blame,
            led,
        ),
        rel,
    );
    if j.directory {
        out.extend(own_name_and_heading_refusals(
            rel,
            rf.topic.as_deref(),
            &rf.sections,
        ));
        out.extend(cross_file_heading_refusals(rel, rf, j.all, led));
    }
    // Per "Does a depends entry carry the answers it holds under?": a letter
    // the target has no option for is found before `ask`.
    out.extend(about_topic_file(
        in_sections(Class::Plan, letter_found(rel, rf, led)),
        rel,
    ));
    if j.planning {
        out.extend(found(
            Class::Plan,
            rel,
            planning_steps(j.root, rel, rf, j.all, j.terms_list, led, j.shared),
        ));
    }
    out.extend(about_topic_file(
        banned_in_questions(j.root, rel, rf, j.terms_list, led),
        rel,
    ));
    out
}

/// A banned word in the text of a question open to ask (per "What does the
/// ban scan read?" and "Does ban refuse a word that appears in the
/// repository?"): one finding per question, naming the question and never
/// the word. What `ask` refuses a candidate on, read by `show`, `plan` and
/// `approve` alike, so no command names `adr ask` or the owner's approve
/// for a plan holding one. `check` reports the word by file and line in
/// its own scan of every tracked file and does not file these again.
pub fn banned_in_questions(
    root: &Path,
    rel: &str,
    rf: &RecordFile,
    terms_list: &[terms::Term],
    led: &Ledger,
) -> Vec<Finding> {
    let entries: HashMap<String, Option<String>> =
        crate::banned::read_entries(&root.join(crate::banned::BAN_LIST))
            .into_iter()
            .collect();
    if entries.is_empty() {
        return Vec::new();
    }
    rf.sections
        .iter()
        .enumerate()
        .filter(|(_, sec)| status::open_to_ask_text(led, rf, sec))
        .filter_map(|(i, sec)| {
            let text = state::section_text_excluding_terms_line(&rf.lines, sec);
            let hit = crate::banned::first_hit(&text, &entries, terms_list)?;
            let said = msg::banned_word_in_candidate(
                rel,
                &sec.heading_text,
                &msg::term_suffix(hit.term.as_deref()),
            );
            Some(Finding::said(Class::Banned, said).in_section(i))
        })
        .collect()
}

/// The questions asked (or refused) in the topic file `rel` and struck from
/// its plan since: each id whose latest `approved` row names `rel`, whose
/// latest state is `asked` or `refused`, and which no topic file of `all`
/// holds any more, while the plan of `rel` differs from its approval
/// (`stale`, read by the caller off the file as written; `sole_stale` when
/// it is the only such file). One finding per
/// id, whose move is the owner's approve of `rel`, which writes the
/// withdrawn row (per "Does approve withdraw an asked question struck from
/// the plan?"). Not part of `judge_topic_file`: approve clears it rather
/// than refusing on it; show, plan and check print it the same way.
pub fn struck_asked(
    rel: &str,
    stale: bool,
    sole_stale: bool,
    all: &[RecordFile],
    led: &Ledger,
) -> Vec<Finding> {
    if !stale {
        return Vec::new();
    }
    struck_asked_ids(rel, sole_stale, all, led)
        .into_iter()
        .map(|(id, heading, event)| {
            Finding::said(
                Class::Ids,
                msg::removed_from_plan(&id, &heading, event, rel),
            )
            .about(Subject::TopicFile(rel.to_string()))
        })
        .collect()
}

/// The ids `struck_asked` reports for `rel`, each with the heading it was
/// asked under and its latest state, in ledger order: the ids whose latest
/// per-question `approved` row names `rel`, and, when `rel` is the only file
/// awaiting a re-approval (`sole_stale`), the ids asked under a ledger that
/// approved whole files only and so names no file for them.
pub fn struck_asked_ids<'l>(
    rel: &str,
    sole_stale: bool,
    all: &[RecordFile],
    led: &'l Ledger,
) -> Vec<(String, String, &'l str)> {
    let held: HashSet<&str> = held_ids(all);
    let mut seen: HashSet<&str> = HashSet::new();
    let mut out = Vec::new();
    for row in led
        .rows
        .iter()
        .filter(|r| r.event == "asked" && r.id != "-")
    {
        let id = row.id.as_str();
        if !seen.insert(id) || held.contains(id) {
            continue;
        }
        match led.latest_for_id("approved", id).map(|r| r.detail.as_str()) {
            Some(file) if file == rel => {}
            None if sole_stale => {}
            _ => continue,
        }
        let Some(event) = led
            .latest_state(id)
            .map(|r| r.event.as_str())
            .filter(|e| *e == "asked" || *e == "refused")
        else {
            continue;
        };
        let heading = led
            .latest_asked(id)
            .map(|r| r.detail.clone())
            .unwrap_or_else(|| id.to_string());
        out.push((id.to_string(), heading, event));
    }
    out
}

/// The topic files of `all` that were approved once and whose plan differs
/// from that approval now, read off the files as written.
pub fn stale_files<'a>(all: &'a [RecordFile], led: &Ledger) -> Vec<&'a str> {
    all.iter()
        .filter(|rf| {
            led.latest_for_path("approved", &rf.rel).is_some()
                && !state::plan_approved(led, rf, &rf.rel)
        })
        .map(|rf| rf.rel.as_str())
        .collect()
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
    shared: Option<&crate::plan::Shared>,
) -> Vec<msg::Said> {
    let mut planning = match shared {
        Some(shared) => crate::plan::load_with(rf, terms_list, led, shared),
        None => crate::plan::load(root, rf, all, terms_list, led),
    };
    planning.rel = rel.to_string();
    // The planning steps run in order, and the findings are those of the
    // step the plan is at, the one `moves::of_planning` names: a later
    // step's records are missing only because this one is not done (per
    // "Does a command compute a move of its own?", aac5c313 A).
    match planning.next() {
        crate::plan::NextMove::Plan => planning.rows_missing_at_step(),
        crate::plan::NextMove::WriteLines => planning.line_refusals(),
        crate::plan::NextMove::WriteBodies(_) => no_body_refusals(rel, rf, led),
        crate::plan::NextMove::Ready => Vec::new(),
    }
}

/// What the judgement of every topic file of a run reads of the whole
/// repository, read once per run rather than once per file: the planning
/// records (`plan::Shared`, with the term phrases and the heading indexes),
/// what the approved plans' supersedes lines replace, the files whose plan
/// differs from an approval and the blame files.
pub struct RepoIndex {
    pub planning: crate::plan::Shared,
    /// Every blame file, the `blame:` lines' side of the judgement.
    pub blame: crate::blame::Index,
    base: HashSet<String>,
    stale: Vec<String>,
    records: Vec<Finding>,
}

impl RepoIndex {
    pub fn new(
        root: &Path,
        all: &[RecordFile],
        terms_list: &[terms::Term],
        led: &Ledger,
    ) -> RepoIndex {
        RepoIndex::new_via(root, all, terms_list, led, &repo::Source::WorkingTree)
    }

    /// `new` with the reader's rows as `source` holds them.
    pub fn new_via(
        root: &Path,
        all: &[RecordFile],
        terms_list: &[terms::Term],
        led: &Ledger,
        source: &repo::Source,
    ) -> RepoIndex {
        let (blame, _) = crate::blame::Index::read(root, source, led);
        RepoIndex::new_with_blame(root, all, terms_list, led, source, blame)
    }

    /// `new_via` with the blame index the caller already holds, read from the
    /// same rows as `source`, instead of reading it again.
    pub fn new_with_blame(
        root: &Path,
        all: &[RecordFile],
        terms_list: &[terms::Term],
        led: &Ledger,
        source: &repo::Source,
        blame: crate::blame::Index,
    ) -> RepoIndex {
        let planning = crate::plan::Shared::new_via(root, all, terms_list, led, source);
        let base = depends::replaced_base(
            all,
            &depends::approved_plan_rels(all, led),
            &planning.cross,
            led,
        );
        let stale = stale_files(all, led)
            .into_iter()
            .map(str::to_string)
            .collect();
        let records = judge_records(all, led);
        RepoIndex {
            planning,
            blame,
            base,
            stale,
            records,
        }
    }

    /// The rules on the topic files as a whole (`judge_records`).
    pub fn records(&self) -> &[Finding] {
        &self.records
    }

    /// The files whose plan differs from an approval (`stale_files`).
    pub fn stale(&self) -> &[String] {
        &self.stale
    }
}

/// `judge_topic_file` of the file as loaded, judged the way approve judges
/// it, with what the run shares read once (`index`).
pub fn judge_topic_file_in(
    root: &Path,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[terms::Term],
    led: &Ledger,
    index: &RepoIndex,
) -> Vec<Finding> {
    let shared = &index.planning;
    let replaced = depends::replaced_ids(&index.base, rf, &shared.cross, led);
    judge_topic_file(&TopicFileJudge {
        root,
        rel: &rf.rel,
        rf,
        all,
        terms_list,
        phrase_lookup: &shared.phrase_lookup,
        cross_file_headings: &shared.cross,
        names: &shared.headings,
        replaced: &replaced,
        blame: &index.blame,
        led,
        planning: planning_rules_apply(led, rf, &rf.rel),
        shared: Some(shared),
        directory: true,
    })
}

/// Per "Does the agent write the bodies before approve?": the plan is
/// approved with its bodies, so a question never asked that has no body is
/// a finding.
pub fn no_body_refusals(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<msg::Said> {
    rf.sections
        .iter()
        .filter(|sec| sec.is_skeleton)
        .filter(|sec| {
            !sec.id
                .as_ref()
                .is_some_and(|id| led.latest_asked(id).is_some())
        })
        .map(|sec| msg::question_without_body(rel, &sec.heading_text))
        .collect()
}

/// The rules on the records as a whole: a current decision whose id no
/// topic file holds (`missing_answered_refusals`).
pub fn judge_records(records: &[RecordFile], led: &Ledger) -> Vec<Finding> {
    judge_records_except(records, led, &HashSet::new())
}

/// The terms a topic file is judged against, by every command that judges
/// one: the terms file with an identifier on every entry
/// (`terms::minted_copy`), so a word an entry still waiting for its
/// approval defines pairs to it, and the finding is the terms file's own
/// approval (`terms_gate`), never a missing term the file already holds.
pub fn judged_terms(tf: Option<&terms::TermsFile>) -> Vec<terms::Term> {
    tf.map(|tf| terms::minted_copy(tf).terms)
        .unwrap_or_default()
}

/// What keeps `approve`, `ask` and `answer` from taking the topic file
/// `rel` while adrdom/terms.md is missing, unreadable, malformed or waits
/// for its approval (per "Does the tool stop while the terms file waits for
/// approval?"): the gate `actions::require_terms` stops them on, as
/// findings of the file, so `show` and `plan` name the move that clears it
/// and no move those commands would refuse.
pub fn terms_gate(root: &Path, led: &Ledger, rel: &str) -> TermsGate {
    let mut gate = TermsGate::default();
    gate.waits = crate::terms::load(&root.join(crate::repo::TERMS_PATH))
        .is_ok_and(|tf| !state::terms_approved(led, &tf));
    if let Err(lines) = crate::actions::require_terms(root, led, rel, Command::Show) {
        for l in lines {
            match l {
                crate::out::Line::Finding(f) => gate.findings.push(f.about(Subject::TermsFile)),
                // What stopped the gate on the terms file's own input or
                // files stays the stop line it is.
                stop @ crate::out::Line::Stop { .. } => gate.stops.push(stop),
                _ => {}
            }
        }
    }
    gate
}

/// What `terms_gate` found: the findings of the terms file and the stops on
/// its own input or files.
#[derive(Debug, Default)]
pub struct TermsGate {
    pub findings: Vec<Finding>,
    pub stops: Vec<crate::out::Line>,
    /// The terms file is there and differs from its latest approval: a
    /// commit waits for the owner's approve.
    pub waits: bool,
}

impl TermsGate {
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty() && self.stops.is_empty()
    }
}

/// `judge_records` leaving out of the missing-answered rule the ids in
/// `skip` (`missing_answered_refusals_except`): every rule on the topic
/// files as a whole, the one judgement `check` fails, `approve` refuses and
/// `show` and `plan` print with a file's report. An identifier held by two
/// questions, a withdrawn identifier back in a file (a withdrawn identifier
/// never returns, per "Does approve withdraw an asked question struck from
/// the plan?"), and a current decision no topic file holds.
pub fn judge_records_except(
    records: &[RecordFile],
    led: &Ledger,
    skip: &HashSet<String>,
) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut seen: HashMap<&str, String> = HashMap::new();
    for rf in records {
        for sec in rf.sections.iter().filter(|s| !s.is_skeleton) {
            let Some(id) = sec.id.as_deref() else {
                continue;
            };
            match seen.get(id) {
                Some(prev) => out.push(
                    Finding::said(
                        Class::Ids,
                        msg::duplicate_id(id, prev, &rf.rel, &sec.heading_text),
                    )
                    .about(Subject::TopicFile(rf.rel.clone())),
                ),
                None => {
                    seen.insert(id, format!("{}:{}", rf.rel, sec.heading_text));
                }
            }
        }
    }
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if led
                .latest_state(id)
                .is_some_and(|row| row.event == "withdrawn")
            {
                out.push(
                    Finding::said(
                        Class::Ids,
                        msg::withdrawn_id_present(id, &sec.heading_text, &rf.rel),
                    )
                    .about(Subject::TopicFile(rf.rel.clone())),
                );
            }
        }
    }
    out.extend(missing_answered_refusals_except(records, led, skip));
    out
}

/// The rules on a removed topic file: each question it held that keeps its
/// removal from being approved (`removal_holds`).
pub fn judge_removed_topic_file(
    root: &Path,
    rel: &str,
    led: &Ledger,
    present_ids: &HashSet<String>,
) -> Vec<Finding> {
    removal_holds(root, rel, led, present_ids)
        .holds
        .iter()
        .map(|h| {
            Finding::said(Class::Plan, msg::removal_holds_open(rel, h))
                .about(Subject::TopicFile(rel.to_string()))
        })
        .collect()
}

/// What approve would refuse in the terms file `tf` (minted) as it stands
/// in the working tree, banned words in its text included: the one terms
/// judgement `show` prints and the gate of `ask`, `answer` and the approve
/// of a topic file names, so no command offers the owner's approve of the
/// terms file while approve would refuse it.
pub fn terms_file_findings(
    root: &Path,
    records: &[RecordFile],
    tf: &terms::TermsFile,
    led: &Ledger,
) -> Vec<Finding> {
    let approved_plans: HashSet<String> = records
        .iter()
        .filter(|rf| state::plan_approved(led, rf, &rf.rel))
        .map(|rf| rf.rel.clone())
        .collect();
    judge_terms_file(
        root,
        records,
        tf,
        led,
        &repo::Source::WorkingTree,
        true,
        &approved_plans,
    )
    .0
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
    terms_refusals(root, records, tf, led, source, scan_banned, approved_plans)
}

/// The coverage rule of a blame file over an origin of `n` lines (per "Does a
/// blame file cover its whole origin?"): each range past the origin's end and
/// each run of lines no row covers.
pub fn judge_blame_coverage(bf: &crate::blame::BlameFile, n: usize) -> Vec<Finding> {
    let (beyond, gaps) = crate::blame::coverage_errors(&bf.rows, n);
    let about = || Subject::BlameFile(bf.rel.clone());
    let mut out: Vec<Finding> = beyond
        .iter()
        .map(|b| {
            Finding::said(
                Class::Blame,
                msg::blame_range_beyond(&bf.rel, b.start, b.end, n),
            )
            .about(about())
        })
        .collect();
    out.extend(gaps.iter().map(|g| {
        Finding::said(
            Class::Blame,
            msg::blame_coverage_gap(&bf.rel, g.start, g.end),
        )
        .about(about())
    }));
    out
}

/// Every rule one blame file breaks, read off its state against its origin
/// (`blame::file_state`): the findings of the state, then, when the origin
/// could be read, coverage. What `approve` refuses on, `check` prints and
/// `show` names the moves of a blame file from.
pub fn judge_blame_file(
    st: &crate::blame::FileState,
    bf: &crate::blame::BlameFile,
) -> Vec<Finding> {
    let mut out = st.findings.clone();
    if let Some(c) = &st.origin_content {
        out.extend(judge_blame_coverage(bf, c.lines().count()));
    }
    out
}
