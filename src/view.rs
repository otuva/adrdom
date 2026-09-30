//! Everything the page shows, computed from the repo. No HTML here.
use crate::commands::check;
use crate::depends;
use crate::ledger::{self, Ledger};
use crate::record::{self, Dep, RecordFile, Section};
use crate::repo;
use crate::state;
use crate::terms::{self, Term};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

/// The shipped list of common English words (`assets/common_words.txt`,
/// one word per line, lowercase): words that never need a term to explain
/// them, so the "No term" line under a question only names words that
/// might. Not governed data — it ships with the binary.
static COMMON_WORDS: &str = include_str!("../assets/common_words.txt");

fn common_words() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        COMMON_WORDS
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect()
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QState {
    Skeleton,
    Unasked,
    Open,
    Answered,
    /// The owner's answer that the question cannot be answered as asked
    /// (per "Does the owner answer a question with a refusal?":
    /// `**Answer:** refused`): answered for hashing/editing, but
    /// unanswered for the tier rule and for `done`.
    Refused,
    Superseded,
    Withdrawn,
}

pub struct OptionView {
    pub letter: char,
    pub sentence: String,
    pub why: Option<String>,
    pub costs: Option<String>,
}

/// One word on a section's terms line (or its live resolution), paired
/// with the term it names.
pub struct WordPair {
    pub word: String,
    pub term_name: String,
    pub definition: String,
}

pub struct QuestionView {
    pub id: Option<String>,
    pub heading: String,
    /// The first `record::VERSION_LEN` hex characters of this section's
    /// current hash (per "Does an answer carry the version of the question
    /// the owner read?"): what `show` and the page print next to an asked
    /// question, and what the page's answer form carries as its hidden `v`
    /// field. Computed for every question regardless of state; only shown
    /// to the owner while the question is `Open`.
    pub version: String,
    /// The section's derived tier (per "Does the tool derive tiers from
    /// the depends lines?"), or `None` when tier derivation
    /// failed for the file (`RecordFile::derive_error`).
    pub tier: Option<usize>,
    /// This section's `depends:` entries, each formatted as written in the
    /// file (a term name, a uuid, or a heading with its `=set` suffix
    /// restored — see `format_dep`), in file order.
    pub depends: Vec<String>,
    pub state: QState,
    pub problem: Option<String>,
    pub standard: Option<String>,
    pub options: Vec<OptionView>,
    pub answer: Option<char>,
    /// Each supersedes entry as the heading text it names (per "Does a
    /// reference in a topic file carry the identifier of what it names?":
    /// the page turns the id back into the name), or the bare id when no
    /// topic file holds that heading any more.
    pub supersedes: Vec<String>,
    pub tests: usize,
    /// Every word=term pairing on this section's terms line (answered or
    /// asked), or computed live over the section text when it has none yet.
    pub words: Vec<WordPair>,
    /// Distinct words (lowercased) in the section text that pair no term:
    /// not common English, not a uuid or number, not shorter than three
    /// letters, and not already covered by `words`.
    pub no_entry: Vec<String>,
}

pub struct FileView {
    pub path: String,
    pub topic: String,
    pub plan_approved: bool,
    pub context_approved: bool,
    pub done: bool,
    pub open_tier: Option<usize>,
    pub waits_on: Option<String>,
    /// One note per heading dep whose set excludes the answer its target
    /// was given (per "Does a depends entry carry the answers it holds
    /// under?", "moot", `moot.rs`), formatted `"<h>" stands
    /// on "<other>"=A|B, answered C` — the file-scoped counterpart to
    /// `check`'s `moot: <rel>: ...` lines, shown alongside this file on
    /// the overview page.
    pub moot: Vec<String>,
    pub questions: Vec<QuestionView>,
}

/// One `## name` term in `adrdom/terms.md`, as shown on the page.
pub struct TermEntryView {
    pub name: String,
    pub retired: bool,
    pub definition: String,
    pub aliases: Vec<String>,
    /// The number of current decisions whose terms line carries this
    /// term's uuid.
    pub dependents: usize,
}

pub struct TermsView {
    pub path: String,
    pub approved: bool,
    pub count: usize,
    /// Shape errors from parsing, plus one "circular definition: a -> b ->
    /// a" line per cycle `terms::cycles` finds.
    pub errors: Vec<String>,
    pub entries: Vec<TermEntryView>,
}

pub struct CheckReport {
    pub failures: Vec<String>,
    pub notes: Vec<String>,
}

pub struct Snapshot {
    pub files: Vec<FileView>,
    pub terms: Option<TermsView>,
    pub check: CheckReport,
}

/// The state a question heading is in, from the ledger's latest state row
/// for its id (`asked` -> `Open`, `answered` -> `Answered`, `refused` ->
/// `Refused`, `superseded`/`withdrawn` accordingly). A heading with no such
/// row, approved or not, is `Skeleton` while it has no body and `Unasked`
/// once it has one: approve gives every heading an id, so the id says
/// nothing about the body. `pub(crate)` so `commands::show` can reuse it
/// instead of re-deriving state.
pub(crate) fn qstate(led: &Ledger, sec: &Section) -> QState {
    let never_asked = if sec.is_skeleton {
        QState::Skeleton
    } else {
        QState::Unasked
    };
    let Some(id) = &sec.id else {
        return never_asked;
    };
    match led.latest_state(id).map(|r| r.event.as_str()) {
        Some("asked") => QState::Open,
        Some("answered") => QState::Answered,
        Some("refused") => QState::Refused,
        Some("superseded") => QState::Superseded,
        Some("withdrawn") => QState::Withdrawn,
        _ => never_asked,
    }
}

/// Every heading id, from any topic file (not just one), paired with its
/// heading text (per "Does a reference in a topic file carry the
/// identifier of what it names?"): what turns a depends `Id` entry that
/// names a decision in another topic file back into that decision's
/// heading text, for `format_dep`.
pub fn all_heading_texts(root: &Path) -> HashMap<String, String> {
    heading_texts(&record::load_all(root))
}

/// `all_heading_texts` for a caller that holds every topic file loaded.
pub fn heading_texts(records: &[RecordFile]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for rf in records {
        for sec in &rf.sections {
            if let Some(id) = &sec.id {
                out.insert(id.clone(), sec.heading_text.clone());
            }
        }
    }
    out
}

fn dep_with_set(text: &str, set: &Option<record::AnswerSet>) -> String {
    format!("{}{}", text, record::set_suffix(set))
}

/// A `depends:` entry's text as words (per "Does a reference in a topic
/// file carry the identifier of what it names?"): a `Text` entry (not yet
/// approved) prints as written, with its `=set`
/// suffix restored (`=A`, `=A|B`, `=*`) when it carries one. An `Id` entry
/// is looked up -- first as a heading id in `rf` (this file), else as a
/// term id (`terms_list`), else as another topic file's heading id
/// (`all_headings`) -- printed as that name/heading with the same set
/// suffix restored, or the bare id when none of the three resolve it
/// (should not happen once a plan is approved, but never hidden). Shared by
/// `show` (the "stands on:" line) and `/next` (the "Stands on:" line).
pub fn format_dep(
    d: &record::Dep,
    rf: &RecordFile,
    terms_list: &[Term],
    all_headings: &HashMap<String, String>,
) -> String {
    match d {
        record::Dep::Text { text, set } => dep_with_set(text, set),
        record::Dep::Id { id, set } => {
            if let Some(sec) = rf.sections.iter().find(|s| s.id.as_deref() == Some(id.as_str())) {
                return dep_with_set(&sec.heading_text, set);
            }
            if let Some(t) = terms_list.iter().find(|t| t.id.as_deref() == Some(id.as_str())) {
                return dep_with_set(&t.name, set);
            }
            if let Some(heading) = all_headings.get(id) {
                return dep_with_set(heading, set);
            }
            dep_with_set(id, set)
        }
    }
}

/// One entry per heading dep in `rf` whose declared answer set excludes
/// the letter its target was actually answered with (per "Does a depends
/// entry carry the answers it holds under?", "moot"; mirrors `check`'s
/// `moot.rs`, computed independently here since `record.rs` has no ledger
/// access).
pub(crate) struct MootPair {
    /// The dependent's own index in `rf.sections`: lets `ask` match a
    /// candidate by identity, since two sections may share a heading text
    /// until `check` refuses the duplicate.
    pub section_idx: usize,
    pub dependent_heading: String,
    pub target_heading: String,
    pub set_text: String,
    pub letter: char,
}

/// Every `MootPair` in `rf`. A target that is unanswered (gated on the
/// ledger's `qstate`, not the raw file text, so a body edited to show a
/// letter that was never actually asked/answered through the ledger never
/// counts), or a set of `=*`, never produces an entry. Shared by
/// `moot_notes` (its formatted-string wrapper, read by `check`/the page)
/// and `ask`'s moot refusal, so the two can never diverge on what counts
/// as moot.
pub(crate) fn moot_pairs(rf: &RecordFile, led: &Ledger, all: &[RecordFile]) -> Vec<MootPair> {
    let mut out = Vec::new();
    for (si, sec) in rf.sections.iter().enumerate() {
        for d in &sec.depends {
            let Some(record::AnswerSet::Letters(letters)) = d.set() else {
                continue;
            };
            // A same-file heading, else (an `Id` entry, which is what a
            // cross-file heading entry is once normalized) the section of
            // that id in any topic file.
            let target = match rf.dep_target(d) {
                Some(ti) => &rf.sections[ti],
                None => {
                    let Dep::Id { id, .. } = d else {
                        continue;
                    };
                    let Some(t) = all
                        .iter()
                        .flat_map(|r| r.sections.iter())
                        .find(|s| s.id.as_deref() == Some(id.as_str()))
                    else {
                        continue;
                    };
                    t
                }
            };
            // `Answered` implies a real id (qstate only reaches it via an
            // id's ledger row), so `target.answer_letter()` below is safe.
            if qstate(led, target) != QState::Answered {
                continue;
            }
            let Some(c) = target.answer_letter() else {
                continue;
            };
            if !letters.contains(&c) {
                out.push(MootPair {
                    section_idx: si,
                    dependent_heading: sec.heading_text.clone(),
                    target_heading: target.heading_text.clone(),
                    set_text: record::canonical_set(&record::AnswerSet::Letters(letters.clone())),
                    letter: c,
                });
            }
        }
    }
    out
}

/// `"<h>" stands on "<other>"=A|B, answered C`, `<h>` the dependent's
/// heading and `<other>` the target's -- `moot_pairs`, formatted for
/// `check`'s note and the page.
pub(crate) fn moot_notes(rf: &RecordFile, led: &Ledger, all: &[RecordFile]) -> Vec<String> {
    moot_pairs(rf, led, all)
        .into_iter()
        .map(|p| {
            format!(
                "\"{}\" stands on \"{}\"={}, answered {}",
                p.dependent_heading, p.target_heading, p.set_text, p.letter
            )
        })
        .collect()
}

/// `state::asked_text_stands` for a section of a loaded topic file: the
/// question `answer` takes, and the one `show` prints a version for.
pub(crate) fn asked_text_stands(led: &Ledger, rf: &RecordFile, sec: &Section) -> bool {
    let Some(id) = &sec.id else { return false };
    state::asked_text_stands(led, id, &record::section_hash(&rf.lines, sec))
}

/// What is next in one topic file, one line per move, each naming who
/// makes it: what `show` prints under the file and `ask` prints when it has
/// nothing to ask. The agent reads its next move here and never works it
/// out from the tiers. In order: the decisions to build (per "Does the
/// agent build on a decision before its tier is fully answered?": built
/// once every question beside it is answered), the bodies to write, the
/// answer sets to write, the questions to ask, the approval the file waits
/// for, the answers the owner owes, a refused or moot question that holds
/// the rest, the decisions that wait for a question beside them, and the
/// questions whose turn has not come. Empty when nothing is left to do.
///
/// `plan_ok` is read by the caller off the file as loaded, before
/// `depends::normalize_cross_file_deps` (see `snapshot`). `tests` is
/// `check::test_index`: an answered decision with a test tied to it is
/// built.
pub(crate) fn next_moves(
    rf: &RecordFile,
    led: &Ledger,
    plan_ok: bool,
    plan_refusals: &[String],
    tests: &HashMap<String, Vec<String>>,
    all: &[RecordFile],
) -> Vec<String> {
    use crate::msg;
    // What `approve` and `check` refuse in this file's plan
    // (`actions::record_refusals`), each with the move it names.
    let clears: Vec<String> = plan_refusals.iter().map(|r| msg::next_clears(r)).collect();
    let Some(tiers) = &rf.tiers else {
        if clears.is_empty() {
            return vec![msg::next_plan_unresolved()];
        }
        return clears;
    };
    let open_tier = state::open_tier(rf);
    let moot: HashSet<usize> = moot_pairs(rf, led, all).iter().map(|p| p.section_idx).collect();

    let mut bodies = Vec::new();
    let mut to_ask = Vec::new();
    let mut answers = Vec::new();
    let mut held = Vec::new();
    let mut moot_lines = Vec::new();
    let mut build = Vec::new();
    let mut build_waits = Vec::new();
    let mut later = Vec::new();

    for (i, sec) in rf.sections.iter().enumerate() {
        let heading = sec.heading_text.as_str();
        match qstate(led, sec) {
            QState::Skeleton | QState::Unasked if moot.contains(&i) => {
                moot_lines.push(msg::next_moot(&rf.rel, heading));
            }
            QState::Skeleton | QState::Unasked if Some(tiers[i]) != open_tier => {
                later.push(heading.to_string());
            }
            QState::Skeleton | QState::Unasked if sec.is_skeleton => {
                bodies.push(msg::next_write_body(heading));
            }
            QState::Skeleton | QState::Unasked => to_ask.push(heading.to_string()),
            QState::Open => {
                let Some(id) = &sec.id else { continue };
                // Edited since it was asked: `answer` takes only the text
                // the ledger holds, so the agent asks it again first, once
                // `ask` takes it (it asks the open tier only).
                if !asked_text_stands(led, rf, sec) {
                    if Some(tiers[i]) == open_tier {
                        to_ask.push(heading.to_string());
                    } else {
                        later.push(heading.to_string());
                    }
                } else {
                    let version = record::version_of(&rf.lines, sec);
                    answers.push(msg::next_answer(id, &version, heading));
                }
            }
            QState::Refused => held.push(msg::next_refused(heading)),
            QState::Answered => {
                let Some(id) = &sec.id else { continue };
                if tests.get(id).is_some_and(|locations| !locations.is_empty()) {
                    continue;
                }
                // A question beside it that still waits for an answer is
                // named ahead of a refused one: the refused one has its own
                // line, and the answer is what the build waits for longest.
                match state::unresolved_sibling(rf, i) {
                    Some(s) if s.is_refused() => {
                        build_waits.push(msg::next_build_waits_refused(heading, &s.heading_text))
                    }
                    Some(s) => build_waits.push(msg::next_build_waits(heading, &s.heading_text)),
                    None => build.push(msg::next_build(heading, id)),
                }
            }
            QState::Superseded | QState::Withdrawn => {}
        }
    }

    // Per "Does a depends entry carry the answers it holds under?": every
    // entry that names a question about to be asked carries its letters
    // before `ask` runs. Named once the target has its options.
    let mut sets = Vec::new();
    for (ti, target) in rf.sections.iter().enumerate() {
        if Some(tiers[ti]) != open_tier || target.is_skeleton || !state::tier_unresolved(target) {
            continue;
        }
        for (di, set) in rf.dependents_of(ti) {
            if set.is_none() {
                sets.push(msg::next_answer_set(
                    &rf.sections[di].heading_text,
                    &target.heading_text,
                ));
            }
        }
    }

    // What is answered is built before anything new is written: the commit
    // that carries an answer carries its code and its test.
    let mut out = build;
    let refused = !clears.is_empty();
    out.extend(clears);
    // A question with no case left is dropped before anything is asked:
    // `ask` refuses the whole run while it stands.
    out.extend(moot_lines);
    if plan_ok {
        out.extend(bodies);
        out.extend(sets);
        if !to_ask.is_empty() {
            out.push(msg::next_ask(&rf.rel, &to_ask));
        }
    } else if !refused {
        // Nothing is written or asked under a plan that waits for its
        // approval; what the owner owes and what is answered stands. The
        // owner's command is named once `approve` would take the plan.
        out.push(msg::next_plan_unapproved(&rf.rel));
    }
    out.extend(answers);
    out.extend(held);
    out.extend(build_waits);
    if plan_ok && !later.is_empty() {
        out.push(msg::next_later(&later));
    }
    out
}

/// The text following `prefix` (e.g. `"Problem: "`) on the first matching
/// line in `sec`'s body, trimmed, with the prefix itself left out.
fn body_field(lines: &[String], sec: &Section, prefix: &str) -> Option<String> {
    lines[sec.start..sec.end.min(lines.len())]
        .iter()
        .find_map(|l| l.strip_prefix(prefix).map(|v| v.trim().to_string()))
}

/// `open_tier`: the lowest derived tier holding an unresolved section
/// (`state::open_tier` -- text-only, a skeleton heading with no answer
/// keeps its tier open exactly as a refused or still-open question does),
/// or `None` when every tier is clear or tier derivation failed
/// (`RecordFile::derive_error`; `check`/`approve`/`ask` report why). This
/// is the "Tier N is open" tier the overview names.
///
/// `waits_on`: the heading `state::tier_blocker` names for `open_tier` --
/// the same section `ask`'s tier-blocked refusal would name -- or `None`
/// when there is no open tier at all.
fn open_tier_and_waits_on(rf: &RecordFile) -> (Option<usize>, Option<String>) {
    let open_tier = state::open_tier(rf);
    let waits_on = open_tier
        .and_then(|t| state::tier_blocker(rf, t))
        .map(|s| s.heading_text.clone());
    (open_tier, waits_on)
}

/// Every word=term pairing for `sec` (from its terms line if it has one,
/// else computed live over its text) plus the distinct words in that text
/// that pair no term (7.9): not common English, not a uuid or number, not
/// shorter than three letters, and not covered by a pairing. A word pairs
/// to a term only by its name or an explicit alias
/// (521e6211-344a-4b56-a1aa-07be956b3b29, mirrored from `terms::resolve`):
/// an un-aliased plural of a term name is not covered, and still shows up
/// here as "no term".
fn section_words(
    rf: &RecordFile,
    sec: &Section,
    terms_list: &[Term],
) -> (Vec<WordPair>, Vec<String>) {
    let text = state::section_text_excluding_terms_line(&rf.lines, sec);
    let pairs: Vec<(String, String)> = match sec.terms_line {
        Some(tl) => terms::parse_terms_line(rf.lines[tl].trim_end()).unwrap_or_default(),
        None => terms::resolve(&text, terms_list),
    };

    let mut words = Vec::new();
    for (w, tid) in &pairs {
        if let Some(t) = terms_list
            .iter()
            .find(|t| t.id.as_deref() == Some(tid.as_str()))
        {
            words.push(WordPair {
                word: w.clone(),
                term_name: t.name.clone(),
                definition: t.definition.clone(),
            });
        }
    }

    let no_entry = no_entry_words(&text, &pairs);
    (words, no_entry)
}

/// Removes every dashed uuid (8-4-4-4-12 hex, case-insensitive, on a
/// non-alphanumeric boundary) from `text`, replacing each with a single
/// space. `tokenize` splits a dashed uuid into hex fragments at the
/// hyphens, so without this pass those fragments (not 32 characters long,
/// and not necessarily all-digit) would otherwise show up as "no term"
/// words.
fn strip_uuid_tokens(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        if let Some(end) = match_uuid_at(&chars, i) {
            let before_ok = i == 0 || !chars[i - 1].is_alphanumeric();
            let after_ok = end == n || !chars[end].is_alphanumeric();
            if before_ok && after_ok {
                out.push(' ');
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// If a uuid (8-4-4-4-12 hex groups separated by `-`) starts at `start`,
/// returns the index just past it (`record::is_uuid`, run on the
/// fixed-length candidate slice this shape always is).
fn match_uuid_at(chars: &[char], start: usize) -> Option<usize> {
    let end = start + 36;
    if end > chars.len() {
        return None;
    }
    let candidate: String = chars[start..end].iter().collect();
    record::is_uuid(&candidate).then_some(end)
}

/// A word counts as common (and so never shows up as "No term") when it
/// is in `common_words()` itself, or becomes a listed word once one of
/// these trailing suffixes is stripped: "s" (accepts -> accept, stays ->
/// stay, shapes -> shape, leaves -> leave), "es", "ed" (picked -> pick),
/// "d", or "ing" (staying -> stay, picking -> pick). For "ed" and "ing"
/// only, if stripping the suffix doesn't land on a listed word, trying
/// again with an "e" appended does (shaped/shaping -> "shap" + "e" ->
/// shape). This keeps the shipped word list to base forms instead of
/// spelling out every inflection.
fn is_common_word(w: &str) -> bool {
    let common = common_words();
    if common.contains(w) {
        return true;
    }
    for suf in ["s", "es", "ed", "d", "ing"] {
        if let Some(stripped) = w.strip_suffix(suf) {
            if common.contains(stripped) {
                return true;
            }
            if suf == "ed" || suf == "ing" {
                let with_e = format!("{}e", stripped);
                if common.contains(with_e.as_str()) {
                    return true;
                }
            }
        }
    }
    false
}

fn no_entry_words(text: &str, pairs: &[(String, String)]) -> Vec<String> {
    let mut pair_words: HashSet<String> = HashSet::new();
    for (w, _) in pairs {
        for part in w.to_lowercase().split_whitespace() {
            pair_words.insert(part.to_string());
        }
    }

    let stripped = strip_uuid_tokens(text);
    let mut found: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in stripped.split('\n') {
        if crate::tells::starts_with_any(line, &[crate::tells::ID_PREFIX, crate::tells::SUPERSEDES_PREFIX]) {
            continue;
        }
        for tok in crate::text::word_runs(line) {
            let w = tok.to_lowercase();
            if w.chars().count() < 3 {
                continue;
            }
            if w.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            if is_common_word(&w) {
                continue;
            }
            if pair_words.contains(&w) {
                continue;
            }
            found.insert(w);
        }
    }
    found.into_iter().collect()
}

fn build_question_view(
    rf: &RecordFile,
    idx: usize,
    sec: &Section,
    led: &Ledger,
    test_idx: &HashMap<String, Vec<String>>,
    terms_list: &[Term],
    all_headings: &HashMap<String, String>,
) -> QuestionView {
    let state = qstate(led, sec);
    let tier = rf.tier_of(idx);
    let depends: Vec<String> = sec
        .depends
        .iter()
        .map(|d| format_dep(d, rf, terms_list, all_headings))
        .collect();
    let problem = body_field(&rf.lines, sec, "Problem: ");
    let standard = body_field(&rf.lines, sec, "Standard: ");
    let options = record::option_details(&rf.lines, sec)
        .into_iter()
        .map(|(letter, why, costs)| {
            let sentence = sec
                .options
                .iter()
                .find(|o| o.letter == letter)
                .and_then(|o| o.sentence.clone())
                .unwrap_or_default();
            OptionView {
                letter,
                sentence,
                why,
                costs,
            }
        })
        .collect();
    let tests = sec
        .id
        .as_ref()
        .and_then(|id| test_idx.get(id))
        .map(|v| v.len())
        .unwrap_or(0);
    let (words, no_entry) = section_words(rf, sec, terms_list);
    let version = record::version_of(&rf.lines, sec);

    QuestionView {
        id: sec.id.clone(),
        heading: sec.heading_text.clone(),
        version,
        tier,
        depends,
        state,
        problem,
        standard,
        options,
        answer: sec.answer_letter(),
        supersedes: sec
            .supersedes
            .iter()
            .map(|id| all_headings.get(id).cloned().unwrap_or_else(|| id.clone()))
            .collect(),
        tests,
        words,
        no_entry,
    }
}

/// `plan_approved`/`context_approved` are supplied by the caller rather
/// than computed here: `snapshot` must read them off `rf` before its own
/// cross-file normalize pass (`depends::normalize_cross_file_deps`) mutates
/// `rf`'s parsed `depends:` entries in memory (plan hashing covers them),
/// so it always had to capture them ahead of calling this and hand them
/// in. Computing them again in here, from the by-then-normalized `rf`,
/// would only ever be wrong -- every caller must overwrite it with the
/// value it already had -- so there is nothing here left to get wrong or
/// forget to overwrite.
#[allow(clippy::too_many_arguments)]
fn build_file_view(
    root: &Path,
    rf: &RecordFile,
    led: &Ledger,
    test_idx: &HashMap<String, Vec<String>>,
    terms_list: &[Term],
    all_headings: &HashMap<String, String>,
    plan_approved: bool,
    context_approved: bool,
    all: &[RecordFile],
) -> FileView {
    let path = state::rel_path(root, &rf.path);
    let done = state::file_done(rf, plan_approved, context_approved);
    let (open_tier, waits_on) = open_tier_and_waits_on(rf);
    let moot = moot_notes(rf, led, all);
    let questions = rf
        .sections
        .iter()
        .enumerate()
        .map(|(idx, sec)| build_question_view(rf, idx, sec, led, test_idx, terms_list, all_headings))
        .collect();

    FileView {
        path,
        topic: rf.topic.clone().unwrap_or_default(),
        plan_approved,
        context_approved,
        done,
        open_tier,
        waits_on,
        moot,
        questions,
    }
}

/// "circular definition: a -> b -> a" (the cycle's names, in traversal
/// order, closed by repeating the first at the end).
fn format_cycle(names: &[String]) -> String {
    format!("circular definition: {}", crate::msg::cycle_chain(names))
}

/// Every current decision's terms-line pairing, term id -> its dependents'
/// labels (a decision counted once per term even if its line pairs the
/// term through more than one word). The narrowest reach of the five
/// `state::dependent_entries` readers: decisions' own terms lines only, no
/// `depends:` ids and no term-to-term pairing.
fn dependents_by_entry(root: &Path, led: &Ledger, tf: &terms::TermsFile) -> HashMap<String, Vec<String>> {
    let records = record::load_all(root);
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for dep in state::dependent_entries(&records, led, tf, false, false) {
        let label = dep.source.label();
        let mut seen_here: HashSet<String> = HashSet::new();
        for (_, tid) in dep.pairs {
            if seen_here.insert(tid.clone()) {
                dependents.entry(tid).or_default().push(label.clone());
            }
        }
    }
    dependents
}

/// Terms and dependents-per-term.
fn build_terms_view(root: &Path, led: &Ledger, tf: terms::TermsFile) -> TermsView {
    let approved = state::terms_approved(led, &tf);
    let dependents = dependents_by_entry(root, led, &tf);

    let mut errors = tf.errors.clone();
    for c in terms::cycles(&tf) {
        errors.push(format_cycle(&c));
    }

    let entries: Vec<TermEntryView> = tf
        .terms
        .iter()
        .map(|t| TermEntryView {
            name: t.name.clone(),
            retired: t.retired,
            definition: t.definition.clone(),
            aliases: t.aliases.clone(),
            dependents: t
                .id
                .as_ref()
                .and_then(|id| dependents.get(id))
                .map(|v| v.len())
                .unwrap_or(0),
        })
        .collect();

    let count = tf.terms.len();
    TermsView {
        path: repo::TERMS_PATH.to_string(),
        approved,
        count,
        errors,
        entries,
    }
}

pub fn snapshot(root: &Path) -> Snapshot {
    let ledger_path = root.join(repo::LEDGER_PATH);
    let led = ledger::read(&ledger_path);
    let test_idx = check::test_index(root, &crate::repo::Source::WorkingTree);

    let terms_opt = state::load_terms(root);
    let terms_list: Vec<Term> = terms_opt
        .as_ref()
        .map(|tf| tf.terms.clone())
        .unwrap_or_default();

    let all_headings = all_heading_texts(root);
    // Same pre-pass `approve`/`check` run before reading `rf.tiers`/
    // `rf.derive_error` (`depends::normalize_cross_file_deps`): a
    // heading-shaped `depends:` entry naming a current decision's heading
    // in another topic file is rewritten in memory to that decision's id
    // and the tiers re-derived, so an unapproved file with such an entry
    // still shows its real tiers on the page instead of the file-wide
    // "no heading in this file" error a plain, unresolved `Text` heading
    // dep would otherwise earn. The ambiguous case (the same heading text
    // naming a current decision in more than one other file) becomes that
    // file's `derive_error` (`DeriveError::AmbiguousHeading`) instead of
    // tiers -- `check`'s own report (already folded into `report.failures`
    // below) is still where the ambiguity itself is reported; the page
    // just shows no tiers for that file rather than guessing.
    let phrase_lookup = terms::phrase_lookup(&terms_list);
    // Every topic file, loaded once: `cross_file_heading_index` used to
    // reload every one of them itself, then this loop reloaded whichever
    // one it was about to build a `FileView` for all over again.
    let records: Vec<RecordFile> = record::load_all(root);
    let cross_file_headings = depends::cross_file_heading_index(&records, &led);
    let all_records = records.clone();
    let mut files: Vec<FileView> = Vec::new();
    for mut rf in records {
        let rel = rf.rel.clone();
        // Plan/context approval state must be read off `rf` exactly as
        // loaded, before the normalize pass below mutates its parsed
        // `depends:` entries in memory (per "..."): captured here, ahead of
        // that mutation, and passed straight into `build_file_view` (which
        // takes them as parameters rather than computing its own, wrong,
        // copy off the by-then-normalized `rf`).
        let plan_ok = state::plan_approved(&led, &rf, &rel);
        let ctx_ok = state::context_approved(&led, &rf, &rel);
        depends::normalize_cross_file_deps(&mut rf, &terms_list, &phrase_lookup, &cross_file_headings, &led);
        let fv = build_file_view(
            root,
            &rf,
            &led,
            &test_idx,
            &terms_list,
            &all_headings,
            plan_ok,
            ctx_ok,
            &all_records,
        );
        files.push(fv);
    }

    let terms = terms_opt.map(|tf| build_terms_view(root, &led, tf));

    let report = check::collect_with_tests(root, &test_idx);
    let check_report = CheckReport {
        failures: report.failures,
        notes: report.notes,
    };

    Snapshot {
        files,
        terms,
        check: check_report,
    }
}

/// Every Open question, in file order, paired with its file's path.
pub fn open_questions(s: &Snapshot) -> Vec<(String, &QuestionView)> {
    let mut out = Vec::new();
    for f in &s.files {
        for q in &f.questions {
            if q.state == QState::Open {
                out.push((f.path.clone(), q));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn is_common_word_covers_inflected_common_words() {
        assert!(is_common_word("accepts"), "accepts -> accept");
        assert!(is_common_word("picked"), "picked -> pick");
        assert!(is_common_word("stays"), "stays -> stay");
        assert!(is_common_word("shapes"), "shapes -> shape");
        assert!(is_common_word("beside"), "beside is a plain base word");
    }

    #[test]
    fn is_common_word_does_not_swallow_real_no_term_words() {
        assert!(!is_common_word("principle"));
        assert!(!is_common_word("constraint"));
    }

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_repo() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "adrdom-view-test-{}-{}-{}",
            std::process::id(),
            n,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join(repo::DECISIONS_DIR)).unwrap();
        std::fs::create_dir_all(dir.join(repo::STATE_DIR)).unwrap();
        Command::new("git")
            .current_dir(&dir)
            .args(["init", "-q"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(&dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(&dir)
            .args(["config", "user.name", "Test"])
            .output()
            .unwrap();
        std::fs::write(dir.join(repo::LEDGER_PATH), "").unwrap();
        dir
    }

    fn write_record(dir: &Path, rel: &str, content: &str) -> PathBuf {
        let path = dir.join(rel);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn append_row(dir: &Path, event: &str, id: &str, hash: &str, detail: &str) {
        let path = dir.join(repo::LEDGER_PATH);
        let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
        existing.push_str(&format!(
            "2024-01-01T00:00:00Z\t{}\t{}\t{}\t{}\n",
            event, id, hash, detail
        ));
        std::fs::write(&path, existing).unwrap();
    }

    /// Same as `append_row`, but with an explicit `time`: per "Does the
    /// tool take the order of ledger rows from their time?", two rows about
    /// the same subject sharing `append_row`'s one fixed time are ordered
    /// by row text, not by which one a fixture wrote first, so a fixture
    /// whose point is which row is "the latest" (an `asked` row and the
    /// `answered`/`superseded`/`withdrawn` row that follows it) needs each
    /// row to carry a time later than the one before it.
    fn append_row_at(dir: &Path, event: &str, id: &str, hash: &str, detail: &str, time: &str) {
        let path = dir.join(repo::LEDGER_PATH);
        let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
        existing.push_str(&format!("{}\t{}\t{}\t{}\t{}\n", time, event, id, hash, detail));
        std::fs::write(&path, existing).unwrap();
    }

    const TWO_GROUP: &str = "# Topic\n\n\
Context prose.\n\n\
### 1.1 First question?\n\n\
id: id-1\n\n\
Problem: p1.\n\n\
Standard: s1.\n\n\
- A) Answer one.\n  Why: reason one.\n  Costs: cost one.\n\
- B) Answer two.\n  Why: reason two.\n\n\
### 2.1 Second question?\n\
depends: 1.1 First question?\n\n\
Problem: p2.\n\n\
Standard: s2.\n\n\
- A) Second answer.\n  Why: reasons.\n";

    #[test]
    fn qstate_maps_ledger_states() {
        let dir = temp_repo();
        let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP);
        append_row(&dir, "asked", "id-1", "hash-1", "1.1 First question?");

        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join(repo::LEDGER_PATH));

        // 1.1 has id-1, asked -> Open.
        let sec1 = rf
            .sections
            .iter()
            .find(|s| s.heading_text == "1.1 First question?")
            .unwrap();
        assert_eq!(qstate(&led, sec1), QState::Open);

        // 2.1 has no id (never asked) but has a body -> Unasked.
        let sec2 = rf
            .sections
            .iter()
            .find(|s| s.heading_text == "2.1 Second question?")
            .unwrap();
        assert_eq!(qstate(&led, sec2), QState::Unasked);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn qstate_answered_superseded_and_withdrawn() {
        let dir = temp_repo();
        let led_path = dir.join(repo::LEDGER_PATH);
        append_row_at(&dir, "asked", "id-a", "h", "A?", "2024-01-01T00:00:00Z");
        append_row_at(&dir, "answered", "id-a", "h", "A", "2024-01-01T00:00:01Z");
        append_row_at(&dir, "asked", "id-b", "h", "B?", "2024-01-01T00:00:02Z");
        append_row_at(&dir, "answered", "id-b", "h", "A", "2024-01-01T00:00:03Z");
        append_row_at(&dir, "superseded", "id-b", "-", "id-c", "2024-01-01T00:00:04Z");
        append_row_at(&dir, "asked", "id-d", "h", "D?", "2024-01-01T00:00:05Z");
        append_row_at(&dir, "withdrawn", "id-d", "-", "D?", "2024-01-01T00:00:06Z");
        let led = ledger::read(&led_path);

        let mk = |id: &str| Section {
            start: 0,
            end: 1,
            heading_text: "x".to_string(),
            id: Some(id.to_string()),
            supersedes: vec![],
            depends: vec![],
            options: vec![],
            answer_line: None,
            answer_value: None,
            parse_errors: vec![],
            shape_errors: vec![],
            is_skeleton: false,
            terms_line: None,
        };

        assert_eq!(qstate(&led, &mk("id-a")), QState::Answered);
        assert_eq!(qstate(&led, &mk("id-b")), QState::Superseded);
        assert_eq!(qstate(&led, &mk("id-d")), QState::Withdrawn);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn qstate_maps_refused() {
        let dir = temp_repo();
        append_row(&dir, "asked", "id-r", "h", "R?");
        append_row(&dir, "refused", "id-r", "h", "R?");
        let led = ledger::read(&dir.join(repo::LEDGER_PATH));

        let sec = Section {
            start: 0,
            end: 1,
            heading_text: "x".to_string(),
            id: Some("id-r".to_string()),
            supersedes: vec![],
            depends: vec![],
            options: vec![],
            answer_line: None,
            answer_value: None,
            parse_errors: vec![],
            shape_errors: vec![],
            is_skeleton: false,
            terms_line: None,
        };
        assert_eq!(qstate(&led, &sec), QState::Refused);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skeleton_heading_has_no_id_maps_to_skeleton() {
        let dir = temp_repo();
        let content = "# Topic\n\n### 1.1 Not written yet?\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join(repo::LEDGER_PATH));
        assert_eq!(qstate(&led, &rf.sections[0]), QState::Skeleton);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn open_tier_and_waits_on_for_unanswered_tier_one() {
        let dir = temp_repo();
        let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP);

        let rf = record::load(&record).unwrap();

        let (open_tier, waits_on) = open_tier_and_waits_on(&rf);
        // Tier 1's question has no letter answer, so it is the lowest open
        // tier.
        assert_eq!(open_tier, Some(1));
        // Tier 2's question has no answer either, but tier 1's is earlier
        // and also unresolved: that is what `ask` would name.
        assert_eq!(waits_on.as_deref(), Some("1.1 First question?"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn waits_on_names_tier_twos_own_question_once_tier_one_is_answered() {
        let dir = temp_repo();
        // Q1 gets a real `**Answer:**` line; Q2 stays unanswered.
        let content = TWO_GROUP.replacen(
            "\n\n### 2.1 Second question?",
            "\n\n**Answer:** A\n\n### 2.1 Second question?",
            1,
        );
        let record = write_record(&dir, "adrdom/decisions/example.md", &content);

        let rf = record::load(&record).unwrap();

        let (open_tier, waits_on) = open_tier_and_waits_on(&rf);
        // Tier one is fully answered; tier two (unanswered) is now the
        // lowest open tier, and its own question is what `waits_on` names
        // -- the same section `ask` would name if a later tier existed and
        // tried to jump ahead of it.
        assert_eq!(open_tier, Some(2));
        assert_eq!(waits_on.as_deref(), Some("2.1 Second question?"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn open_tier_counts_a_refused_question_as_unresolved() {
        let dir = temp_repo();
        // Q1 is refused in the file text itself (a refused `**Answer:**`
        // line never parses as a letter, so it stays unresolved).
        let content = TWO_GROUP.replacen(
            "\n\n### 2.1 Second question?",
            "\n\n**Answer:** refused\n\n### 2.1 Second question?",
            1,
        );
        let record = write_record(&dir, "adrdom/decisions/example.md", &content);

        let rf = record::load(&record).unwrap();

        let (open_tier, waits_on) = open_tier_and_waits_on(&rf);
        // Tier 1's question is refused, not answered: tier 1 still counts
        // as the open (stuck) tier, and it still blocks tier 2 by name.
        assert_eq!(open_tier, Some(1));
        assert_eq!(waits_on.as_deref(), Some("1.1 First question?"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn options_carry_why_and_costs() {
        let dir = temp_repo();
        let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP);
        let rf = record::load(&record).unwrap();
        let sec = &rf.sections[0];
        let details = record::option_details(&rf.lines, sec);
        assert_eq!(details.len(), 2);
        assert_eq!(
            details[0],
            (
                'A',
                Some("reason one.".to_string()),
                Some("cost one.".to_string())
            )
        );
        assert_eq!(details[1], ('B', Some("reason two.".to_string()), None));
        std::fs::remove_dir_all(&dir).ok();
    }

    const TWO_GROUP_ANSWERED: &str = "# Topic\n\n\
Context prose.\n\n\
### 1.1 First question?\n\n\
id: id-1\n\n\
Problem: p1.\n\n\
Standard: s1.\n\n\
- A) Answer one.\n  Why: reason one.\n  Costs: cost one.\n\
- B) Answer two.\n  Why: reason two.\n\n\
**Answer:** A\n\n\
### 2.1 Second question?\n\
depends: 1.1 First question?\n\n\
id: id-2\n\n\
Problem: p2.\n\n\
Standard: s2.\n\n\
- A) Second answer.\n  Why: reasons.\n\n\
**Answer:** A\n";

    #[test]
    fn done_true_only_when_approved_and_every_question_answered() {
        let dir = temp_repo();
        let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP);
        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join(repo::LEDGER_PATH));
        let test_idx = HashMap::new();

        // Nothing approved yet, nothing answered: not done.
        let path = state::rel_path(&dir, &rf.path);
        let plan_ok = state::plan_approved(&led, &rf, &path);
        let ctx_ok = state::context_approved(&led, &rf, &path);
        let fv = build_file_view(
            &dir,
            &rf,
            &led,
            &test_idx,
            &[],
            &HashMap::new(),
            plan_ok,
            ctx_ok,
            std::slice::from_ref(&rf));
        assert!(!fv.done);

        // Approve plan/context, then rewrite the file with every section
        // answered: now it is done. The fixture bakes its own ids in
        // (`id: id-1`/`id: id-2`), and `plan_hash` covers a heading's id
        // line (per "Does a reference in a topic file carry the identifier
        // of what it names?"), so the hashes are computed from the id-bearing
        // (already-answered) fixture, not the id-less base one -- an
        // answer line itself never moves either hash.
        std::fs::write(&record, TWO_GROUP_ANSWERED).unwrap();
        let rf2 = record::load(&record).unwrap();
        let plan_h = record::plan_hash(&rf2);
        let ctx_h = record::context_hash(&rf2);
        append_row(
            &dir,
            "approved",
            "-",
            &plan_h,
            "adrdom/decisions/example.md",
        );
        append_row(&dir, "context", "-", &ctx_h, "adrdom/decisions/example.md");
        let led2 = ledger::read(&dir.join(repo::LEDGER_PATH));
        assert!(
            rf2.sections.iter().all(|s| s.answer_line.is_some()),
            "fixture should have every section answered"
        );
        let path2 = state::rel_path(&dir, &rf2.path);
        let plan_ok2 = state::plan_approved(&led2, &rf2, &path2);
        let ctx_ok2 = state::context_approved(&led2, &rf2, &path2);
        let fv2 = build_file_view(
            &dir,
            &rf2,
            &led2,
            &test_idx,
            &[],
            &HashMap::new(),
            plan_ok2,
            ctx_ok2,
            std::slice::from_ref(&rf2));
        assert!(fv2.plan_approved);
        assert!(fv2.context_approved);
        assert!(fv2.done);

        std::fs::remove_dir_all(&dir).ok();
    }

    const TWO_GROUP_ONE_REFUSED: &str = "# Topic\n\n\
Context prose.\n\n\
### 1.1 First question?\n\n\
id: id-1\n\n\
Problem: p1.\n\n\
Standard: s1.\n\n\
- A) Answer one.\n  Why: reason one.\n  Costs: cost one.\n\
- B) Answer two.\n  Why: reason two.\n\n\
**Answer:** A\n\n\
### 2.1 Second question?\n\
depends: 1.1 First question?\n\n\
id: id-2\n\n\
Problem: p2.\n\n\
Standard: s2.\n\n\
- A) Second answer.\n  Why: reasons.\n\n\
**Answer:** refused\n";

    #[test]
    fn done_is_false_when_a_question_is_refused_not_answered() {
        let dir = temp_repo();
        let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP);
        let test_idx = HashMap::new();

        // Approve plan/context from the id-bearing fixture (an answer or
        // refusal line never moves either hash, but the fixture's own
        // `id: id-1`/`id: id-2` lines do, since `plan_hash` covers them --
        // per "Does a reference in a topic file carry the identifier of
        // what it names?"), then rewrite the file with one question
        // answered and the other refused.
        std::fs::write(&record, TWO_GROUP_ONE_REFUSED).unwrap();
        let rf2 = record::load(&record).unwrap();
        let plan_h = record::plan_hash(&rf2);
        let ctx_h = record::context_hash(&rf2);
        append_row(
            &dir,
            "approved",
            "-",
            &plan_h,
            "adrdom/decisions/example.md",
        );
        append_row(&dir, "context", "-", &ctx_h, "adrdom/decisions/example.md");
        let led2 = ledger::read(&dir.join(repo::LEDGER_PATH));
        assert!(
            rf2.sections.iter().all(|s| s.answer_line.is_some()),
            "fixture should have every section carrying an Answer line (one letter, one refused)"
        );

        let path2 = state::rel_path(&dir, &rf2.path);
        let plan_ok2 = state::plan_approved(&led2, &rf2, &path2);
        let ctx_ok2 = state::context_approved(&led2, &rf2, &path2);
        let fv = build_file_view(
            &dir,
            &rf2,
            &led2,
            &test_idx,
            &[],
            &HashMap::new(),
            plan_ok2,
            ctx_ok2,
            std::slice::from_ref(&rf2));
        assert!(fv.plan_approved);
        assert!(fv.context_approved);
        assert!(
            !fv.done,
            "a refused question must not count as answered for done"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn format_dep_prints_words_for_text_and_id_entries() {
        let dir = temp_repo();
        let content = "# Topic\n\n\
### 1.1 Q?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
### 1.2 R?\ndepends: 11111111-1111-1111-1111-111111111111=A|B\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        let rf = record::load(&record).unwrap();

        let terms_list = vec![Term {
            name: "widget".to_string(),
            id: Some("22222222-2222-2222-2222-222222222222".to_string()),
            definition: String::new(),
            aliases: vec![],
            retired: false,
            start_line: 1,
            id_line: None,
            terms_line: None,
            terms_line_idx: None,
        }];

        // A `Text` entry (not yet approved) prints as written.
        assert_eq!(
            format_dep(&rf.sections[0].depends[0], &rf, &terms_list, &HashMap::new()),
            "widget"
        );

        // An `Id` entry naming a heading in this same file prints that
        // heading's text with the set suffix restored.
        assert_eq!(
            format_dep(&rf.sections[1].depends[0], &rf, &terms_list, &HashMap::new()),
            "1.1 Q?=A|B"
        );

        // An `Id` entry naming a term prints the term's name.
        let term_dep = record::Dep::Id {
            id: "22222222-2222-2222-2222-222222222222".to_string(),
            set: None,
        };
        assert_eq!(
            format_dep(&term_dep, &rf, &terms_list, &HashMap::new()),
            "widget"
        );

        // An `Id` entry naming a decision in another topic file prints that
        // decision's heading text, from `all_headings`.
        let mut all_headings = HashMap::new();
        all_headings.insert(
            "33333333-3333-3333-3333-333333333333".to_string(),
            "Elsewhere?".to_string(),
        );
        let decision_dep = record::Dep::Id {
            id: "33333333-3333-3333-3333-333333333333".to_string(),
            set: None,
        };
        assert_eq!(
            format_dep(&decision_dep, &rf, &terms_list, &all_headings),
            "Elsewhere?"
        );

        // An id resolving to none of the three (should not happen once
        // approved) prints the bare id rather than hiding it.
        let unknown_dep = record::Dep::Id {
            id: "44444444-4444-4444-4444-444444444444".to_string(),
            set: None,
        };
        assert_eq!(
            format_dep(&unknown_dep, &rf, &terms_list, &HashMap::new()),
            "44444444-4444-4444-4444-444444444444"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn question_view_carries_tier_and_depends_as_written() {
        let dir = temp_repo();
        let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP);
        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join(repo::LEDGER_PATH));
        let test_idx = HashMap::new();

        let path = state::rel_path(&dir, &rf.path);
        let plan_ok = state::plan_approved(&led, &rf, &path);
        let ctx_ok = state::context_approved(&led, &rf, &path);
        let fv = build_file_view(
            &dir,
            &rf,
            &led,
            &test_idx,
            &[],
            &HashMap::new(),
            plan_ok,
            ctx_ok,
            std::slice::from_ref(&rf));
        let q1 = fv
            .questions
            .iter()
            .find(|q| q.heading == "1.1 First question?")
            .unwrap();
        assert_eq!(q1.tier, Some(1));
        assert!(q1.depends.is_empty());

        let q2 = fv
            .questions
            .iter()
            .find(|q| q.heading == "2.1 Second question?")
            .unwrap();
        assert_eq!(q2.tier, Some(2));
        assert_eq!(q2.depends, vec!["1.1 First question?".to_string()]);

        std::fs::remove_dir_all(&dir).ok();
    }

    const MOOT_FIXTURE: &str = "# Topic\n\n\
### 1.1 Root?\n\n\
id: id-root\n\n\
Problem: p.\n\n\
Standard: s.\n\n\
- A) First choice.\n  Why: w.\n  Costs: c.\n\
- B) Second choice.\n  Why: w.\n  Costs: c.\n\n\
**Answer:** B\n\n\
### 1.2 Dependent?\n\
depends: 1.1 Root?=A\n\n\
id: id-dep\n\n\
Problem: p2.\n\n\
Standard: s2.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n";

    #[test]
    fn moot_notes_and_file_view_moot_name_the_excluded_answer() {
        let dir = temp_repo();
        let record = write_record(&dir, "adrdom/decisions/example.md", MOOT_FIXTURE);
        append_row_at(&dir, "asked", "id-root", "h", "1.1 Root?", "2024-01-01T00:00:00Z");
        append_row_at(&dir, "answered", "id-root", "h", "B", "2024-01-01T00:00:01Z");

        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join(repo::LEDGER_PATH));

        let notes = moot_notes(&rf, &led, std::slice::from_ref(&rf));
        assert_eq!(
            notes,
            vec!["\"1.2 Dependent?\" stands on \"1.1 Root?\"=A, answered B".to_string()]
        );

        let test_idx = HashMap::new();
        let path = state::rel_path(&dir, &rf.path);
        let plan_ok = state::plan_approved(&led, &rf, &path);
        let ctx_ok = state::context_approved(&led, &rf, &path);
        let fv = build_file_view(
            &dir,
            &rf,
            &led,
            &test_idx,
            &[],
            &HashMap::new(),
            plan_ok,
            ctx_ok,
            std::slice::from_ref(&rf));
        assert_eq!(fv.moot, notes);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn moot_notes_silent_when_the_answer_is_in_the_set() {
        let dir = temp_repo();
        let content = MOOT_FIXTURE.replace("**Answer:** B", "**Answer:** A");
        let record = write_record(&dir, "adrdom/decisions/example.md", &content);
        append_row(&dir, "asked", "id-root", "h", "1.1 Root?");
        append_row(&dir, "answered", "id-root", "h", "A");

        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join(repo::LEDGER_PATH));

        assert!(moot_notes(&rf, &led, std::slice::from_ref(&rf)).is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
