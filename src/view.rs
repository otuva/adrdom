//! Everything the page shows, computed from the repo (7.1 A). No HTML here.
use crate::commands::check;
use crate::ledger::{self, Ledger};
use crate::record::{self, RecordFile, Section};
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
    /// The section's derived tier, printed as a plain number (or empty
    /// when derivation failed). Temporary: wave 2's view.rs (spec §7)
    /// replaces this field with `tier: Option<usize>`.
    pub group: String,
    pub state: QState,
    pub problem: Option<String>,
    pub standard: Option<String>,
    pub options: Vec<OptionView>,
    pub answer: Option<char>,
    pub supersedes: Vec<String>,
    pub tests: usize,
    /// Every word=term pairing on this section's terms line (answered or
    /// asked), or computed live over the section text when it has none yet.
    pub words: Vec<WordPair>,
    /// Distinct words (lowercased) in the section text that pair no term:
    /// not common English, not a uuid or number, not shorter than three
    /// letters, and not already covered (plural rule included) by `words`.
    pub no_entry: Vec<String>,
}

pub struct FileView {
    pub path: String,
    pub topic: String,
    pub plan_approved: bool,
    pub context_approved: bool,
    pub done: bool,
    pub open_group: Option<String>,
    pub waits_on: Option<String>,
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

/// A term whose definition or aliases differ from the last approved
/// (HEAD) version of `adrdom/terms.md`, shown only while the working
/// copy is unapproved.
pub struct TermDiffView {
    pub name: String,
    pub before_definition: String,
    pub after_definition: String,
    pub before_aliases: Vec<String>,
    pub after_aliases: Vec<String>,
    /// "<path>: <heading>" for every current decision carrying this term.
    pub dependents: Vec<String>,
}

pub struct TermsView {
    pub path: String,
    pub approved: bool,
    pub count: usize,
    /// Shape errors from parsing, plus one "circular definition: a -> b ->
    /// a" line per cycle `terms::cycles` finds.
    pub errors: Vec<String>,
    pub entries: Vec<TermEntryView>,
    pub diffs: Vec<TermDiffView>,
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

/// The state a question heading is in, from the ledger's latest
/// state row for its id: no id and a skeleton heading is `Skeleton`; no id
/// with a body is `Unasked`; otherwise the id's latest ledger event maps
/// straight across (`asked` -> `Open`, `answered` -> `Answered`,
/// `superseded`/`withdrawn` accordingly).
fn qstate(led: &Ledger, sec: &Section) -> QState {
    let Some(id) = &sec.id else {
        return if sec.is_skeleton {
            QState::Skeleton
        } else {
            QState::Unasked
        };
    };
    match led.latest_state(id).map(|r| r.event.as_str()) {
        Some("asked") => QState::Open,
        Some("answered") => QState::Answered,
        Some("superseded") => QState::Superseded,
        Some("withdrawn") => QState::Withdrawn,
        _ => QState::Unasked,
    }
}

/// The text following `prefix` (e.g. `"Problem: "`) on the first matching
/// line in `sec`'s body, trimmed, with the prefix itself left out.
fn body_field(lines: &[String], sec: &Section, prefix: &str) -> Option<String> {
    lines[sec.start..sec.end.min(lines.len())]
        .iter()
        .find_map(|l| l.strip_prefix(prefix).map(|v| v.trim().to_string()))
}

/// `open_group`: the derived tier number (as a string) of the lowest tier
/// that still has an Open or Unasked question in it. `waits_on`: the
/// heading of the first Open/Unasked question in an earlier tier than the
/// lowest tier that has an Unasked/Skeleton question waiting to be asked —
/// i.e. the heading `ask` would name in its tier refusal — or `None` when
/// nothing is waiting on an earlier tier.
///
/// Temporary: derived via `rf.tiers`/`rf.tier_of` (record.rs's §1) instead
/// of the removed h2 groups; wave 2's view.rs (spec §7) replaces
/// `open_group: Option<String>` with `open_tier: Option<usize>` and gives
/// this its proper "Tier N" treatment (including the derive-error case).
fn open_group_and_waits_on(rf: &RecordFile, led: &Ledger) -> (Option<String>, Option<String>) {
    let Some(tiers) = &rf.tiers else {
        return (None, None);
    };
    let max_tier = tiers.iter().copied().max().unwrap_or(0);
    let mut tier_states: Vec<Vec<(String, QState)>> = vec![Vec::new(); max_tier + 1];
    for (i, sec) in rf.sections.iter().enumerate() {
        let t = tiers[i];
        tier_states[t].push((sec.heading_text.clone(), qstate(led, sec)));
    }

    let open_tier = (1..=max_tier).find(|&t| {
        tier_states[t]
            .iter()
            .any(|(_, s)| matches!(s, QState::Open | QState::Unasked))
    });
    let open_group = open_tier.map(|t| t.to_string());

    let waiting_tier = (1..=max_tier).find(|&t| {
        tier_states[t]
            .iter()
            .any(|(_, s)| matches!(s, QState::Unasked | QState::Skeleton))
    });

    let waits_on = waiting_tier.and_then(|wt| {
        for t in 1..wt {
            for (heading, s) in &tier_states[t] {
                if matches!(s, QState::Open | QState::Unasked) {
                    return Some(heading.clone());
                }
            }
        }
        None
    });

    (open_group, waits_on)
}

/// Every word=term pairing for `sec` (from its terms line if it has one,
/// else computed live over its text) plus the distinct words in that text
/// that pair no term (7.9): not common English, not a uuid or number, not
/// shorter than three letters, and not covered by a pairing (the plural
/// rule applies: a word whose trailing "s"/"es" strips to a current name or
/// alias counts as covered even if it isn't itself a pairing's word).
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

    let no_entry = no_entry_words(&text, &pairs, terms_list);
    (words, no_entry)
}

fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
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
/// returns the index just past it.
fn match_uuid_at(chars: &[char], start: usize) -> Option<usize> {
    const GROUP_LENS: [usize; 5] = [8, 4, 4, 4, 12];
    let mut pos = start;
    for (gi, &len) in GROUP_LENS.iter().enumerate() {
        for _ in 0..len {
            if pos >= chars.len() || !chars[pos].is_ascii_hexdigit() {
                return None;
            }
            pos += 1;
        }
        if gi < GROUP_LENS.len() - 1 {
            if pos >= chars.len() || chars[pos] != '-' {
                return None;
            }
            pos += 1;
        }
    }
    Some(pos)
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

fn no_entry_words(text: &str, pairs: &[(String, String)], terms_list: &[Term]) -> Vec<String> {
    let mut pair_words: HashSet<String> = HashSet::new();
    for (w, _) in pairs {
        for part in w.to_lowercase().split_whitespace() {
            pair_words.insert(part.to_string());
        }
    }
    let mut name_alias: HashSet<String> = HashSet::new();
    for t in terms_list {
        name_alias.insert(t.name.trim().to_lowercase());
        for a in &t.aliases {
            name_alias.insert(a.trim().to_lowercase());
        }
    }

    let stripped = strip_uuid_tokens(text);
    let mut found: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in stripped.split('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("id:") || trimmed.starts_with("supersedes:") {
            continue;
        }
        for tok in tokenize(line) {
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
            let plural_covered = w
                .strip_suffix("es")
                .map(|s| name_alias.contains(s))
                .unwrap_or(false)
                || w.strip_suffix('s')
                    .map(|s| name_alias.contains(s))
                    .unwrap_or(false);
            if plural_covered {
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
) -> QuestionView {
    let state = qstate(led, sec);
    let group = rf
        .tier_of(idx)
        .map(|t| t.to_string())
        .unwrap_or_default();
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

    QuestionView {
        id: sec.id.clone(),
        heading: sec.heading_text.clone(),
        group,
        state,
        problem,
        standard,
        options,
        answer: sec.answer_letter(),
        supersedes: sec.supersedes.clone(),
        tests,
        words,
        no_entry,
    }
}

fn build_file_view(
    root: &Path,
    rf: &RecordFile,
    led: &Ledger,
    test_idx: &HashMap<String, Vec<String>>,
    terms_list: &[Term],
) -> FileView {
    let path = state::rel_path(root, &rf.path);
    let plan_approved = state::plan_approved(led, rf, &path);
    let context_approved = state::context_approved(led, rf, &path);
    let done =
        plan_approved && context_approved && rf.sections.iter().all(|s| s.answer_line.is_some());
    let (open_group, waits_on) = open_group_and_waits_on(rf, led);
    let questions = rf
        .sections
        .iter()
        .enumerate()
        .map(|(idx, sec)| build_question_view(rf, idx, sec, led, test_idx, terms_list))
        .collect();

    FileView {
        path,
        topic: rf.topic.clone().unwrap_or_default(),
        plan_approved,
        context_approved,
        done,
        open_group,
        waits_on,
        questions,
    }
}

/// "circular definition: a -> b -> a" (the cycle's names, in traversal
/// order, closed by repeating the first at the end).
fn format_cycle(names: &[String]) -> String {
    let mut chain = names.to_vec();
    if let Some(first) = names.first() {
        chain.push(first.clone());
    }
    format!("circular definition: {}", chain.join(" -> "))
}

/// Every current decision's terms-line pairing, id -> "<path>: <heading>"
/// per decision that carries it (a decision counted once per term even if
/// its line pairs the term through more than one word).
fn dependents_by_entry(root: &Path, led: &Ledger) -> HashMap<String, Vec<String>> {
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for f in repo::list_decision_files(root) {
        let Ok(rf) = record::load(&f) else { continue };
        let rel = state::rel_path(root, &rf.path);
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_current(led, id) {
                continue;
            }
            let Some(tl) = sec.terms_line else { continue };
            let pairs = terms::parse_terms_line(rf.lines[tl].trim_end()).unwrap_or_default();
            let mut seen_here: HashSet<String> = HashSet::new();
            for (_, tid) in pairs {
                if seen_here.insert(tid.clone()) {
                    dependents
                        .entry(tid)
                        .or_default()
                        .push(format!("{}: {}", rel, sec.heading_text));
                }
            }
        }
    }
    dependents
}

/// Terms, dependents-per-term, and (while unapproved) before/after diffs
/// against the last approved (HEAD) version of `adrdom/terms.md` (7.9).
fn build_terms_view(root: &Path, led: &Ledger, tf: terms::TermsFile) -> TermsView {
    let approved = state::terms_approved(led, &tf);
    let dependents = dependents_by_entry(root, led);

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

    let mut diffs: Vec<TermDiffView> = Vec::new();
    if !approved {
        if let Some(latest) = led.latest_for_path("terms", "adrdom/terms.md") {
            if let Some(head_content) = repo::git(root, &["show", "HEAD:adrdom/terms.md"]) {
                let head_tf = terms::parse(&record::split_lines(&head_content));
                if terms::file_hash(&head_tf) == latest.hash {
                    for t in &tf.terms {
                        let old =
                            t.id.as_ref()
                                .and_then(|id| {
                                    head_tf
                                        .terms
                                        .iter()
                                        .find(|o| o.id.as_deref() == Some(id.as_str()))
                                })
                                .or_else(|| head_tf.terms.iter().find(|o| o.name == t.name));
                        let Some(old_t) = old else { continue };
                        if old_t.definition == t.definition && old_t.aliases == t.aliases {
                            continue;
                        }
                        diffs.push(TermDiffView {
                            name: t.name.clone(),
                            before_definition: old_t.definition.clone(),
                            after_definition: t.definition.clone(),
                            before_aliases: old_t.aliases.clone(),
                            after_aliases: t.aliases.clone(),
                            dependents: t
                                .id
                                .as_ref()
                                .and_then(|id| dependents.get(id))
                                .cloned()
                                .unwrap_or_default(),
                        });
                    }
                }
            }
        }
    }

    let count = tf.terms.len();
    TermsView {
        path: "adrdom/terms.md".to_string(),
        approved,
        count,
        errors,
        entries,
        diffs,
    }
}

pub fn snapshot(root: &Path) -> Snapshot {
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);
    let test_idx = check::test_index(root);

    let terms_opt = state::load_terms(root);
    let terms_list: Vec<Term> = terms_opt
        .as_ref()
        .map(|tf| tf.terms.clone())
        .unwrap_or_default();

    let mut files: Vec<FileView> = Vec::new();
    for f in repo::list_decision_files(root) {
        if let Ok(rf) = record::load(&f) {
            files.push(build_file_view(root, &rf, &led, &test_idx, &terms_list));
        }
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
        std::fs::create_dir_all(dir.join("adrdom/decisions")).unwrap();
        std::fs::create_dir_all(dir.join("adrdom/state")).unwrap();
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
        std::fs::write(dir.join("adrdom/state/ledger.tsv"), "").unwrap();
        dir
    }

    fn write_record(dir: &Path, rel: &str, content: &str) -> PathBuf {
        let path = dir.join(rel);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn append_row(dir: &Path, event: &str, id: &str, hash: &str, detail: &str) {
        let path = dir.join("adrdom/state/ledger.tsv");
        let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
        existing.push_str(&format!(
            "2024-01-01T00:00:00Z\t{}\t{}\t{}\t{}\n",
            event, id, hash, detail
        ));
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
        let led = ledger::read(&dir.join("adrdom/state/ledger.tsv"));

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
        let led_path = dir.join("adrdom/state/ledger.tsv");
        append_row(&dir, "asked", "id-a", "h", "A?");
        append_row(&dir, "answered", "id-a", "h", "A");
        append_row(&dir, "asked", "id-b", "h", "B?");
        append_row(&dir, "answered", "id-b", "h", "A");
        append_row(&dir, "superseded", "id-b", "-", "id-c");
        append_row(&dir, "asked", "id-d", "h", "D?");
        append_row(&dir, "withdrawn", "id-d", "-", "D?");
        let led = ledger::read(&led_path);

        let mk = |id: &str| Section {
            start: 0,
            end: 1,
            heading_text: "x".to_string(),
            id: Some(id.to_string()),
            id_dup: false,
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
    fn skeleton_heading_has_no_id_maps_to_skeleton() {
        let dir = temp_repo();
        let content = "# Topic\n\n### 1.1 Not written yet?\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join("adrdom/state/ledger.tsv"));
        assert_eq!(qstate(&led, &rf.sections[0]), QState::Skeleton);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn open_group_and_waits_on_for_unanswered_group_one() {
        let dir = temp_repo();
        let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP);
        append_row(&dir, "asked", "id-1", "hash-1", "1.1 First question?");

        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join("adrdom/state/ledger.tsv"));

        let (open_group, waits_on) = open_group_and_waits_on(&rf, &led);
        // Tier 1's question is Open, so it is the lowest open tier.
        assert_eq!(open_group.as_deref(), Some("1"));
        // Tier 2's question is Unasked (waiting), and tier 1's question is
        // Open (unanswered) and in an earlier tier: that is what `ask`
        // would name.
        assert_eq!(waits_on.as_deref(), Some("1.1 First question?"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn waits_on_is_none_once_group_one_is_answered() {
        let dir = temp_repo();
        let record = write_record(&dir, "adrdom/decisions/example.md", TWO_GROUP);
        append_row(&dir, "asked", "id-1", "hash-1", "1.1 First question?");
        append_row(&dir, "answered", "id-1", "hash-1", "A");

        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join("adrdom/state/ledger.tsv"));

        let (open_group, waits_on) = open_group_and_waits_on(&rf, &led);
        // Tier one is fully answered; tier two (Unasked) is now the lowest
        // tier with an open/unasked question.
        assert_eq!(open_group.as_deref(), Some("2"));
        // Nothing earlier than tier two is unanswered any more.
        assert_eq!(waits_on, None);

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
        let led = ledger::read(&dir.join("adrdom/state/ledger.tsv"));
        let test_idx = HashMap::new();

        // Nothing approved yet, nothing answered: not done.
        let fv = build_file_view(&dir, &rf, &led, &test_idx, &[]);
        assert!(!fv.done);

        // Approve plan/context (hashes are computed from the unanswered
        // fixture, since an answer line never moves either hash) and
        // rewrite the file with every section answered: now it is done.
        let plan_h = record::plan_hash(&rf);
        let ctx_h = record::context_hash(&rf);
        append_row(
            &dir,
            "approved",
            "-",
            &plan_h,
            "adrdom/decisions/example.md",
        );
        append_row(&dir, "context", "-", &ctx_h, "adrdom/decisions/example.md");

        std::fs::write(&record, TWO_GROUP_ANSWERED).unwrap();
        let rf2 = record::load(&record).unwrap();
        let led2 = ledger::read(&dir.join("adrdom/state/ledger.tsv"));
        assert!(
            rf2.sections.iter().all(|s| s.answer_line.is_some()),
            "fixture should have every section answered"
        );
        let fv2 = build_file_view(&dir, &rf2, &led2, &test_idx, &[]);
        assert!(fv2.plan_approved);
        assert!(fv2.context_approved);
        assert!(fv2.done);

        std::fs::remove_dir_all(&dir).ok();
    }
}
