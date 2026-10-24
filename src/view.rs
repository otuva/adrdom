//! Views over the records shared by the commands: check's report, the
//! words a reference names.
use crate::ledger::Ledger;
use crate::record::{self, RecordFile, Section};
use crate::state;
use std::collections::HashMap;

pub use crate::status::QuestionStatus;

/// What `check` reports: the findings that fail it, and the notes, the
/// findings that fail nothing (a fault on a file awaiting approval, an
/// untested decision).
pub struct CheckReport {
    pub failures: Vec<crate::rules::Finding>,
    pub notes: Vec<crate::rules::Finding>,
    /// What stopped `check` reading a file or a commit, apart from the
    /// findings: each prints as `check: <what>; <move>`.
    pub stops: Vec<String>,
    /// The moves no finding carries: a commit that waits while the terms
    /// file waits for the owner's approval.
    pub moves: Vec<crate::moves::Move>,
}

/// Every heading id of the topic files `records`, paired with its heading
/// text (per "What does a reference in a topic file carry?"): what turns an identifier back into
/// the heading it names.
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

/// `heading_texts`, and for a question no topic file holds any more (struck
/// as a superseded section, or dropped) the heading its latest `asked` row
/// carries (per "What strikes the section of a superseded question from its
/// topic file?"). adr-code: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
pub fn heading_names(records: &[RecordFile], led: &Ledger) -> HashMap<String, String> {
    let mut out: HashMap<String, String> = led
        .rows
        .iter()
        .filter(|r| r.event == "asked" && !r.detail.is_empty())
        .map(|r| (r.id.clone(), r.detail.clone()))
        .collect();
    out.extend(heading_texts(records));
    out
}

/// `state::asked_text_stands` for a section of a loaded topic file: the
/// question `answer` takes, and the one `show` prints a version for.
pub(crate) fn asked_text_stands(led: &Ledger, rf: &RecordFile, sec: &Section) -> bool {
    let Some(id) = &sec.id else { return false };
    state::asked_text_stands(led, id, &record::section_hash(&rf.lines, sec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ledger, repo};
    use std::path::{Path, PathBuf};
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
        existing.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            time, event, id, hash, detail
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
        let led = ledger::read(&dir.join(repo::LEDGER_PATH));

        // 1.1 has id-1, asked -> Open.
        let sec1 = rf
            .sections
            .iter()
            .find(|s| s.heading_text == "1.1 First question?")
            .unwrap();
        assert_eq!(
            crate::status::question_status(&led, sec1),
            QuestionStatus::Asked
        );

        // 2.1 has no id (never asked) but has a body -> Unasked.
        let sec2 = rf
            .sections
            .iter()
            .find(|s| s.heading_text == "2.1 Second question?")
            .unwrap();
        assert_eq!(
            crate::status::question_status(&led, sec2),
            QuestionStatus::NotAsked
        );

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
        append_row_at(
            &dir,
            "superseded",
            "id-b",
            "-",
            "id-c",
            "2024-01-01T00:00:04Z",
        );
        append_row_at(&dir, "asked", "id-d", "h", "D?", "2024-01-01T00:00:05Z");
        append_row_at(&dir, "withdrawn", "id-d", "-", "D?", "2024-01-01T00:00:06Z");
        let led = ledger::read(&led_path);

        let mk = |id: &str| Section {
            start: 0,
            end: 1,
            heading_text: "x".to_string(),
            id: Some(id.to_string()),
            supersedes: vec![],
            blame: vec![],
            depends: vec![],
            options: vec![],
            answer_line: None,
            answer_value: None,
            parse_errors: vec![],
            shape_errors: vec![],
            is_skeleton: false,
            terms_line: None,
        };

        assert_eq!(
            crate::status::question_status(&led, &mk("id-a")),
            QuestionStatus::Answered
        );
        assert_eq!(
            crate::status::question_status(&led, &mk("id-b")),
            QuestionStatus::Superseded
        );
        assert_eq!(
            crate::status::question_status(&led, &mk("id-d")),
            QuestionStatus::Withdrawn
        );

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
            blame: vec![],
            depends: vec![],
            options: vec![],
            answer_line: None,
            answer_value: None,
            parse_errors: vec![],
            shape_errors: vec![],
            is_skeleton: false,
            terms_line: None,
        };
        assert_eq!(
            crate::status::question_status(&led, &sec),
            QuestionStatus::Refused
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skeleton_heading_has_no_id_maps_to_skeleton() {
        let dir = temp_repo();
        let content = "# Topic\n\n### 1.1 Not written yet?\n";
        let record = write_record(&dir, "adrdom/decisions/example.md", content);
        let rf = record::load(&record).unwrap();
        let led = ledger::read(&dir.join(repo::LEDGER_PATH));
        assert_eq!(
            crate::status::question_status(&led, &rf.sections[0]),
            QuestionStatus::NoBody
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
