//! What the pages read of the records: each topic file with its findings and
//! moves from the one computation (`show::topic_file_report`), and each of
//! its questions with its status line and the moves that name it. Nothing
//! here computes a status or a move of its own.
use crate::commands::show::{self, Records};
use crate::moves::{Move, MoveKind, Report};
use crate::out::Line;
use crate::record::{RecordFile, Section};
use crate::status::{self, QuestionStatus};

/// One topic file of the page.
pub(super) struct File<'a> {
    pub rf: &'a RecordFile,
    pub report: Report,
    pub qs: Vec<Q<'a>>,
}

/// One question of a topic file: its status, the status line, and the moves
/// of the file's report that name it (the owner's answer, what an asked
/// question waits on, the refusal that holds the plan).
pub(super) struct Q<'a> {
    pub rf: &'a RecordFile,
    pub idx: usize,
    pub sec: &'a Section,
    pub status: QuestionStatus,
    pub line: Line,
    pub moves: Vec<Move>,
}

impl Q<'_> {
    pub fn id(&self) -> &str {
        self.sec.id.as_deref().unwrap_or("")
    }

    pub fn heading(&self) -> &str {
        &self.sec.heading_text
    }

    /// The identifier and the version of the owner's answer move, when the
    /// report names one: the question `answer` takes now.
    pub fn answer(&self) -> Option<(&str, &str)> {
        self.moves.iter().find_map(|m| match &m.kind {
            MoveKind::Answer { id, version, .. } => Some((id.as_str(), version.as_str())),
            _ => None,
        })
    }

    pub fn answerable(&self) -> bool {
        self.answer().is_some()
    }

    /// Asked and not answered: put to the owner, or edited since and asked
    /// again by the agent. What the pages count as open.
    pub fn listed(&self) -> bool {
        matches!(self.status, QuestionStatus::Asked | QuestionStatus::ChangedSinceAsked)
    }
}

/// The headings of the questions a move names, for the moves that name some.
fn named(m: &Move) -> Vec<&str> {
    match &m.kind {
        MoveKind::Answer { heading, .. } | MoveKind::AnswerWaits { heading, .. } | MoveKind::Refused { heading } => {
            vec![heading]
        }
        MoveKind::Ask { headings, .. } => headings.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    }
}

/// Every topic file with its report and questions. With `once`, a finding an
/// earlier file already holds is left out, as the listing of `adr show`
/// prints it.
pub(super) fn files(r: &Records, once: bool) -> Vec<File<'_>> {
    let mut printed: Vec<String> = Vec::new();
    r.topic_files
        .iter()
        .map(|rf| {
            let report = if once { show::listed_report(r, rf, &mut printed) } else { show::topic_file_report(r, rf) };
            let qs = rf
                .sections
                .iter()
                .enumerate()
                .map(|(idx, sec)| Q {
                    rf,
                    idx,
                    sec,
                    status: status::question_status_in(&r.led, rf, sec),
                    line: Line::Status {
                        subject: sec.heading_text.clone(),
                        status: show::question_status_of(&r.led, rf, sec),
                        mark: None,
                    },
                    moves: report
                        .moves
                        .iter()
                        .filter(|m| named(m).contains(&sec.heading_text.as_str()))
                        .cloned()
                        .collect(),
                })
                .collect();
            File { rf, report, qs }
        })
        .collect()
}

/// The moves of a report that no question of the page's lists carries:
/// shown with the file's own.
pub(super) fn unplaced<'a>(f: &'a File, placed: &dyn Fn(&Q) -> bool) -> Vec<&'a Move> {
    f.report
        .moves
        .iter()
        .filter(|m| {
            let names = named(m);
            names.is_empty() || !f.qs.iter().any(|q| names.contains(&q.heading()) && placed(q))
        })
        .collect()
}
