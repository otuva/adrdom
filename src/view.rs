//! Everything the page shows, computed from the repo (7.1 A). No HTML here.
use crate::commands::check;
use crate::ledger::{self, Ledger};
use crate::record::{self, RecordFile, Section};
use crate::repo;
use crate::state;
use crate::terms;
use std::collections::HashMap;
use std::path::Path;

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

pub struct QuestionView {
    pub id: Option<String>,
    pub heading: String,
    pub group: String,
    pub state: QState,
    pub problem: Option<String>,
    pub standard: Option<String>,
    pub options: Vec<OptionView>,
    pub answer: Option<char>,
    pub supersedes: Vec<String>,
    pub ties: usize,
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

pub struct TermsView {
    pub path: String,
    pub approved: bool,
    pub count: usize,
    pub errors: Vec<String>,
}

pub struct CheckReport {
    pub failures: Vec<String>,
    pub notes: Vec<String>,
}

pub struct Snapshot {
    pub files: Vec<FileView>,
    pub terms: Option<TermsView>,
    pub check: CheckReport,
    pub open_count: usize,
}

/// The state a question/principle heading is in, from the ledger's latest
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

/// `open_group`: the heading of the lowest-tier group that still has an
/// Open or Unasked question in it. `waits_on`: the heading of the first
/// Open/Unasked question in a lower-tier group than the lowest group that
/// has an Unasked/Skeleton question waiting to be asked — i.e. the heading
/// `ask` would name in its tier refusal — or `None` when nothing is
/// waiting on an earlier group.
fn open_group_and_waits_on(rf: &RecordFile, led: &Ledger) -> (Option<String>, Option<String>) {
    let mut group_states: Vec<Vec<(String, QState)>> = vec![Vec::new(); rf.groups.len()];
    for sec in &rf.sections {
        if let Some(gi) = sec.group {
            group_states[gi].push((sec.heading_text.clone(), qstate(led, sec)));
        }
    }

    let open_group_idx = group_states
        .iter()
        .position(|qs| qs.iter().any(|(_, s)| matches!(s, QState::Open | QState::Unasked)));
    let open_group = open_group_idx.map(|gi| rf.groups[gi].heading_text.clone());

    let waiting_group_idx = group_states.iter().position(|qs| {
        qs.iter()
            .any(|(_, s)| matches!(s, QState::Unasked | QState::Skeleton))
    });

    let waits_on = waiting_group_idx.and_then(|g_idx| {
        for states in &group_states[..g_idx] {
            for (heading, s) in states {
                if matches!(s, QState::Open | QState::Unasked) {
                    return Some(heading.clone());
                }
            }
        }
        None
    });

    (open_group, waits_on)
}

fn build_question_view(
    rf: &RecordFile,
    sec: &Section,
    led: &Ledger,
    tie_idx: &HashMap<String, Vec<String>>,
) -> QuestionView {
    let state = qstate(led, sec);
    let group = sec
        .group
        .and_then(|gi| rf.groups.get(gi))
        .map(|g| g.heading_text.clone())
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
    let ties = sec
        .id
        .as_ref()
        .and_then(|id| tie_idx.get(id))
        .map(|v| v.len())
        .unwrap_or(0);

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
        ties,
    }
}

fn build_file_view(
    root: &Path,
    rf: &RecordFile,
    led: &Ledger,
    tie_idx: &HashMap<String, Vec<String>>,
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
        .map(|sec| build_question_view(rf, sec, led, tie_idx))
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

pub fn snapshot(root: &Path) -> Snapshot {
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);
    let tie_idx = check::tie_index(root);

    let mut files: Vec<FileView> = Vec::new();
    for f in repo::list_decision_files(root) {
        if let Ok(rf) = record::load(&f) {
            files.push(build_file_view(root, &rf, &led, &tie_idx));
        }
    }

    let terms = state::load_terms(root).map(|tf| {
        let mut errors = tf.errors.clone();
        errors.extend(terms::check_owners(&tf, root));
        TermsView {
            path: "adrdom/terms.md".to_string(),
            approved: state::terms_approved(&led, &tf),
            count: tf.terms.len(),
            errors,
        }
    });

    let report = check::collect(root);
    let check_report = CheckReport {
        failures: report.failures,
        notes: report.notes,
    };

    let open_count = files
        .iter()
        .flat_map(|f| f.questions.iter())
        .filter(|q| q.state == QState::Open)
        .count();

    Snapshot {
        files,
        terms,
        check: check_report,
        open_count,
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
## Group One\n\n\
### 1.1 First question?\n\n\
id: id-1\n\n\
Problem: p1.\n\n\
Standard: s1.\n\n\
- A) Answer one.\n  Why: reason one.\n  Costs: cost one.\n\
- B) Answer two.\n  Why: reason two.\n\n\
## Group Two\n\n\
### 2.1 Second question?\n\n\
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
            kind: record::Kind::Question,
            start: 0,
            end: 1,
            heading_text: "x".to_string(),
            id: Some(id.to_string()),
            id_dup: false,
            supersedes: vec![],
            options: vec![],
            answer_line: None,
            answer_value: None,
            parse_errors: vec![],
            shape_errors: vec![],
            is_skeleton: false,
            group: None,
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
        let content = "# Topic\n\n## Group\n\n### 1.1 Not written yet?\n";
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
        // Group One's question is Open, so it is the lowest open group.
        assert_eq!(open_group.as_deref(), Some("Group One"));
        // Group Two's question is Unasked (waiting), and group one's
        // question is Open (unanswered) and in a lower group: that is what
        // `ask` would name.
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
        // Group one is fully answered; group two (Unasked) is now the
        // lowest group with an open/unasked question.
        assert_eq!(open_group.as_deref(), Some("Group Two"));
        // Nothing earlier than group two is unanswered any more.
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
## Group One\n\n\
### 1.1 First question?\n\n\
id: id-1\n\n\
Problem: p1.\n\n\
Standard: s1.\n\n\
- A) Answer one.\n  Why: reason one.\n  Costs: cost one.\n\
- B) Answer two.\n  Why: reason two.\n\n\
**Answer:** A\n\n\
## Group Two\n\n\
### 2.1 Second question?\n\n\
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
        let tie_idx = HashMap::new();

        // Nothing approved yet, nothing answered: not done.
        let fv = build_file_view(&dir, &rf, &led, &tie_idx);
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
        let fv2 = build_file_view(&dir, &rf2, &led2, &tie_idx);
        assert!(fv2.plan_approved);
        assert!(fv2.context_approved);
        assert!(fv2.done);

        std::fs::remove_dir_all(&dir).ok();
    }
}
