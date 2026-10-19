//! The listing with no file named: every current decision and every move.
use super::file::file_status;
use super::findings::{blame_report, removed_report, terms_report, topic_file_report_judged};
use super::topic::chosen_line;
use super::Records;
use crate::ledger::Ledger;
use crate::moves::{self, Report};
use crate::msg::{self};
use crate::out::{self, Line};
use crate::record::RecordFile;
use crate::repo;
use crate::rules::Command;
use crate::rules::{self, Finding};
use crate::status::{self, FileStatus, QuestionStatus};

// --- the listing with no file named ------------------------------------------

/// Every current decision as its heading and the option picked, under its
/// topic file, and every move: of each topic file, each removed topic file,
/// the terms file and each blame file.
pub(crate) fn summary(r: &Records, ids: bool) -> Vec<Line> {
    if let Some(place) = &r.satellite {
        return satellite_summary(r, place, ids);
    }
    let mut lines = Vec::new();
    // The reports of every file, merged and printed once after all decisions.
    let mut merged = Report::default();
    let mut terms_part = None;
    // The terms file first: while it waits for its approval, approve, ask
    // and answer stop on it in every topic file (per "Does the tool stop
    // while the terms file waits for approval?"), so its move comes before
    // theirs, after the decisions; the topic files do not name it again.
    if let Some(tf) = &r.terms_file {
        let (st, report) = terms_report(r, tf);
        if !out::report(&report).is_empty() {
            terms_part = Some((st, report));
        }
    }
    // A finding on the records as a whole is in every topic file's report;
    // the listing prints it once.
    let mut printed: Vec<String> = Vec::new();
    for rf in &r.topic_files {
        lines.push(topic_heading(rf));
        lines.extend(decision_lines(rf, &r.led, ids));
        absorb(&mut merged, listed_report(r, rf, &mut printed));
    }
    let mut blame_part = Vec::new();
    lines.extend(rest_into(r, Some((&mut merged, &mut blame_part)), ids));
    // The refusals of files that cannot be read carry moves: they stand in
    // the end block, a topic file's first and the terms file's after it (a
    // stop on everything), before the terms file's own report.
    lines.extend(unreadable_topic_lines(r));
    lines.extend(r.terms_unreadable_finding());
    // The terms file's report comes before the merged one: its move is the
    // first the reader takes.
    if let Some((st, report)) = terms_part {
        lines.push(file_status(repo::TERMS_PATH, st));
        lines.extend(out::report(&report));
    }
    lines.extend(out::report(&merged));
    // A satellite the list names and the disk lacks: a note, as check prints it.
    lines.extend(
        r.missing
            .iter()
            .map(|row| Line::Finding(satellite_missing_finding(row))),
    );
    // A blame file's status line stands with its report at the end, as the
    // terms file's does.
    lines.extend(blame_part);
    lines.extend(unreadable_blame_lines(r));
    lines
}

/// The note for a satellite with no checkout at its path.
pub(crate) fn satellite_missing_finding(row: &crate::satellite::Row) -> Finding {
    Finding::said(
        crate::rules::Class::Untested,
        msg::satellite_missing(&row.path, &row.url),
    )
}

/// The listing in a satellite: every current decision of the primary as the
/// primary prints it, then the satellite's own report at the working tree
/// (per "What does check in a satellite read the record from?"): its
/// failures and notes, no untested decision and no move.
fn satellite_summary(r: &Records, place: &crate::satellite::Place, ids: bool) -> Vec<Line> {
    let mut lines = Vec::new();
    for rf in &r.topic_files {
        lines.push(topic_heading(rf));
        lines.extend(decision_lines(rf, &r.led, ids));
    }
    for rel in r.removed_topic_files() {
        lines.push(file_status(&rel, FileStatus::Removed));
        if let Some(old) = rules::latest_approved_version(&r.root, &rel, &r.led) {
            lines.extend(decision_lines(&old, &r.led, ids));
        }
    }
    let report = crate::commands::check::satellite_report(place, false, None, Some(&r.tests));
    lines.extend(
        report
            .stops
            .into_iter()
            .map(|s| out::refusal(Command::Check, &s)),
    );
    lines.extend(
        report
            .failures
            .into_iter()
            .chain(report.notes)
            .map(Line::Finding),
    );
    lines
}

/// The findings of the blame files that cannot be read.
pub(super) fn unreadable_blame_lines(r: &Records) -> Vec<Line> {
    r.unreadable_blame_files
        .iter()
        .map(|(rel, _)| out::refusal(Command::Show, &msg::blame_file_unreadable(rel)))
        .collect()
}

/// Appends the stops, findings and moves of `more` to `into`, finding texts
/// already in `into` left out.
fn absorb(into: &mut Report, more: Report) {
    into.stops.extend(more.stops);
    for f in more.findings {
        let text = f.text();
        if !into.findings.iter().any(|g| g.text() == text) {
            into.findings.push(f);
        }
    }
    into.moves.extend(more.moves);
}

/// The findings of the topic files that cannot be read.
pub(crate) fn unreadable_topic_lines(r: &Records) -> Vec<Line> {
    r.unreadable_topic_files
        .iter()
        .map(|(rel, e)| out::refusal(Command::Show, &msg::topic_file_unreadable(rel, e)))
        .collect()
}

/// The line that opens a topic file in the listing.
pub(crate) fn topic_heading(rf: &RecordFile) -> Line {
    Line::Topic {
        text: msg::show_topic(rf.topic.as_deref().unwrap_or(""), &rf.rel),
    }
}

/// Every current decision of `rf` as its heading and the option picked;
/// with `ids`, its `id:` line between them.
pub(crate) fn decision_lines(rf: &RecordFile, led: &Ledger, ids: bool) -> Vec<Line> {
    let mut lines = Vec::new();
    for sec in rf
        .sections
        .iter()
        .filter(|s| status::question_status(led, s) == QuestionStatus::Answered)
    {
        lines.push(Line::Record {
            depth: 1,
            text: sec.heading_text.clone(),
        });
        if ids {
            lines.extend(id_line(sec, 2));
        }
        lines.extend(chosen_line(led, sec, QuestionStatus::Answered, 2));
    }
    lines
}

/// The findings and moves of `rf` as the listing prints them: a finding on
/// the terms file (while that file's own lines stand) and a finding an
/// earlier file printed (`printed`) are left out.
pub(crate) fn listed_report(r: &Records, rf: &RecordFile, printed: &mut Vec<String>) -> Report {
    listed_report_judged(r, rf, printed).0
}

/// `listed_report` and the file's own judgement, as `moves::of_topic_file_judged`
/// returns it.
pub(crate) fn listed_report_judged(
    r: &Records,
    rf: &RecordFile,
    printed: &mut Vec<String>,
) -> (Report, Vec<Finding>) {
    let (mut report, judged) = topic_file_report_judged(r, rf);
    report.findings.retain(|f| {
        let text = f.text();
        if f.subject == rules::Subject::TermsFile && r.terms_file.is_some()
            || printed.contains(&text)
        {
            return false;
        }
        printed.push(text);
        true
    });
    // The terms file's own report names the commit that waits on it.
    if r.terms_file.is_some() {
        report
            .moves
            .retain(|m| m.kind != moves::MoveKind::CommitWaits);
    }
    (report, judged)
}

/// What the listing prints after the topic files: each removed topic file,
/// each blame file, and the blame files that cannot be read.
pub(crate) fn rest(r: &Records) -> Vec<Line> {
    rest_into(r, None, false)
}

/// The `id:` line of `sec`, as `--ids` prints it under the heading.
pub(crate) fn id_line(sec: &crate::record::Section, depth: usize) -> Option<Line> {
    sec.id.as_ref().map(|id| Line::Record {
        depth,
        text: format!("id: {}", id),
    })
}

/// `rest`; with `merged`, the reports of the removed files go into its report
/// and each blame file's status line and report into its lines, not among
/// the lines returned.
fn rest_into(
    r: &Records,
    mut merged: Option<(&mut Report, &mut Vec<Line>)>,
    ids: bool,
) -> Vec<Line> {
    let mut lines = Vec::new();
    for rel in r.removed_topic_files() {
        lines.push(file_status(&rel, FileStatus::Removed));
        // Its current decisions stand until a plan change supersedes them:
        // listed from its approved version, as every file's are.
        if let Some(old) = rules::latest_approved_version(&r.root, &rel, &r.led) {
            lines.extend(decision_lines(&old, &r.led, ids));
        }
        match merged.as_mut() {
            Some((m, _)) => absorb(m, removed_report(r, &rel)),
            None => lines.extend(out::report(&removed_report(r, &rel))),
        }
    }
    for e in r.blame.entries() {
        let report = blame_report(r, e);
        if !out::report(&report).is_empty() {
            let status = file_status(e.rel(), e.status);
            match merged.as_mut() {
                Some((_, tail)) => {
                    tail.push(status);
                    tail.extend(out::report(&report));
                }
                None => {
                    lines.push(status);
                    lines.extend(out::report(&report));
                }
            }
        }
    }
    // In the listing (`merged` given) they stand in its end block.
    if merged.is_none() {
        lines.extend(unreadable_blame_lines(r));
    }
    lines
}
