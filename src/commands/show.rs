//! `adr show`: what the records hold and every move. With no file, every
//! current decision and every move (per "What does show print with no file
//! named?", c84a4bd9 A). With a file approve takes -- a topic file, the
//! terms file or a blame file -- every question or entry with its
//! status (a blame file: its origin and each row as written), every depends
//! and supersedes line and the body of an asked question, with what the file
//! holds beyond its latest approval marked (per "Does show print what a file
//! holds beyond its latest approval?", bceceda8 A), then the moves of that
//! file. What it computes from the records comes under a flag (per
//! "Does a command print under a flag what it computes from the records?",
//! 36dfbeaa A): `--neighbours`, `--dependents`, `--unrelated` and
//! `--questions` and `--reader`. The reference lines of an answered question
//! are one of them: printed under `--dependents` (per "Does show print the
//! reference lines of an answered question?", 83190b10 A). The reader's word
//! `inconsistent` or `same` on a neighbour comes under a question not yet
//! answered in the plain listing, every word under `--reader` (per "What
//! does show print of the reader's word on a question?", 332ff0c7 A). Every
//! line goes through
//! `out` (per "What does a command that writes nothing print?", 848e1b76 A).
use crate::rules::Command;
use crate::approval::{self, Change, Side};
use crate::blame;
use crate::ledger::{self, Ledger};
use crate::moves::{self, Report};
use crate::msg::{self, Actor};
use crate::out::{self, Line, Status};
use crate::plan;
use crate::record::{self, RecordFile, Section};
use crate::repo;
use crate::rules::{self, Finding};
use crate::state;
use crate::status::{self, FileStatus, QuestionStatus};
use crate::terms::{self, Term, TermsFile};
use crate::view;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// The computed views `show` prints under a flag.
#[derive(Debug, Clone, Copy, Default)]
pub struct Views {
    /// Each neighbour of a question not yet answered, with the reader's word
    /// and the agent's line.
    pub neighbours: bool,
    /// What stands on each question or term entry, and what replaced a
    /// superseded question.
    pub dependents: bool,
    /// The `unrelated:` line of each question, marked when changed.
    pub unrelated: bool,
    /// For a blame file: how many of its decision rows a question names and
    /// how many none names (per "What does check do with a decision row
    /// that no question names?", ee27fd4c A).
    pub questions: bool,
    /// Every word the reader gave on every neighbour of each question, with
    /// its reason.
    pub reader: bool,
}

/// Everything `show` reads, loaded once; the page reads its lines from the
/// same (per "Does the ui show what the terminal prints?", 7857d633 A).
pub(crate) struct Records {
    pub(crate) root: PathBuf,
    pub(crate) led: Ledger,
    pub(crate) terms_file: Option<TermsFile>,
    /// The error of an adrdom/terms.md that exists and cannot be read:
    /// named, never skipped in silence.
    terms_unreadable: Option<String>,
    pub(crate) terms_list: Vec<Term>,
    pub(crate) topic_files: Vec<RecordFile>,
    pub(crate) unreadable_topic_files: Vec<(String, String)>,
    blame: blame::Index,
    pub(crate) unreadable_blame_files: Vec<(String, String)>,
    pub(crate) tests: HashMap<String, Vec<String>>,
    /// Every question's heading, every term's name and every blame file's
    /// name by its identifier: what turns an identifier on a line back into
    /// words.
    pub(crate) names: HashMap<String, String>,
    /// What the judgement of every topic file reads of the whole
    /// repository, read the first time a file is judged and kept for the
    /// rest (`rules::RepoIndex`).
    index: std::cell::OnceCell<rules::RepoIndex>,
    /// The terms file with an identifier on every entry
    /// (`terms::minted_copy`), minted the first time it is needed: the one
    /// list the terms judgement reads, as in `check`.
    minted_terms: std::cell::OnceCell<Option<TermsFile>>,
}

impl Records {
    pub(crate) fn load(root: PathBuf) -> Records {
        let led = ledger::read(&root.join(repo::LEDGER_PATH));
        let (terms_file, terms_unreadable) = match state::load_terms_checked(&root) {
            state::TermsLoad::Ok(tf) => (Some(tf), None),
            state::TermsLoad::LoadError(e) => (None, Some(e)),
            state::TermsLoad::Missing => (None, None),
        };
        // The terms every topic file is judged against, as check judges
        // them: one minted copy, so an entry with no id yet carries one
        // identifier.
        let minted = terms_file.as_ref().map(terms::minted_copy);
        let terms_list: Vec<Term> = minted.as_ref().map(|tf| tf.terms.clone()).unwrap_or_default();
        let (topic_files, unreadable) = record::load_all_via(&root, &repo::Source::WorkingTree);
        let unreadable_topic_files =
            unreadable.into_iter().map(|(f, e)| (state::rel_path(&root, &f), e.to_string())).collect();
        let (blame_files, unreadable_blame_files) = blame::load_all_with_errors(&root);
        let blame = blame::Index::of(blame_files, &led);
        let tests = crate::commands::check::test_index(&root, &repo::Source::WorkingTree);
        let mut names: HashMap<String, String> =
            terms_list.iter().filter_map(|t| Some((t.id.clone()?, t.name.clone()))).collect();
        // A question no topic file holds any more (struck, or its file
        // removed) by the heading its latest `asked` row carries; one a
        // topic file holds by its heading there.
        for row in led.rows.iter().filter(|r| r.event == "asked" && !r.detail.is_empty()) {
            names.insert(row.id.clone(), row.detail.clone());
        }
        names.extend(view::heading_texts(&topic_files));
        // A blame file by the name its path gives, under the identifier its
        // id line holds: what turns the identifier of a `blame:` line back
        // into the name approve turned (a copy of a file's id names nothing,
        // and no blame file's name replaces a question's or a term's).
        for e in blame.entries() {
            let Some(id) = e.id() else { continue };
            if blame.keeper_of(id).is_some_and(|k| k.rel() == e.rel()) {
                names.entry(id.to_ascii_lowercase()).or_insert_with(|| e.name.clone());
            }
        }
        Records {
            root,
            led,
            terms_file,
            terms_unreadable,
            terms_list,
            topic_files,
            unreadable_topic_files,
            blame,
            unreadable_blame_files,
            tests,
            names,
            index: std::cell::OnceCell::new(),
            minted_terms: std::cell::OnceCell::from(minted),
        }
    }

    /// The finding on an adrdom/terms.md that exists and cannot be read.
    pub(crate) fn terms_unreadable_finding(&self) -> Option<Line> {
        let e = self.terms_unreadable.as_ref()?;
        Some(out::refusal(Command::Show, &msg::file_unreadable(repo::TERMS_PATH, e)))
    }

    pub(crate) fn minted_terms(&self) -> Option<&TermsFile> {
        self.minted_terms.get_or_init(|| self.terms_file.as_ref().map(terms::minted_copy)).as_ref()
    }

    pub(crate) fn index(&self) -> &rules::RepoIndex {
        self.index.get_or_init(|| {
            rules::RepoIndex::new_with_blame(
                &self.root,
                &self.topic_files,
                &self.terms_list,
                &self.led,
                &repo::Source::WorkingTree,
                self.blame.clone(),
            )
        })
    }

    fn present_ids(&self) -> HashSet<String> {
        rules::held_ids(&self.topic_files).into_iter().map(str::to_string).collect()
    }

    pub(crate) fn removed_topic_files(&self) -> Vec<String> {
        rules::removal_awaiting(&self.led, &|rel| self.root.join(rel).exists())
    }
}

/// The arguments `show` ran with, as the command line names them: what a
/// refusal's move names to run again (per "Does a refusal name the command
/// that clears it?").
fn show_args(record_arg: Option<&std::path::Path>, all: bool, views: &Views) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if let Some(p) = record_arg {
        args.push(p.display().to_string());
    }
    if all {
        args.push("--all".to_string());
    }
    if views.neighbours {
        args.push("--neighbours".to_string());
    }
    if views.dependents {
        args.push("--dependents".to_string());
    }
    if views.unrelated {
        args.push("--unrelated".to_string());
    }
    if views.questions {
        args.push("--questions".to_string());
    }
    if views.reader {
        args.push("--reader".to_string());
    }
    args
}

pub fn run(record_arg: Option<PathBuf>, all: bool, views: Views) -> i32 {
    let Some(root) = repo::find_root() else {
        let args = show_args(record_arg.as_deref(), all, &views);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        out::eprint(&[out::refusal(Command::Show, &msg::not_a_repo("show", &args, Actor::Agent))]);
        return 1;
    };
    let r = Records::load(root);
    let lines = match &record_arg {
        Some(p) => match file_form(&r, &state::given_rel(&r.root, p).unwrap_or_else(|| p.display().to_string()), views) {
            Ok(lines) => lines,
            Err(refusal) => {
                out::eprint(&[out::refusal(Command::Show, &refusal)]);
                return 1;
            }
        },
        None if all => whole(&r, views),
        None => summary(&r),
    };
    out::print(&lines);
    0
}

// --- the listing with no file named ------------------------------------------

/// Every current decision as its heading and the option picked, under its
/// topic file, and every move: of each topic file, each removed topic file,
/// the terms file and each blame file.
pub(crate) fn summary(r: &Records) -> Vec<Line> {
    let mut lines = unreadable_topic_lines(r);
    // The terms file first: while it waits for its approval, approve, ask
    // and answer stop on it in every topic file (per "Does the tool stop
    // while the terms file waits for approval?"), so its move comes before
    // theirs, and the topic files below do not name it again.
    lines.extend(r.terms_unreadable_finding());
    if let Some(tf) = &r.terms_file {
        let (st, report) = terms_report(r, tf);
        let report = out::report(&report);
        if !report.is_empty() {
            lines.push(file_status(repo::TERMS_PATH, st));
            lines.extend(report);
        }
    }
    // A finding on the records as a whole is in every topic file's report;
    // the listing prints it once.
    let mut printed: Vec<String> = Vec::new();
    for rf in &r.topic_files {
        lines.push(topic_heading(rf));
        lines.extend(decision_lines(rf, &r.led));
        lines.extend(out::report(&listed_report(r, rf, &mut printed)));
    }
    lines.extend(rest(r));
    lines
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
    Line::Record { depth: 0, text: msg::show_topic(rf.topic.as_deref().unwrap_or(""), &rf.rel) }
}

/// Every current decision of `rf` as its heading and the option picked.
pub(crate) fn decision_lines(rf: &RecordFile, led: &Ledger) -> Vec<Line> {
    let mut lines = Vec::new();
    for sec in rf.sections.iter().filter(|s| status::question_status(led, s) == QuestionStatus::Answered) {
        lines.push(Line::Record { depth: 1, text: sec.heading_text.clone() });
        if let Some(s) = sec.chosen_sentence() {
            lines.push(Line::Record { depth: 2, text: s.to_string() });
        }
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
pub(crate) fn listed_report_judged(r: &Records, rf: &RecordFile, printed: &mut Vec<String>) -> (Report, Vec<Finding>) {
    let (mut report, judged) = topic_file_report_judged(r, rf);
    report.findings.retain(|f| {
        let text = f.text();
        if f.subject == rules::Subject::TermsFile && r.terms_file.is_some() || printed.contains(&text) {
            return false;
        }
        printed.push(text);
        true
    });
    (report, judged)
}

/// What the listing prints after the topic files: each removed topic file,
/// each blame file, and the blame files that cannot be read.
pub(crate) fn rest(r: &Records) -> Vec<Line> {
    let mut lines = Vec::new();
    for rel in r.removed_topic_files() {
        lines.push(file_status(&rel, FileStatus::Removed));
        // Its current decisions stand until a plan change supersedes them:
        // listed from its approved version, as every file's are.
        if let Some(old) = rules::latest_approved_version(&r.root, &rel, &r.led) {
            lines.extend(decision_lines(&old, &r.led));
        }
        lines.extend(out::report(&removed_report(r, &rel)));
    }
    for e in r.blame.entries() {
        let report = out::report(&blame_report(r, e));
        if !report.is_empty() {
            lines.push(file_status(e.rel(), e.status));
            lines.extend(report);
        }
    }
    for (rel, _) in &r.unreadable_blame_files {
        lines.push(out::refusal(Command::Show, &msg::blame_file_unreadable(rel)));
    }
    lines
}

// --- the file form -------------------------------------------------------------

/// Every file approve takes, each in the file form.
fn whole(r: &Records, views: Views) -> Vec<Line> {
    let mut lines = Vec::new();
    for (rel, e) in &r.unreadable_topic_files {
        lines.push(out::refusal(Command::Show, &msg::topic_file_unreadable(rel, e)));
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
    for (rel, _) in &r.unreadable_blame_files {
        lines.push(out::refusal(Command::Show, &msg::blame_file_unreadable(rel)));
    }
    lines
}

/// The file form of `rel`, or the refusal when it is not a file approve
/// takes.
pub(crate) fn file_form(r: &Records, rel: &str, views: Views) -> Result<Vec<Line>, String> {
    if rel == repo::TERMS_PATH {
        return match (&r.terms_file, &r.terms_unreadable) {
            (Some(tf), _) => Ok(terms_file(r, tf, views)),
            (None, Some(e)) => Err(msg::file_unreadable(rel, e)),
            (None, None) => Err(msg::read_failed(rel, "no such file", msg::Next::agent("names a file that exists"))),
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
            return Ok(terms_file_lines(r, tf, views, status::terms_file_status(&r.led, tf)));
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

fn file_status(subject: &str, st: FileStatus) -> Line {
    Line::Status { subject: subject.to_string(), status: Status::File(st), mark: None }
}

/// The status line of a part of a file whose `change` beyond its latest
/// approval is shown under it: marked when the part is new or struck as a
/// whole.
fn change_status(subject: &str, status: Status, change: Option<&Change>) -> Line {
    let mark = change.and_then(|c| match c.status {
        FileStatus::New => Some(Side::Added),
        FileStatus::Removed => Some(Side::Removed),
        FileStatus::Changed | FileStatus::Approved => None,
    });
    Line::Status { subject: subject.to_string(), status, mark }
}

/// The lines of a change, marked, the identifiers of its reference lines
/// turned back into words. `own` is the part's own heading line (`### ..`
/// of a question, `## ..` of a term entry): left out, the status line above
/// names it, unless the heading itself changed. Every other line is
/// printed, one that starts like a heading included. A changed part whose
/// approved text no commit holds says so above its lines, all marked added.
fn change_lines(change: &Change, own: Option<&str>, names: &HashMap<String, String>, unrelated: bool) -> Vec<Line> {
    let prefix = own.and_then(|o| o.split_once(' ').map(|(p, _)| format!("{} ", p)));
    // The heading changed: a heading line gone and one come, both shown.
    let heading_changed = prefix.as_deref().is_some_and(|p| {
        [Side::Removed, Side::Added].iter().all(|side| change.lines.iter().any(|l| l.side == *side && l.text.starts_with(p)))
    });
    let mut out = Vec::new();
    if !change.baseline {
        out.push(Line::Record { depth: 1, text: msg::show_no_baseline() });
    }
    out.extend(
        change
            .lines
            .iter()
            .filter(|l| heading_changed || own != Some(l.text.as_str()))
            .filter(|l| unrelated || !l.text.starts_with("unrelated:"))
            .map(|l| Line::Record { depth: 1, text: reference_named(&l.marked(), &l.text, names) }),
    );
    out
}

/// `printed`, the printed form of the record line `line`, with the
/// identifiers turned back into words when `line` is a reference line
/// (`depends:`, `supersedes:`, `unrelated:`, `blame:`): what approve turns
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
    ["depends:", "supersedes:", "unrelated:", "blame:"].iter().any(|p| line.starts_with(p))
}

/// A line the records hold with every identifier on it turned back into
/// the heading or the name it stands for (per "Does a reference in a topic
/// file carry the identifier of what it names?": show turns it back into
/// the name). An identifier no question or term holds stays as it is.
fn named(line: &str, names: &HashMap<String, String>) -> String {
    const LEN: usize = 36;
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < line.len() {
        if let Some(cand) = line.get(i..i + LEN) {
            let bounded = !line[..i].ends_with(|c: char| c.is_ascii_hexdigit())
                && !line[i + LEN..].starts_with(|c: char| c.is_ascii_hexdigit());
            if bounded && record::is_uuid(cand) {
                if let Some(name) = names.get(cand).or_else(|| names.get(&cand.to_ascii_lowercase())) {
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
/// `unrelated:`, `blame:`), and its body, each as the file holds it.
fn reference_and_body(lines: &[String]) -> (Vec<String>, Vec<String>) {
    let rest = lines.iter().skip(1);
    (rest.clone().filter(|l| is_reference(l)).cloned().collect(), rest.filter(|l| !is_reference(l)).cloned().collect())
}

/// One topic file: its status, its prose when beyond its latest approval,
/// every question, every question of the approved plan struck from it, then
/// its moves.
fn topic_file(r: &Records, rf: &RecordFile, views: Views) -> Vec<Line> {
    let mut lines = topic_file_lines(r, rf, views);
    lines.extend(out::report(&topic_file_report(r, rf)));
    lines
}

/// A topic file in the file form without its findings and moves, for a
/// caller that holds the file's report already.
fn topic_file_lines(r: &Records, rf: &RecordFile, views: Views) -> Vec<Line> {
    let led = &r.led;
    let mut lines = vec![topic_file_status_line(led, rf)];
    let changes = approval::topic_file(&r.root, rf, led);
    if let Some(prose) = &changes.prose {
        lines.push(change_status("prose", Status::File(prose.status), Some(prose)));
        lines.extend(change_lines(prose, None, &r.names, true));
    }
    let shown = (views.neighbours).then(|| plan::load_shown_with(rf, &r.terms_list, led, &r.index().planning));
    let by_body = plan::BodyIndex::of(rf);
    for (i, sec) in rf.sections.iter().enumerate() {
        let st = status::question_status(led, sec);
        let change = changes.questions.get(i).and_then(Option::as_ref);
        lines.push(change_status(&sec.heading_text, question_status_of(led, rf, sec), change));
        match change {
            Some(change) => lines.extend(change_lines(change, Some(&format!("### {}", sec.heading_text)), &r.names, views.unrelated)),
            None => lines.extend(reference_and_body_lines(
                &approval::question_lines(rf, sec),
                st,
                &r.names,
                views.unrelated,
                views.dependents || !is_answered(st),
            )),
        }
        if views.reader || !is_answered(st) {
            lines.extend(reader_lines(r, rf, i, views.reader, &by_body));
        }
        if views.neighbours && matches!(st, QuestionStatus::NoBody | QuestionStatus::NotAsked | QuestionStatus::Asked) {
            if let Some(q) = shown.as_ref().and_then(|p| p.questions.iter().find(|q| q.heading == sec.heading_text)) {
                lines.extend(
                    plan::neighbour_lines(q)
                        .into_iter()
                        .map(|l| Line::Record { depth: 1, text: l.trim_start().to_string() }),
                );
            }
        }
        if views.dependents {
            lines.extend(stood_on_lines(r, rf, sec, st));
        }
    }
    if changes.struck_unsure {
        lines.push(Line::Record { depth: 0, text: msg::show_struck_unsure() });
    }
    for (heading, change) in &changes.struck {
        // An identifier stands for a struck question whose approved text no
        // commit holds: turned back into the heading its asked row carries.
        let subject = names_of(heading, &r.names);
        lines.push(change_status(&subject, Status::File(change.status), Some(change)));
        lines.extend(change_lines(change, Some(&format!("### {}", heading)), &r.names, views.unrelated));
    }
    lines
}

/// The status of a question as a status line names it: its status, the
/// letter of an answer, the version of an asked question whose text stands
/// as asked, or `changed since asked` for one edited since.
pub(crate) fn question_status_of(led: &Ledger, rf: &RecordFile, sec: &Section) -> Status {
    let st = status::question_status_in(led, rf, sec);
    let version = (st == QuestionStatus::Asked).then(|| record::version_of(&rf.lines, sec));
    Status::Question { status: st, letter: answer_letter(led, sec), version }
}

/// The letter of a question's standing answer: its `**Answer:**` line, or
/// the ledger's latest `answered` row when the line is not written yet (an
/// answer not yet committed, read from a committed version).
fn answer_letter(led: &Ledger, sec: &Section) -> Option<char> {
    sec.answer_letter().or_else(|| {
        let id = sec.id.as_deref()?;
        (status::question_status(led, sec) == QuestionStatus::Answered)
            .then(|| led.latest_for_id("answered", id).and_then(|r| r.detail.chars().next()))
            .flatten()
    })
}

/// A question's reference lines with identifiers turned back into words,
/// and its body when it is asked: what the file form prints under a
/// question approved as it stands.
/// `show_refs` is whether the reference lines print: those of an answered
/// question do not, unless `--dependents` asks (per "Does show print the
/// reference lines of an answered question?", 83190b10 A). `--unrelated`
/// prints the `unrelated:` line of every question, answered or not.
fn reference_and_body_lines(
    qlines: &[String],
    st: QuestionStatus,
    names: &HashMap<String, String>,
    unrelated: bool,
    show_refs: bool,
) -> Vec<Line> {
    // The `id:` line is printed only as part of a change (what approve signs
    // beyond its latest approval), never in the listing of a question.
    let qlines: Vec<String> = qlines.iter().filter(|l| !l.starts_with("id: ")).cloned().collect();
    let (refs, body) = reference_and_body(&qlines);
    let refs: Vec<String> =
        refs.into_iter().filter(|l| if l.starts_with("unrelated:") { unrelated } else { show_refs }).collect();
    let body = if st == QuestionStatus::Asked { body } else { Vec::new() };
    refs.iter()
        .map(|l| Line::Record { depth: 1, text: named(l, names) })
        .chain(body.iter().map(|l| Line::Record { depth: 1, text: l.clone() }))
        .collect()
}

/// Whether the owner has acted on the question: picked an option or
/// refused it. A superseded question was answered first.
fn is_answered(st: QuestionStatus) -> bool {
    matches!(st, QuestionStatus::Answered | QuestionStatus::Refused | QuestionStatus::Superseded)
}

/// The reader's words under question `idx` of `rf`: with `all`, every word
/// on every neighbour, the reason a line under at depth 2; otherwise the
/// neighbours the reader marked `inconsistent` or `same` that the
/// question's `supersedes:` line does not name (per "What does show print of
/// the reader's word on a question?", 332ff0c7 A). A view: no move stands on
/// a line.
fn reader_lines(r: &Records, rf: &RecordFile, idx: usize, all: bool, by_body: &plan::BodyIndex) -> Vec<Line> {
    let words = r.index().planning.reader_words(rf, idx, &r.terms_list, &r.led, by_body);
    let mut lines = Vec::new();
    for w in words {
        if all {
            lines.push(Line::Word { word: w.word, name: w.name });
            lines.push(Line::Record { depth: 2, text: w.reason });
        } else if matches!(w.word.as_str(), "inconsistent" | "same") && !w.on_supersedes {
            lines.push(Line::Word { word: w.word, name: w.name });
        }
    }
    lines
}

/// `text` with an identifier turned back into the words it stands for.
fn names_of(text: &str, names: &HashMap<String, String>) -> String {
    named(text, names)
}

/// What stands on `sec`, and for a superseded question what replaced it.
fn stood_on_lines(r: &Records, rf: &RecordFile, sec: &Section, st: QuestionStatus) -> Vec<Line> {
    let mut lines = Vec::new();
    let dependents = stood_on_by(rf, sec, &r.topic_files, &r.led);
    if !dependents.is_empty() {
        lines.push(Line::Record { depth: 1, text: format!("dependents: {}", dependents.join(", ")) });
    }
    // Each `superseded` row holds the id of a question that replaced it; a
    // decision several questions supersede has one row for each (per "Does
    // one decision supersede several?"), named in ledger order.
    if st == QuestionStatus::Superseded {
        let mut by: Vec<String> = Vec::new();
        if let Some(id) = sec.id.as_deref() {
            for row in r.led.rows.iter().filter(|row| row.id == id && row.event == "superseded") {
                let b = row.detail.trim();
                if b.is_empty() {
                    continue;
                }
                let name = r.names.get(b).cloned().unwrap_or_else(|| b.to_string());
                if !by.contains(&name) {
                    by.push(name);
                }
            }
        }
        if !by.is_empty() {
            lines.push(Line::Record { depth: 1, text: format!("replaced by: {}", by.join(", ")) });
        }
    }
    lines
}

/// The questions whose depends line names `sec`, in the order of the topic
/// files, each by its heading and, for a question of another file, the
/// path of that file. A refused, superseded or withdrawn question is left
/// out: its depends line is a record of what it stood on, nothing rests on
/// `sec` through it any more. An entry written as a heading text names
/// `sec` in its own file, and in another file only while `sec` is a
/// current decision (per "What does a depends line name?").
fn stood_on_by(rf: &RecordFile, sec: &Section, all: &[RecordFile], led: &Ledger) -> Vec<String> {
    let heading = sec.heading_text.trim();
    let current = sec.id.as_deref().is_some_and(|id| state::is_current(led, id));
    let mut out = Vec::new();
    for other in all {
        let same_file = other.rel == rf.rel;
        for dependent in &other.sections {
            if (same_file && dependent.heading_text.trim() == heading) || state::is_dead(led, dependent) {
                continue;
            }
            let names_it = dependent.depends.iter().any(|d| match d {
                record::Dep::Id { id, .. } => sec.id.as_deref() == Some(id.as_str()),
                record::Dep::Text { text, .. } => text.trim() == heading && (same_file || current),
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
/// records it is (per "Does a command print under a flag what it computes
/// from the records?", 36dfbeaa A).
pub(crate) fn term_dependents(r: &Records, t: &Term) -> Vec<String> {
    let names: Vec<String> = std::iter::once(&t.name).chain(&t.aliases).map(|n| n.trim().to_lowercase()).collect();
    let mut out = Vec::new();
    for rf in &r.topic_files {
        for sec in rf.sections.iter().filter(|s| status::question_status(&r.led, s) == QuestionStatus::Answered) {
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

/// A topic file gone from disk whose removal the owner has not approved:
/// its status, each question of its approved plan with the question's own
/// status (per "Does a command name a status of its own?", f53ac3b9 A), and
/// its move.
fn removed_topic_file(r: &Records, rel: &str, views: Views) -> Vec<Line> {
    let mut lines = vec![file_status(rel, FileStatus::Removed)];
    if let Some(old) = rules::latest_approved_version(&r.root, rel, &r.led) {
        for sec in &old.sections {
            let st = status::question_status(&r.led, sec);
            lines.push(Line::Status { subject: sec.heading_text.clone(), status: question_status_of(&r.led, &old, sec), mark: None });
            lines.extend(reference_and_body_lines(
                &approval::question_lines(&old, sec),
                st,
                &r.names,
                views.unrelated,
                views.dependents || !is_answered(st),
            ));
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
        lines.extend(change_lines(whole, None, &r.names, true));
    }
    for (t, change) in tf.terms.iter().zip(&changes.entries) {
        lines.push(change_status(&t.name, Status::Term(status::term_status(&r.led, tf, t)), change.as_ref()));
        match change {
            Some(change) => lines.extend(change_lines(change, Some(&format!("## {}", t.name)), &r.names, true)),
            None => lines.extend(
                approval::entry_lines(tf, t)
                    .into_iter()
                    .skip(1)
                    .map(|l| Line::Record { depth: 1, text: named(&l, &r.names) }),
            ),
        }
        if views.dependents {
            let dependents = term_dependents(r, t);
            if !dependents.is_empty() {
                lines.push(Line::Record { depth: 1, text: format!("dependents: {}", dependents.join(", ")) });
            }
        }
    }
    for (name, change) in &changes.gone {
        lines.push(change_status(name, Status::Term(status::TermStatus::Removed), Some(change)));
        lines.extend(change_lines(change, Some(&format!("## {}", name)), &r.names, true));
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
                lines.push(Line::Record { depth: 1, text: format!("origin: {}", origin) });
            }
            lines.extend(bf.rows.iter().map(blame_row_line));
        }
    }
    if views.questions {
        let rows = blame::decision_rows(e, &r.blame, &r.topic_files);
        let with = rows.iter().filter(|&&(_, named)| named).count();
        lines.push(Line::Record { depth: 1, text: msg::show_decision_rows(with, rows.len() - with) });
    }
    lines.extend(out::report(&report));
    lines
}

/// A row of a blame file as the listing prints it: its range, a tab, its
/// value.
pub(crate) fn blame_row_line(row: &blame::Row) -> Line {
    Line::Record { depth: 1, text: format!("{}-{}\t{}", row.start, row.end, row.value) }
}

/// The lines of a blame file's change, each as written and marked.
fn blame_change_lines(change: &Change) -> Vec<Line> {
    let mut out = Vec::new();
    if !change.baseline {
        out.push(Line::Record { depth: 1, text: msg::show_no_baseline() });
    }
    out.extend(change.lines.iter().map(|l| Line::Record { depth: 1, text: l.marked() }));
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
        .map(|q| (q.heading.clone(), plan::neighbour_lines(q).into_iter().map(|l| l.trim_start().to_string()).collect()))
        .collect()
}

/// The neighbour lines `show --neighbours` prints under the question
/// `heading` of `rf`.
pub(crate) fn neighbours_of(r: &Records, rf: &RecordFile, heading: &str) -> Vec<String> {
    neighbours_map(r, rf).remove(heading).unwrap_or_default()
}

// --- findings and moves -----------------------------------------------------

pub(crate) fn topic_file_report(r: &Records, rf: &RecordFile) -> Report {
    topic_file_report_judged(r, rf).0
}

pub(crate) fn topic_file_report_judged(r: &Records, rf: &RecordFile) -> (Report, Vec<Finding>) {
    moves::of_topic_file_judged(&r.root, rf, &r.topic_files, &r.terms_list, &r.led, &r.tests, r.index())
}

fn removed_report(r: &Records, rel: &str) -> Report {
    moves::of_removed_topic_file(rel, rules::judge_removed_topic_file(&r.root, rel, &r.led, &r.present_ids()))
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
    (st, moves::of_terms_file(findings, st))
}

/// The findings and moves of a blame file (`moves::of_blame_file`).
fn blame_report(r: &Records, e: &blame::Entry) -> Report {
    moves::of_blame_file(&r.root, e, &r.blame, Command::Show)
}
