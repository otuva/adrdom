//! The findings and moves of each file, computed once for the terminal and the page.
use super::Records;
use crate::blame;
use crate::moves::{self, Report};
use crate::record::RecordFile;
use crate::rules::Command;
use crate::rules::{self, Finding};
use crate::status::{self, FileStatus};
use crate::terms::TermsFile;

// --- findings and moves -----------------------------------------------------

pub(crate) fn topic_file_report(r: &Records, rf: &RecordFile) -> Report {
    topic_file_report_judged(r, rf).0
}

pub(crate) fn topic_file_report_judged(r: &Records, rf: &RecordFile) -> (Report, Vec<Finding>) {
    moves::of_topic_file_judged(
        &r.root,
        rf,
        &r.topic_files,
        &r.terms_list,
        &r.led,
        &r.tests,
        r.index(),
        r.satellite.is_some(),
    )
}

pub(super) fn removed_report(r: &Records, rel: &str) -> Report {
    moves::of_removed_topic_file(
        rel,
        rules::judge_removed_topic_file(&r.root, rel, &r.led, &r.present_ids()),
    )
}

/// The status of the terms file and its findings and moves
/// (`moves::of_terms_file`), judged as `approve` judges it.
pub(crate) fn terms_report(r: &Records, tf: &TermsFile) -> (FileStatus, Report) {
    let st = status::terms_file_status(&r.led, tf);
    let Some(minted) = r.minted_terms() else {
        return (st, Report::default());
    };
    if st == FileStatus::Approved {
        return (st, Report::default());
    }
    let findings = rules::terms_file_findings(&r.root, &r.topic_files, minted, &r.led);
    let waiting: Vec<String> = crate::state::waiting_terms(&r.led, tf)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    (st, moves::of_terms_file(findings, st, &waiting))
}

/// The findings and moves of a blame file (`moves::of_blame_file`).
pub(super) fn blame_report(r: &Records, e: &blame::Entry) -> Report {
    moves::of_blame_file(&r.root, e, &r.blame, Command::Show)
}
