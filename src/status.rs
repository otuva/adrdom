//! The status of every thing in the records, one closed list per thing, each
//! status named once (per "Does a command name a status of its own?",
//! f53ac3b9 A): a command prints a status by `name()` and never words of its
//! own. Each list has the one function that computes it from the ledger.
use crate::ledger::Ledger;
use crate::record::{RecordFile, Section};
use crate::terms::{self, Term, TermsFile};

/// The status of a question, from the ledger's latest state row for its id
/// and whether it has a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionStatus {
    /// A heading of the plan, its body not written.
    NoBody,
    /// The body is written, `ask` has not run on it.
    NotAsked,
    /// It waits for the owner's answer.
    Asked,
    Answered,
    /// The owner's answer that the question cannot be settled as asked
    /// (per "Does the owner answer a question with a refusal?").
    Refused,
    Superseded,
    Withdrawn,
    /// It left its file when an answer's set lacked the letter (per "Does
    /// answer remove a question whose answer set lacks the answer?"); its
    /// identifier never returns.
    Dropped,
}

impl QuestionStatus {
    /// Every status, in the order a question passes through them.
    pub const ALL: [QuestionStatus; 8] = [
        QuestionStatus::NoBody,
        QuestionStatus::NotAsked,
        QuestionStatus::Asked,
        QuestionStatus::Answered,
        QuestionStatus::Refused,
        QuestionStatus::Superseded,
        QuestionStatus::Withdrawn,
        QuestionStatus::Dropped,
    ];

    pub fn name(self) -> &'static str {
        match self {
            QuestionStatus::NoBody => "no body",
            QuestionStatus::NotAsked => "not asked",
            QuestionStatus::Asked => "asked",
            QuestionStatus::Answered => "answered",
            QuestionStatus::Refused => "refused",
            QuestionStatus::Superseded => "superseded",
            QuestionStatus::Withdrawn => "withdrawn",
            QuestionStatus::Dropped => "dropped",
        }
    }

    /// The name with the picked letter of an answered question
    /// (`answered A`); every other status is its bare name.
    pub fn name_with(self, letter: Option<char>) -> String {
        match (self, letter) {
            (QuestionStatus::Answered, Some(c)) => format!("{} {}", self.name(), c),
            _ => self.name().to_string(),
        }
    }

    /// Not yet asked: no body, or a body `ask` has not run on.
    pub fn is_unasked(self) -> bool {
        matches!(self, QuestionStatus::NoBody | QuestionStatus::NotAsked)
    }

    /// History: a status no later act moves the question out of.
    pub fn is_dead(self) -> bool {
        matches!(
            self,
            QuestionStatus::Refused | QuestionStatus::Superseded | QuestionStatus::Withdrawn | QuestionStatus::Dropped
        )
    }
}

/// The status of `sec`. A heading with no state row, approved or not, is
/// `NoBody` while it has no body and `NotAsked` once it has one: approve
/// gives every heading an id, so the id says nothing about the body.
pub fn question_status(led: &Ledger, sec: &Section) -> QuestionStatus {
    let never_asked = if sec.is_skeleton {
        QuestionStatus::NoBody
    } else {
        QuestionStatus::NotAsked
    };
    let Some(id) = &sec.id else {
        return never_asked;
    };
    match led.latest_state(id).map(|r| r.event.as_str()) {
        Some("asked") => QuestionStatus::Asked,
        Some("answered") => QuestionStatus::Answered,
        Some("refused") => QuestionStatus::Refused,
        Some("superseded") => QuestionStatus::Superseded,
        Some("withdrawn") => QuestionStatus::Withdrawn,
        Some("dropped") => QuestionStatus::Dropped,
        _ => never_asked,
    }
}

/// Whether `ask` takes `sec` in this run: a question with a body that has no
/// state, or is asked and edited since (its text no longer the hash of its
/// latest `asked` row). The one "not yet asked" test every rule on text that
/// can still change reads (per "Does a command refuse what check passes?",
/// fe14991d A): `ask` refuses on it what `approve` and `check` find on it.
pub fn open_to_ask(led: &Ledger, rf: &RecordFile, sec: &Section) -> bool {
    open_to_ask_text(led, rf, sec) && !sec.is_skeleton
}

/// `open_to_ask` with or without a body: text a rule on unasked text reads
/// (a heading with no body is read for its heading and depends line).
pub fn open_to_ask_text(led: &Ledger, rf: &RecordFile, sec: &Section) -> bool {
    let Some(id) = &sec.id else { return true };
    match led.latest_state(id) {
        None => true,
        Some(row) if row.event == "asked" => crate::record::section_hash(&rf.lines, sec) != row.hash,
        Some(_) => false,
    }
}

/// The status of a file the owner approves: a topic file, the terms file or
/// a blame file. `Removed` is a topic file gone from disk whose removal the
/// owner has not approved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// Never approved.
    New,
    /// Differs from its latest approval.
    Changed,
    /// Equal to its latest approval.
    Approved,
    Removed,
}

impl FileStatus {
    pub const ALL: [FileStatus; 4] = [FileStatus::New, FileStatus::Changed, FileStatus::Approved, FileStatus::Removed];

    pub fn name(self) -> &'static str {
        match self {
            FileStatus::New => "new",
            FileStatus::Changed => "changed",
            FileStatus::Approved => "approved",
            FileStatus::Removed => "removed",
        }
    }

    /// New or changed: approve has something to take.
    pub fn awaits_approval(self) -> bool {
        !matches!(self, FileStatus::Approved)
    }
}

/// The status of a topic file, one name for its plan and prose together:
/// `Approved` only while both equal their latest approval. `rf` must be the
/// file as written, before `depends::normalize_cross_file_deps`.
pub type TopicFileStatus = FileStatus;
/// The status of the terms file.
pub type TermsFileStatus = FileStatus;
/// The status of a blame file.
pub type BlameFileStatus = FileStatus;

pub fn topic_file_status(led: &Ledger, rf: &RecordFile) -> TopicFileStatus {
    if crate::state::plan_approved(led, rf, &rf.rel) && crate::state::context_approved(led, rf, &rf.rel) {
        return FileStatus::Approved;
    }
    if led.latest_for_path("approved", &rf.rel).is_some() || led.latest_for_path("context", &rf.rel).is_some() {
        FileStatus::Changed
    } else {
        FileStatus::New
    }
}

/// The status of a topic file at `rel` that is gone from disk: `Removed`
/// while the ledger still holds an approval of it, else `None` (a file never
/// approved has no status once it is gone).
pub fn removed_topic_file_status(led: &Ledger, rel: &str) -> Option<TopicFileStatus> {
    crate::rules::removal_awaiting(led, &|_| false).iter().any(|r| r == rel).then_some(FileStatus::Removed)
}

pub fn terms_file_status(led: &Ledger, tf: &TermsFile) -> TermsFileStatus {
    if crate::state::terms_approved(led, tf) {
        FileStatus::Approved
    } else if led.latest_for_path("terms", crate::repo::TERMS_PATH).is_some() {
        FileStatus::Changed
    } else {
        FileStatus::New
    }
}

pub fn blame_file_status(led: &Ledger, rel: &str, content: &str) -> BlameFileStatus {
    if crate::state::blame_approved(led, rel, content) {
        FileStatus::Approved
    } else if led.latest_for_path("blame", rel).is_some() {
        FileStatus::Changed
    } else {
        FileStatus::New
    }
}

/// The status of one entry of the terms file against its latest `term` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermStatus {
    New,
    Changed,
    Approved,
    Retired,
    /// Approved once and no longer in the file.
    Removed,
}

impl TermStatus {
    pub const ALL: [TermStatus; 5] =
        [TermStatus::New, TermStatus::Changed, TermStatus::Approved, TermStatus::Retired, TermStatus::Removed];

    pub fn name(self) -> &'static str {
        match self {
            TermStatus::New => "new",
            TermStatus::Changed => "changed",
            TermStatus::Approved => "approved",
            TermStatus::Retired => "retired",
            TermStatus::Removed => "removed",
        }
    }
}

/// The status of `t`: `New` with no `term` row for its id, `Changed` when
/// its hash differs from that row, else `Retired` or `Approved`.
pub fn term_status(led: &Ledger, t: &Term) -> TermStatus {
    match t.id.as_deref().and_then(|id| led.latest_for_id("term", id)) {
        None => TermStatus::New,
        Some(r) if r.hash != terms::term_hash(t) => TermStatus::Changed,
        Some(_) if t.retired => TermStatus::Retired,
        Some(_) => TermStatus::Approved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_closed_list_names_each_status_once() {
        let q: HashSet<&str> = QuestionStatus::ALL.iter().map(|s| s.name()).collect();
        assert_eq!(q.len(), QuestionStatus::ALL.len());
        let f: HashSet<&str> = FileStatus::ALL.iter().map(|s| s.name()).collect();
        assert_eq!(f.len(), FileStatus::ALL.len());
        let t: HashSet<&str> = TermStatus::ALL.iter().map(|s| s.name()).collect();
        assert_eq!(t.len(), TermStatus::ALL.len());
    }

    #[test]
    fn answered_carries_its_letter_and_nothing_else_does() {
        assert_eq!(QuestionStatus::Answered.name_with(Some('A')), "answered A");
        assert_eq!(QuestionStatus::Asked.name_with(Some('A')), "asked");
    }

    fn led(rows: &[(&str, &str)]) -> Ledger {
        Ledger {
            rows: rows
                .iter()
                .map(|(event, id)| crate::ledger::Row {
                    time: String::new(),
                    event: event.to_string(),
                    id: id.to_string(),
                    hash: "h".to_string(),
                    detail: String::new(),
                    version: String::new(),
                })
                .collect(),
            malformed: Vec::new(),
        }
    }

    #[test]
    fn a_dropped_question_is_dropped_not_unasked() {
        let rf = crate::record::build(
            std::path::PathBuf::from("x.md"),
            "x.md".to_string(),
            crate::record::split_lines("# T\n\n### Does it?\ndepends: w\nid: id-1\n\nProblem: p.\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n"),
        );
        let sec = &rf.sections[0];
        assert_eq!(question_status(&led(&[("asked", "id-1"), ("dropped", "id-1")]), sec), QuestionStatus::Dropped);
        assert_eq!(question_status(&led(&[]), sec), QuestionStatus::NotAsked);
        assert_eq!(question_status(&led(&[("asked", "id-1")]), sec), QuestionStatus::Asked);
    }
}
