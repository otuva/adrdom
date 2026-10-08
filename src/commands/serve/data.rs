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
    /// The file's own judgement before the report's extras: what `check`
    /// files for the file.
    pub judged: Vec<crate::rules::Finding>,
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

/// Whether `m` names the question `sec`: by the identifier where the move
/// carries one (the owner's answer, the build), else by the heading.
fn names(m: &Move, sec: &Section) -> bool {
    match &m.kind {
        MoveKind::Answer { id, .. } | MoveKind::Build { id, .. } => sec.id.as_deref() == Some(id.as_str()),
        _ => named(m).contains(&sec.heading_text.as_str()),
    }
}

/// Whether `m` is one that names a question at all.
fn names_a_question(m: &Move) -> bool {
    matches!(m.kind, MoveKind::Answer { .. } | MoveKind::Build { .. }) || !named(m).is_empty()
}

/// The headings of the questions a move names, for the moves that name some
/// by heading.
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
            let (report, judged) = if once {
                show::listed_report_judged(r, rf, &mut printed)
            } else {
                show::topic_file_report_judged(r, rf)
            };
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
                        .filter(|m| names(m, sec))
                        .cloned()
                        .collect(),
                })
                .collect();
            File { rf, report, judged, qs }
        })
        .collect()
}

/// The moves of a report that no question of the page's lists carries:
/// shown with the file's own.
pub(super) fn unplaced<'a>(f: &'a File, placed: &dyn Fn(&Q) -> bool) -> Vec<&'a Move> {
    f.report
        .moves
        .iter()
        .filter(|m| !names_a_question(m) || !f.qs.iter().any(|q| names(m, q.sec) && placed(q)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Section;

    fn section(id: &str, heading: &str) -> Section {
        Section {
            start: 0,
            end: 0,
            heading_text: heading.to_string(),
            id: Some(id.to_string()),
            supersedes: Vec::new(),
            unrelated: Vec::new(),
            depends: Vec::new(),
            options: Vec::new(),
            answer_line: None,
            answer_value: None,
            parse_errors: Vec::new(),
            shape_errors: Vec::new(),
            is_skeleton: false,
            terms_line: None,
        }
    }

    // A move that carries an identifier names the question with that
    // identifier, whatever the heading; one that carries none names it by
    // heading.
    #[test]
    fn a_move_with_an_identifier_names_its_question_by_it() {
        let q = section("id-1", "Does it?");
        let other = section("id-2", "Does it?");
        let answer = |id: &str| Move::new(MoveKind::Answer { id: id.into(), version: "v".into(), heading: "Does it?".into() });
        assert!(names(&answer("id-1"), &q));
        assert!(!names(&answer("id-1"), &other), "the same heading is not the same question");
        let build = |id: &str, heading: &str| Move::new(MoveKind::Build { heading: heading.into(), id: id.into() });
        assert!(names(&build("id-2", "another heading"), &other));
        assert!(!names(&build("id-2", "Does it?"), &q));
        let waits = Move::new(MoveKind::AnswerWaits { heading: "Does it?".into(), on: Vec::new() });
        assert!(names(&waits, &q) && names(&waits, &other), "by heading where the move has no identifier");
        assert!(names_a_question(&answer("x")) && names_a_question(&waits));
        assert!(!names_a_question(&Move::new(MoveKind::SupersedeMarkedPairs)));
    }
}
