//! The page's routes. Each renders the lines a command prints, every one
//! through `out::html`, read from the same computation the command reads
//! (per "Does the ui show what the terminal prints?", 7857d633 A): `/` is
//! what `adr show` prints, with the change beyond approval of a file whose
//! approval is the owner's move, then what `adr check` prints, then the
//! status of every file approve takes; `/next` is every asked question as
//! `adr show <file>` prints it; `/terms` is `adr show adrdom/terms.md`. A
//! move of the owner is rendered with the form that makes it.
use super::html::{esc, page};
use crate::commands::show::{self, Records, Views};
use crate::moves::{Move, MoveKind};
use crate::out::{self, Line, Status};
use crate::repo;
use crate::status::QuestionStatus;
use std::path::Path;

/// What the last act printed (its events, and the findings on its input or
/// that stopped it), shown once above the page it leads to. Each line keeps
/// its own kind; the banner reads as a stop only when the act wrote nothing.
pub(super) fn banner(lines: &[Line]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let stopped = !lines.iter().any(|l| matches!(l, Line::Event(_)));
    format!(
        "<div class=\"banner{}\">\n{}</div>\n",
        if stopped { " refused" } else { "" },
        plain(lines)
    )
}

fn plain(lines: &[Line]) -> String {
    lines.iter().map(|l| format!("{}\n", out::html(l))).collect()
}

/// `/`: every current decision and every move, as `adr show` prints them,
/// each owner's move with its form. Where the move is the owner's approve,
/// the file's change beyond its latest approval (as `adr show <file>`
/// prints it) stands above it. Then every finding `adr check` prints, the
/// status of every file approve takes, and the form for unban, the owner
/// act no move names (per "Does the ui perform every owner act?").
pub(super) fn overview(root: &Path, last: &[Line], token: &str) -> String {
    let r = Records::load(root.to_path_buf());
    let mut body = banner(last);

    body.push_str("<section class=\"listing\">\n");
    let summary = show::summary(&r);
    // An answer is made on /next, where the question's options stand
    // beside the form: here the answer move links there.
    for line in &summary {
        body.push_str(&with_form(line, &r, token, true, false));
    }
    body.push_str("</section>\n");

    // What `adr check` prints beyond the findings the listing above holds:
    // a finding is shown once on the page.
    let shown: Vec<String> = summary.iter().filter(|l| matches!(l, Line::Finding(_))).map(out::text).collect();
    let report = crate::commands::check::collect_with_tests(root, &r.tests);
    let findings: Vec<Line> = report
        .failures
        .into_iter()
        .chain(report.notes)
        .map(Line::Finding)
        .filter(|l| !shown.contains(&out::text(l)))
        .collect();
    if !findings.is_empty() {
        body.push_str(&format!("<section class=\"findings\">\n{}</section>\n", plain(&findings)));
    }

    let statuses: Vec<Line> = show::file_statuses(&r).into_iter().map(|(_, l)| l).collect();
    body.push_str(&format!("<section class=\"files\">\n{}</section>\n", plain(&statuses)));

    body.push_str(&format!(
        "<details class=\"unban\"><summary>adr unban</summary>\n<form method=\"post\" action=\"/unban\"><textarea name=\"lines\" rows=\"4\"></textarea><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Unban</button></form>\n</details>\n",
        esc(token)
    ));
    page("adr show", "/", &body, false)
}

/// `/next`: every asked question of every topic file, each with its status,
/// the lines it carries and its body as `adr show <file>` prints them, then
/// the move on it: the owner's answer, with its form, or what it waits on.
/// The file's other moves follow its questions.
pub(super) fn questions(root: &Path, last: &[Line], token: &str) -> String {
    let r = Records::load(root.to_path_buf());
    let mut body = banner(last);
    for rf in &r.topic_files {
        let asks = rf.sections.iter().any(|s| crate::status::question_status(&r.led, s) == QuestionStatus::Asked);
        if !asks {
            continue;
        }
        let Ok(lines) = show::file_form(&r, &rf.rel, Views::default()) else { continue };
        let (head, blocks, findings, moves) = split(lines);
        body.push_str("<article class=\"file\">\n");
        body.push_str(&plain(&head));
        let mut placed = vec![false; moves.len()];
        for block in blocks.iter().filter(|b| is_asked(&b[0])) {
            body.push_str(&format!("<div class=\"question\">\n{}", plain(block)));
            let heading = status_subject(&block[0]);
            for (i, m) in moves.iter().enumerate() {
                if move_heading(m) == Some(heading) {
                    body.push_str(&with_form(&Line::Move(m.clone()), &r, token, true, true));
                    placed[i] = true;
                }
            }
            body.push_str("</div>\n");
        }
        body.push_str(&plain(&findings));
        for (m, _) in moves.iter().zip(&placed).filter(|(_, p)| !**p) {
            body.push_str(&with_form(&Line::Move(m.clone()), &r, token, true, true));
        }
        body.push_str("</article>\n");
    }
    page("adr show: asked", "/next", &body, true)
}

/// `/terms`: what `adr show adrdom/terms.md` prints.
pub(super) fn terms(root: &Path, last: &[Line], token: &str) -> String {
    let r = Records::load(root.to_path_buf());
    let mut body = banner(last);
    match show::file_form(&r, repo::TERMS_PATH, Views::default()) {
        Ok(lines) => {
            // The whole file form is on this page, its change among it.
            for l in &lines {
                body.push_str(&with_form(l, &r, token, false, false));
            }
        }
        Err(e) => body.push_str(&plain(&out::stop(crate::rules::Rule::Show, &e))),
    }
    page("adr show adrdom/terms.md", "/terms", &body, false)
}

/// The line, and under an owner's move the form that makes it: approve for
/// an approve, the letters and refuse for an answer. Above an approve form
/// stands what the approve signs, the file's change beyond its latest
/// approval as `adr show <file>` prints it, wherever the form appears, so
/// the page never offers to approve what it does not show (per "Does show
/// print what a file holds beyond its latest approval?"); `change` is false
/// only where the page shows the whole file form already. `answers` is true
/// on the page that shows each asked question's options (`/next`): an
/// answer move gets its form there, and elsewhere a link to it, so no
/// answer is one click away from a page that does not show the options.
fn with_form(line: &Line, r: &Records, token: &str, change: bool, answers: bool) -> String {
    let mut s = String::new();
    if let (true, Some(path)) = (change, approve_path(line)) {
        if let Ok(lines) = show::file_form(r, &path, Views::default()) {
            let shown: Vec<Line> = lines.into_iter().filter(|l| !matches!(l, Line::Move(_) | Line::Finding(_))).collect();
            s.push_str(&format!("<div class=\"change\">\n{}</div>\n", plain(&shown)));
        }
    }
    s.push_str(&format!("{}\n", out::html(line)));
    if let Some(path) = approve_path(line) {
        s.push_str(&format!(
            "<form method=\"post\" action=\"/approve\" class=\"act\"><input type=\"hidden\" name=\"file\" value=\"{}\"><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Approve</button></form>\n",
            esc(&path),
            esc(token)
        ));
    }
    if let Line::Move(Move { kind: MoveKind::Answer { id, version, .. }, .. }) = line {
        if answers {
            s.push_str(&answer_form(r, id, version, token));
        } else {
            // A link, not a line: the page's lines are the terminal's.
            s.push_str("<div class=\"act\"><a href=\"/next\">answer it on /next</a></div>\n");
        }
    }
    s
}

/// The path an owner's move approves: an approve, or a removed topic file
/// nothing holds.
fn approve_path(line: &Line) -> Option<String> {
    match line {
        Line::Move(Move { kind: MoveKind::Approve { path }, .. }) => Some(path.clone()),
        _ => None,
    }
}

/// The form that answers the asked question `id`: one choice per letter of
/// its options and the refusal, the version the page shows carried with it
/// (per "Does an answer carry the version of the question the owner read?").
fn answer_form(r: &Records, id: &str, version: &str, token: &str) -> String {
    let letters: Vec<char> = r
        .topic_files
        .iter()
        .flat_map(|rf| rf.sections.iter())
        .find(|s| s.id.as_deref() == Some(id))
        .map(|s| s.options.iter().map(|o| o.letter).collect())
        .unwrap_or_default();
    let mut s = format!(
        "<form method=\"post\" action=\"/answer\" class=\"act\"><input type=\"hidden\" name=\"id\" value=\"{}\"><input type=\"hidden\" name=\"v\" value=\"{}\"><input type=\"hidden\" name=\"t\" value=\"{}\">",
        esc(id),
        esc(version),
        esc(token)
    );
    // No letter is picked for the owner: the form posts only the one
    // chosen.
    for l in &letters {
        s.push_str(&format!(
            "<label class=\"letter\"><input type=\"radio\" name=\"answer\" value=\"{l}\" required> {l}</label> "
        ));
    }
    // The refuse button comes after the letters: its `answer=refuse` is the
    // last field of that name the form posts, and the one the page reads.
    s.push_str("<button type=\"submit\">Answer</button> <button type=\"submit\" name=\"answer\" value=\"refuse\" formnovalidate>Refuse</button></form>\n");
    s
}

/// The file form cut into what comes before the first question, one block
/// per status line from there on (a question, or a question struck), and
/// the findings and the moves at its end.
fn split(lines: Vec<Line>) -> (Vec<Line>, Vec<Vec<Line>>, Vec<Line>, Vec<Move>) {
    let mut head = Vec::new();
    let mut blocks: Vec<Vec<Line>> = Vec::new();
    let mut findings = Vec::new();
    let mut moves = Vec::new();
    for l in lines {
        match l {
            Line::Move(m) => moves.push(m),
            f @ Line::Finding(_) => findings.push(f),
            Line::Status { status: Status::Question { .. }, .. } => blocks.push(vec![l]),
            Line::Status { .. } if !blocks.is_empty() => blocks.push(vec![l]),
            l => match blocks.last_mut() {
                Some(b) => b.push(l),
                None => head.push(l),
            },
        }
    }
    (head, blocks, findings, moves)
}

/// A question put to the owner: asked, or asked and changed since (shown
/// with the move that asks it again).
fn is_asked(line: &Line) -> bool {
    matches!(
        line,
        Line::Status { status: Status::Question { status: QuestionStatus::Asked | QuestionStatus::ChangedSinceAsked, .. }, .. }
    )
}

fn status_subject(line: &Line) -> &str {
    match line {
        Line::Status { subject, .. } => subject,
        _ => "",
    }
}

/// The question a move on one asked question names.
fn move_heading(m: &Move) -> Option<&str> {
    match &m.kind {
        MoveKind::Answer { heading, .. } | MoveKind::AnswerWaits { heading, .. } => Some(heading),
        _ => None,
    }
}
