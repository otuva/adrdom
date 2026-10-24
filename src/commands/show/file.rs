//! The file form: one file approve takes, its status line, changes and moves; the terms file, a removed topic file and a blame file.
use super::findings::{blame_report, removed_report, terms_report};
use super::listing::unreadable_blame_lines;
use super::topic::{
    chosen_line, is_answered, question_status_of, reference_and_body_lines, replaced_by_lines,
    term_dependents, topic_file, topic_file_lines,
};
use super::{Records, Views};
use crate::approval::{self, Change, Side};
use crate::blame;
use crate::ledger::Ledger;
use crate::msg::{self};
use crate::out::{self, Line, Status};
use crate::plan;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::rules::Command;
use crate::rules::{self};
use crate::status::{self, FileStatus, QuestionStatus};
use crate::terms::TermsFile;
use std::collections::HashMap;

// --- the file form -------------------------------------------------------------

/// Every file approve takes, each in the file form.
pub(super) fn whole(r: &Records, views: Views) -> Vec<Line> {
    let mut lines = Vec::new();
    for (rel, e) in &r.unreadable_topic_files {
        lines.push(out::refusal(
            Command::Show,
            &msg::topic_file_unreadable(rel, e),
        ));
    }
    for rf in &r.topic_files {
        lines.extend(topic_file(r, rf, views));
    }
    for rel in r.removed_topic_files() {
        lines.extend(removed_topic_file(r, &rel, views));
    }
    lines.extend(r.terms_unreadable_finding());
    if let Some(tf) = &r.terms_file {
        lines.extend(terms_file(r, tf, views));
    }
    for e in r.blame.entries() {
        lines.extend(blame_file(r, e, views));
    }
    lines.extend(unreadable_blame_lines(r));
    lines
}

/// The file form of `rel`, or the refusal when it is not a file approve
/// takes.
pub(crate) fn file_form(r: &Records, rel: &str, views: Views) -> Result<Vec<Line>, String> {
    if rel == repo::TERMS_PATH {
        return match (&r.terms_file, &r.terms_unreadable) {
            (Some(tf), _) => Ok(terms_file(r, tf, views)),
            (None, Some(e)) => Err(msg::file_unreadable(rel, e)),
            (None, None) => Err(msg::read_failed(
                rel,
                "no such file",
                msg::Next::agent(msg::Act::Rewrites(msg::Part::Path)),
            )),
        };
    }
    if let Some(rf) = r.topic_files.iter().find(|rf| rf.rel == rel) {
        return Ok(topic_file(r, rf, views));
    }
    if let Some(e) = r.blame.entries().iter().find(|e| e.rel() == rel) {
        return Ok(blame_file(r, e, views));
    }
    // The finding `show` and `check` print for it, word for word.
    if let Some((_, e)) = r.unreadable_topic_files.iter().find(|(f, _)| f == rel) {
        return Err(msg::topic_file_unreadable(rel, e));
    }
    if r.unreadable_blame_files.iter().any(|(f, _)| f == rel) {
        return Err(msg::blame_file_unreadable(rel));
    }
    if r.removed_topic_files().iter().any(|f| f == rel) {
        return Ok(removed_topic_file(r, rel, views));
    }
    Err(msg::show_not_a_file_approve_takes(rel))
}

/// The file form of a topic file or the terms file without its findings and
/// moves, for a caller that holds the report already (the ui reads each file's
/// report once); any other file as `file_form` prints it.
pub(crate) fn file_form_lines(r: &Records, rel: &str, views: Views) -> Result<Vec<Line>, String> {
    if rel == repo::TERMS_PATH {
        if let Some(tf) = &r.terms_file {
            return Ok(terms_file_lines(
                r,
                tf,
                views,
                status::terms_file_status(&r.led, tf),
            ));
        }
    } else if let Some(rf) = r.topic_files.iter().find(|rf| rf.rel == rel) {
        return Ok(topic_file_lines(r, rf, views));
    }
    file_form(r, rel, views)
}

/// The status line of a topic file, as every command that names its status
/// prints it.
pub(crate) fn topic_file_status_line(led: &Ledger, rf: &RecordFile) -> Line {
    file_status(&topic_subject(rf), status::topic_file_status(led, rf))
}

/// A topic file as the status line names it: its topic and its path.
pub(crate) fn topic_subject(rf: &RecordFile) -> String {
    match rf.topic.as_deref() {
        Some(t) if !t.is_empty() => format!("{} ({})", t, rf.rel),
        _ => rf.rel.clone(),
    }
}

/// The status line of a blame file, as every command that names its status
/// prints it.
pub(crate) fn blame_file_status_line(e: &blame::Entry) -> Line {
    file_status(e.rel(), e.status)
}

pub(super) fn file_status(subject: &str, st: FileStatus) -> Line {
    Line::Status {
        subject: subject.to_string(),
        status: Status::File(st),
        mark: None,
    }
}

/// The status line of a part of a file whose `change` beyond its latest
/// approval is shown under it: marked when the part is new or struck as a
/// whole.
pub(super) fn change_status(subject: &str, status: Status, change: Option<&Change>) -> Line {
    let mark = change.and_then(|c| match c.status {
        FileStatus::New => Some(Side::Added),
        FileStatus::Removed => Some(Side::Removed),
        FileStatus::Changed | FileStatus::Approved => None,
    });
    Line::Status {
        subject: subject.to_string(),
        status,
        mark,
    }
}

/// The lines of a change, marked, the identifiers of its reference lines
/// turned back into words. `own` is the part's own heading line (`### ..`
/// of a question, `## ..` of a term entry): left out, the status line above
/// names it, unless the heading itself changed. Every other line is
/// printed, one that starts like a heading included. A changed part whose
/// approved text no commit holds says so above its lines, all marked added.
pub(super) fn change_lines(
    change: &Change,
    own: Option<&str>,
    names: &HashMap<String, String>,
) -> Vec<Line> {
    let prefix = own.and_then(|o| o.split_once(' ').map(|(p, _)| format!("{} ", p)));
    // The heading changed: a heading line gone and one come, both shown.
    let heading_changed = prefix.as_deref().is_some_and(|p| {
        [Side::Removed, Side::Added].iter().all(|side| {
            change
                .lines
                .iter()
                .any(|l| l.side == *side && l.text.starts_with(p))
        })
    });
    let mut out = Vec::new();
    if !change.baseline {
        out.push(Line::Record {
            depth: 1,
            text: msg::show_no_baseline(),
        });
    }
    out.extend(
        change
            .lines
            .iter()
            .filter(|l| heading_changed || own != Some(l.text.as_str()))
            .map(|l| Line::Record {
                depth: 1,
                text: reference_named(&l.marked(), &l.text, names),
            }),
    );
    out
}

/// `printed`, the printed form of the record line `line`, with the
/// identifiers turned back into words when `line` is a reference line
/// (`depends:`, `supersedes:`, `blame:`): what approve turns
/// names into identifiers on, and nothing else. An identifier in prose stays
/// as the file holds it, the text approve signs.
fn reference_named(printed: &str, line: &str, names: &HashMap<String, String>) -> String {
    if is_reference(line) {
        named(printed, names)
    } else {
        printed.to_string()
    }
}

fn is_reference(line: &str) -> bool {
    ["depends:", "supersedes:", "blame:"]
        .iter()
        .any(|p| line.starts_with(p))
}

/// A line the records hold with every identifier on it turned back into
/// the heading or the name it stands for (per "What does a reference in a topic file carry?": show
/// turns it back into the name). An identifier no question or term holds stays as it is.
pub(super) fn named(line: &str, names: &HashMap<String, String>) -> String {
    const LEN: usize = 36;
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < line.len() {
        if let Some(cand) = line.get(i..i + LEN) {
            let bounded = !line[..i].ends_with(|c: char| c.is_ascii_hexdigit())
                && !line[i + LEN..].starts_with(|c: char| c.is_ascii_hexdigit());
            if bounded && record::is_uuid(cand) {
                if let Some(name) = names
                    .get(cand)
                    .or_else(|| names.get(&cand.to_ascii_lowercase()))
                {
                    out.push_str(name);
                    i += LEN;
                    continue;
                }
            }
        }
        let c = line[i..].chars().next().expect("i is on a char boundary");
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// The reference lines a question carries (`depends:`, `supersedes:`,
/// `blame:`), and its body, each as the file holds it. An `unrelated:` line
/// of an earlier record form (`tells::UNRELATED_PREFIX`) is neither: it is
/// never printed.
pub(super) fn reference_and_body(lines: &[String]) -> (Vec<String>, Vec<String>) {
    let rest = lines
        .iter()
        .skip(1)
        .filter(|l| !l.starts_with(crate::tells::UNRELATED_PREFIX));
    (
        rest.clone().filter(|l| is_reference(l)).cloned().collect(),
        rest.filter(|l| !is_reference(l)).cloned().collect(),
    )
}

/// A topic file gone from disk whose removal the owner has not approved:
/// its status, each question of its approved plan with the question's own
/// status (per "Does a command name a status of its own?", f53ac3b9 A), and
/// its move.
fn removed_topic_file(r: &Records, rel: &str, views: Views) -> Vec<Line> {
    let mut lines = vec![file_status(rel, FileStatus::Removed)];
    if let Some(old) = rules::latest_approved_version(&r.root, rel, &r.led) {
        for sec in &old.sections {
            let st = status::question_status(&r.led, sec);
            if st == QuestionStatus::Superseded && !views.superseded {
                continue;
            }
            lines.push(Line::Status {
                subject: sec.heading_text.clone(),
                status: question_status_of(&r.led, &old, sec),
                mark: None,
            });
            lines.extend(chosen_line(&r.led, sec, st, 1));
            lines.extend(reference_and_body_lines(
                &approval::question_lines(&old, sec),
                st,
                &r.names,
                views.dependents || !is_answered(st),
            ));
            if views.superseded && st == QuestionStatus::Superseded {
                lines.extend(replaced_by_lines(r, sec));
            }
        }
    }
    lines.extend(out::report(&removed_report(r, rel)));
    lines
}

/// The terms file: its status, every entry with its status and its lines
/// (and, under `--dependents`, the current decisions that stand on it), each
/// entry approved and gone from the file, then its findings and moves.
fn terms_file(r: &Records, tf: &TermsFile, views: Views) -> Vec<Line> {
    let (st, report) = terms_report(r, tf);
    let mut lines = terms_file_lines(r, tf, views, st);
    lines.extend(out::report(&report));
    lines
}

fn terms_file_lines(r: &Records, tf: &TermsFile, views: Views, st: FileStatus) -> Vec<Line> {
    let mut lines = vec![file_status(repo::TERMS_PATH, st)];
    let changes = approval::terms_file(&r.root, tf, &r.led);
    if let Some(whole) = &changes.whole {
        lines.extend(change_lines(whole, None, &r.names));
    }
    for (t, change) in tf.terms.iter().zip(&changes.entries) {
        lines.push(change_status(
            &t.name,
            Status::Term(status::term_status(&r.led, tf, t)),
            change.as_ref(),
        ));
        match change {
            Some(change) => lines.extend(change_lines(
                change,
                Some(&format!("## {}", t.name)),
                &r.names,
            )),
            None => lines.extend(approval::entry_lines(tf, t).into_iter().skip(1).map(|l| {
                Line::Record {
                    depth: 1,
                    text: named(&l, &r.names),
                }
            })),
        }
        if views.dependents {
            let dependents = term_dependents(r, t);
            if !dependents.is_empty() {
                lines.push(Line::Record {
                    depth: 1,
                    text: format!("dependents: {}", dependents.join(", ")),
                });
            }
        }
    }
    for (name, change) in &changes.gone {
        lines.push(change_status(
            name,
            Status::Term(status::TermStatus::Removed),
            Some(change),
        ));
        lines.extend(change_lines(
            change,
            Some(&format!("## {}", name)),
            &r.names,
        ));
    }
    lines
}

/// A blame file: its status, its origin and every row as written, marked
/// when beyond its latest approval, under `--questions` the count of its
/// decision rows a question names, then its moves.
fn blame_file(r: &Records, e: &blame::Entry, views: Views) -> Vec<Line> {
    let bf = &e.file;
    let report = blame_report(r, e);
    let mut lines = vec![blame_file_status_line(e)];
    match approval::blame_file(&r.root, &bf.rel, &bf.lines, &r.led) {
        Some(change) => lines.extend(blame_change_lines(&change)),
        None => {
            if let Some(origin) = &bf.origin {
                lines.push(Line::Record {
                    depth: 1,
                    text: format!("origin: {}", origin),
                });
            }
            lines.extend(bf.rows.iter().map(blame_row_line));
        }
    }
    if views.questions {
        let rows = blame::decision_rows(e, &r.blame, &r.topic_files);
        let with = rows.iter().filter(|&&(_, named)| named).count();
        lines.push(Line::Record {
            depth: 1,
            text: msg::show_decision_rows(with, rows.len() - with),
        });
    }
    lines.extend(out::report(&report));
    lines
}

/// A row of a blame file as the listing prints it: its range, a tab, its
/// value.
pub(crate) fn blame_row_line(row: &blame::Row) -> Line {
    Line::Record {
        depth: 1,
        text: format!("{}-{}\t{}", row.start, row.end, row.value),
    }
}

/// The lines of a blame file's change, each as written and marked.
fn blame_change_lines(change: &Change) -> Vec<Line> {
    let mut out = Vec::new();
    if !change.baseline {
        out.push(Line::Record {
            depth: 1,
            text: msg::show_no_baseline(),
        });
    }
    out.extend(change.lines.iter().map(|l| Line::Record {
        depth: 1,
        text: l.marked(),
    }));
    out
}

/// The neighbour lines `show --neighbours` prints under each question of
/// `rf`, by heading, read the way it reads them: the file's neighbours are
/// computed once and each heading is looked up in them.
pub(crate) fn neighbours_map(r: &Records, rf: &RecordFile) -> HashMap<String, Vec<String>> {
    let shown = plan::load_shown_with(rf, &r.terms_list, &r.led, &r.index().planning);
    shown
        .questions
        .iter()
        .map(|q| {
            (
                q.heading.clone(),
                plan::neighbour_lines(q)
                    .into_iter()
                    .map(|l| l.trim_start().to_string())
                    .collect(),
            )
        })
        .collect()
}

/// The neighbour lines `show --neighbours` prints under the question
/// `heading` of `rf`.
pub(crate) fn neighbours_of(r: &Records, rf: &RecordFile, heading: &str) -> Vec<String> {
    neighbours_map(r, rf).remove(heading).unwrap_or_default()
}
