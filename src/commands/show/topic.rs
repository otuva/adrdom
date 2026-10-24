//! The file form of a topic file: each question with its status, body, references and dependents.
use super::file::{change_lines, change_status, named, reference_and_body, topic_file_status_line};
use super::findings::topic_file_report;
use super::{Records, Views};
use crate::approval::{self};
use crate::ledger::Ledger;
use crate::msg::{self};
use crate::out::{self, Line, Status};
use crate::plan;
use crate::record::{self, RecordFile, Section};
use crate::state;
use crate::status::{self, QuestionStatus};
use crate::terms::{self, Term};
use std::collections::HashMap;

/// One topic file: its status, its prose when beyond its latest approval,
/// every question, every question of the approved plan struck from it, then
/// its moves.
pub(super) fn topic_file(r: &Records, rf: &RecordFile, views: Views) -> Vec<Line> {
    let mut lines = topic_file_lines(r, rf, views);
    lines.extend(out::report(&topic_file_report(r, rf)));
    lines
}

/// A topic file in the file form without its findings and moves, for a
/// caller that holds the file's report already.
pub(super) fn topic_file_lines(r: &Records, rf: &RecordFile, views: Views) -> Vec<Line> {
    let led = &r.led;
    let mut lines = vec![topic_file_status_line(led, rf)];
    let changes = approval::topic_file(&r.root, rf, led);
    if let Some(prose) = &changes.prose {
        lines.push(change_status(
            "prose",
            Status::File(prose.status),
            Some(prose),
        ));
        lines.extend(change_lines(prose, None, &r.names));
    }
    let shown = (views.neighbours)
        .then(|| plan::load_shown_with(rf, &r.terms_list, led, &r.index().planning));
    // Built on the first question that reads the reader's words.
    let by_body = std::cell::OnceCell::new();
    // Struck superseded questions come first: older than every section left.
    if views.superseded {
        lines.extend(struck_superseded_lines(r, rf, views));
    }
    for (i, sec) in rf.sections.iter().enumerate() {
        let st = status::question_status(led, sec);
        let change = changes.questions.get(i).and_then(Option::as_ref);
        // A superseded question approved as it stands is read by no move:
        // it prints under --superseded. One changed since its approval is
        // what approve reads, and prints marked as every change does.
        if st == QuestionStatus::Superseded && change.is_none() && !views.superseded {
            continue;
        }
        lines.push(change_status(
            &sec.heading_text,
            question_status_of(led, rf, sec),
            change,
        ));
        if views.ids {
            lines.extend(super::listing::id_line(sec, 1));
        }
        lines.extend(chosen_line(led, sec, st, 1));
        match change {
            Some(change) => lines.extend(change_lines(
                change,
                Some(&format!("### {}", sec.heading_text)),
                &r.names,
            )),
            None => lines.extend(reference_and_body_lines(
                &approval::question_lines(rf, sec),
                st,
                &r.names,
                views.dependents || !is_answered(st),
            )),
        }
        if views.reader || !is_answered(st) {
            lines.extend(reader_lines(
                r,
                rf,
                i,
                views.reader,
                by_body.get_or_init(|| plan::BodyIndex::of(rf)),
            ));
        }
        if views.neighbours
            && matches!(
                st,
                QuestionStatus::NoBody | QuestionStatus::NotAsked | QuestionStatus::Asked
            )
        {
            if let Some(q) = shown
                .as_ref()
                .and_then(|p| p.questions.iter().find(|q| q.heading == sec.heading_text))
            {
                lines.extend(plan::neighbour_lines(q).into_iter().map(|l| Line::Record {
                    depth: 1,
                    text: l.trim_start().to_string(),
                }));
            }
        }
        if views.dependents {
            lines.extend(stood_on_lines(r, rf, sec));
        }
        if views.superseded && st == QuestionStatus::Superseded {
            lines.extend(replaced_by_lines(r, sec));
        }
    }
    if changes.struck_unsure {
        lines.push(Line::Record {
            depth: 0,
            text: msg::show_struck_unsure(),
        });
    }
    for (heading, change) in &changes.struck {
        // An identifier stands for a struck question whose approved text no
        // commit holds: turned back into the heading its asked row carries.
        let subject = names_of(heading, &r.names);
        lines.push(change_status(
            &subject,
            Status::File(change.status),
            Some(change),
        ));
        lines.extend(change_lines(
            change,
            Some(&format!("### {}", heading)),
            &r.names,
        ));
    }
    lines
}

/// The status of a question as a status line names it: its status, the
/// letter of an answer, the version of an asked question whose text stands
/// as asked, or `changed since asked` for one edited since.
pub(crate) fn question_status_of(led: &Ledger, rf: &RecordFile, sec: &Section) -> Status {
    let st = status::question_status_in(led, rf, sec);
    let version = (st == QuestionStatus::Asked).then(|| record::version_of(&rf.lines, sec));
    Status::Question {
        status: st,
        letter: answer_letter(led, sec),
        version,
    }
}

/// The letter of a question's standing answer: its `**Answer:**` line, or
/// the ledger's latest `answered` row when the line is not written yet (an
/// answer not yet committed, read from a committed version).
fn answer_letter(led: &Ledger, sec: &Section) -> Option<char> {
    sec.answer_letter().or_else(|| {
        let id = sec.id.as_deref()?;
        (status::question_status(led, sec) == QuestionStatus::Answered)
            .then(|| {
                led.latest_for_id("answered", id)
                    .and_then(|r| r.detail.chars().next())
            })
            .flatten()
    })
}

/// A question's reference lines with identifiers turned back into words,
/// and its body when it is asked: what the file form prints under a
/// question approved as it stands.
/// `show_refs` is whether the reference lines print: those of an answered
/// question do not, unless `--dependents` asks (per "Does show print the
/// reference lines of a question that is answered?", d1d6fed2 A).
pub(super) fn reference_and_body_lines(
    qlines: &[String],
    st: QuestionStatus,
    names: &HashMap<String, String>,
    show_refs: bool,
) -> Vec<Line> {
    // The `id:` line is printed only as part of a change (what approve signs
    // beyond its latest approval), never in the listing of a question.
    let qlines: Vec<String> = qlines
        .iter()
        .filter(|l| !l.starts_with("id: "))
        .cloned()
        .collect();
    let (refs, body) = reference_and_body(&qlines);
    let refs: Vec<String> = if show_refs { refs } else { Vec::new() };
    let body = if st == QuestionStatus::Asked {
        body
    } else {
        Vec::new()
    };
    refs.iter()
        .map(|l| Line::Record {
            depth: 1,
            text: named(l, names),
        })
        .chain(body.iter().map(|l| Line::Record {
            depth: 1,
            text: l.clone(),
        }))
        .collect()
}

/// Whether the owner has acted on the question: picked an option or
/// refused it. A superseded question was answered first.
pub(super) fn is_answered(st: QuestionStatus) -> bool {
    matches!(
        st,
        QuestionStatus::Answered | QuestionStatus::Refused | QuestionStatus::Superseded
    )
}

/// The reader's words under question `idx` of `rf`: with `all`, every word
/// on every neighbour, the reason a line under at depth 2; otherwise the
/// neighbours the reader marked `inconsistent`, `same` or `closes` with its
/// letter that the question's `supersedes:` line does not name (per "What
/// does show print of a reader's word on a question?", 6e72f34e A). A view: no
/// move stands on a line.
fn reader_lines(
    r: &Records,
    rf: &RecordFile,
    idx: usize,
    all: bool,
    by_body: &plan::BodyIndex,
) -> Vec<Line> {
    let words = r
        .index()
        .planning
        .reader_words(rf, idx, &r.terms_list, &r.led, by_body);
    let mut lines = Vec::new();
    for w in words {
        if all {
            lines.push(Line::Word {
                word: w.word,
                name: w.name,
            });
            lines.push(Line::Record {
                depth: 2,
                text: w.reason,
            });
        } else if crate::out::word_is_marked(&w.word) && !w.on_supersedes {
            lines.push(Line::Word {
                word: w.word,
                name: w.name,
            });
        }
    }
    lines
}

/// `text` with an identifier turned back into the words it stands for.
fn names_of(text: &str, names: &HashMap<String, String>) -> String {
    named(text, names)
}

/// The option the owner picked, under a current decision: the sentence the
/// build move reads (per "What does show print for a file under adrdom?",
/// b0acc95b A), the one line the listing with no file named and the file
/// form both print. The letter is the one the status line names
/// (`answer_letter`): the `**Answer:**` line, or the ledger's when the text
/// read predates that line.
pub(super) fn chosen_line(
    led: &Ledger,
    sec: &Section,
    st: QuestionStatus,
    depth: usize,
) -> Option<Line> {
    if st != QuestionStatus::Answered {
        return None;
    }
    let letter = answer_letter(led, sec)?;
    let sentence = sec
        .options
        .iter()
        .find(|o| o.letter == letter)
        .and_then(|o| o.sentence.as_deref())?;
    Some(Line::Record {
        depth,
        text: sentence.to_string(),
    })
}

/// What stands on `sec`: what `--dependents` prints under a question.
fn stood_on_lines(r: &Records, rf: &RecordFile, sec: &Section) -> Vec<Line> {
    let dependents = stood_on_by(rf, sec, &r.topic_files, &r.led);
    if dependents.is_empty() {
        return Vec::new();
    }
    vec![Line::Record {
        depth: 1,
        text: format!("dependents: {}", dependents.join(", ")),
    }]
}

/// The superseded questions of the file `rf` whose section no topic file
/// keeps (per "What strikes the section of a superseded question from its
/// topic file?"): every id whose latest state is `superseded` and whose
/// latest per-question `approved` row names this file, in the order of
/// their first `superseded` rows. The heading is the latest `asked` row's;
/// the reference lines (with `--dependents`) are those of the newest
/// committed version of the file that holds the question, none when no
/// commit does. adr-code: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
fn struck_superseded_lines(r: &Records, rf: &RecordFile, views: Views) -> Vec<Line> {
    let led = &r.led;
    let in_file: std::collections::HashSet<&str> =
        rf.sections.iter().filter_map(|s| s.id.as_deref()).collect();
    let mut ids: Vec<&str> = Vec::new();
    for row in led.rows.iter().filter(|row| row.event == "superseded") {
        let id = row.id.as_str();
        if !ids.contains(&id)
            && !in_file.contains(id)
            && state::is_superseded(led, id)
            && led
                .latest_for_id("approved", id)
                .is_some_and(|a| a.detail == rf.rel)
        {
            ids.push(id);
        }
    }
    let mut lines = Vec::new();
    for id in ids {
        let heading = led
            .latest_asked(id)
            .map(|a| a.detail.clone())
            .unwrap_or_else(|| id.to_string());
        lines.push(Line::Status {
            subject: heading,
            status: Status::Question {
                status: QuestionStatus::Superseded,
                letter: None,
                version: None,
            },
            mark: None,
        });
        if views.dependents {
            let old = approval::committed_topics(&r.root, &rf.rel)
                .find(|v| v.sections.iter().any(|s| s.id.as_deref() == Some(id)));
            if let Some((old, sec)) = old.as_ref().and_then(|o| {
                o.sections
                    .iter()
                    .find(|s| s.id.as_deref() == Some(id))
                    .map(|s| (o, s))
            }) {
                lines.extend(reference_and_body_lines(
                    &approval::question_lines(old, sec),
                    QuestionStatus::Superseded,
                    &r.names,
                    true,
                ));
            }
        }
        lines.extend(replaced_by_lines_of(r, id));
    }
    lines
}

/// What replaced a superseded question: what `--superseded` prints under
/// it. Each `superseded` row holds the id of a question that replaced it; a
/// decision several questions supersede has one row for each (per "Does
/// one decision supersede several?"), named in ledger order.
pub(super) fn replaced_by_lines(r: &Records, sec: &Section) -> Vec<Line> {
    sec.id
        .as_deref()
        .map(|id| replaced_by_lines_of(r, id))
        .unwrap_or_default()
}

/// `replaced_by_lines` of a question named by its identifier.
fn replaced_by_lines_of(r: &Records, id: &str) -> Vec<Line> {
    let mut by: Vec<String> = Vec::new();
    for row in r
        .led
        .rows
        .iter()
        .filter(|row| row.id == id && row.event == "superseded")
    {
        let b = row.detail.trim();
        if b.is_empty() {
            continue;
        }
        let name = r.names.get(b).cloned().unwrap_or_else(|| b.to_string());
        if !by.contains(&name) {
            by.push(name);
        }
    }
    if by.is_empty() {
        return Vec::new();
    }
    vec![Line::Record {
        depth: 1,
        text: format!("replaced by: {}", by.join(", ")),
    }]
}

/// The questions whose depends line names `sec`, in the order of the topic
/// files, each by its heading and, for a question of another file, the
/// path of that file. A refused, superseded or withdrawn question is left
/// out: its depends line is a record of what it stood on, nothing rests on
/// `sec` through it any more. An entry written as a heading text names
/// `sec` in its own file only; an identifier names it wherever the line
/// stands, the record of what an old question stood on (per "What does a
/// depends entry name?").
fn stood_on_by(rf: &RecordFile, sec: &Section, all: &[RecordFile], led: &Ledger) -> Vec<String> {
    let heading = sec.heading_text.trim();
    let mut out = Vec::new();
    for other in all {
        let same_file = other.rel == rf.rel;
        for dependent in &other.sections {
            if (same_file && dependent.heading_text.trim() == heading)
                || state::is_dead(led, dependent)
            {
                continue;
            }
            let names_it = dependent.depends.iter().any(|d| match d {
                record::Dep::Id { id, .. } => sec.id.as_deref() == Some(id.as_str()),
                record::Dep::Text { text, .. } => same_file && text.trim() == heading,
            });
            if names_it {
                out.push(if same_file {
                    dependent.heading_text.clone()
                } else {
                    msg::show_question_of(&dependent.heading_text, &other.rel)
                });
            }
        }
    }
    out
}

/// The current decisions that stand on the term `t`: those whose terms line
/// pairs a word to it, or whose depends line names it, by its identifier or
/// by its name or an alias; each by its heading and the path of its file.
/// What `--dependents` prints under a term entry, the view computed from the
/// records it is (per "Does a command print a computed view under a flag?",
/// cdc905a2 A).
pub(crate) fn term_dependents(r: &Records, t: &Term) -> Vec<String> {
    let names: Vec<String> = std::iter::once(&t.name)
        .chain(&t.aliases)
        .map(|n| n.trim().to_lowercase())
        .collect();
    let mut out = Vec::new();
    for rf in &r.topic_files {
        for sec in rf
            .sections
            .iter()
            .filter(|s| status::question_status(&r.led, s) == QuestionStatus::Answered)
        {
            let on_terms_line = t.id.as_deref().is_some_and(|id| {
                sec.terms_line
                    .and_then(|i| rf.lines.get(i))
                    .and_then(|l| terms::parse_terms_line(l))
                    .is_some_and(|pairs| pairs.iter().any(|(_, tid)| tid == id))
            });
            let on_depends_line = sec.depends.iter().any(|d| match d {
                record::Dep::Id { id, .. } => t.id.as_deref() == Some(id.as_str()),
                record::Dep::Text { text, .. } => names.contains(&text.trim().to_lowercase()),
            });
            if on_terms_line || on_depends_line {
                out.push(msg::show_question_of(&sec.heading_text, &rf.rel));
            }
        }
    }
    out
}
