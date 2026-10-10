//! The `/` route: one card per topic file with its status, the change an
//! approval signs, its findings and moves, and its questions; the terms
//! card; the other files; check's report; the unban form. Every line a
//! card holds is a line a command prints, rendered through `out::html`
//! (per "Does the ui show what the terminal prints?", 7857d633 A).
use super::data::{self, File, Q};
use super::html::{banner, esc, lines, neighbours, page, plural, st};
use super::percent_encode;
use crate::commands::show::{self, Records, Views};
use crate::moves::{Move, MoveKind, Mover};
use crate::out::{self, Line, Status};
use crate::record::RecordFile;
use crate::repo;
use crate::rules::Finding;
use crate::status::{FileStatus, QuestionStatus};
use std::collections::HashMap;

pub(super) fn render_index(r: &Records, last: &[Line], token: &str) -> String {
    let mut body = banner(last);
    let files = data::files(r, true);
    // Every finding the page shows, by its words: check's report below adds
    // only the ones no card holds.
    let mut shown: Vec<String> = Vec::new();

    let open = files.iter().flat_map(|f| &f.qs).filter(|q| q.listed()).count();
    let answerable = files.iter().flat_map(|f| &f.qs).any(|q| q.answerable());
    body.push_str("<header class=\"lead\">\n<h1>Overview</h1>\n");
    body.push_str(&format!(
        "<p class=\"summary\">{} across {}.</p>\n",
        plural(open, "open question", "open questions"),
        plural(files.len(), "topic", "topics")
    ));
    if answerable {
        body.push_str("<p><a class=\"primary\" href=\"/next\">Answer the next question</a></p>\n");
    }
    body.push_str("</header>\n");

    let mut unreadable = show::unreadable_topic_lines(r);
    unreadable.extend(r.terms_unreadable_finding());
    if !unreadable.is_empty() {
        shown.extend(unreadable.iter().map(out::text));
        body.push_str(&format!("<div class=\"failures\">\n{}</div>\n", lines(&unreadable)));
    }

    for f in &files {
        body.push_str(&topic_card(r, f, token, &mut shown));
    }
    if let Some(card) = terms_card(r, token, &mut shown) {
        body.push_str(&card);
    }

    let rest = show::rest(r);
    if !rest.is_empty() {
        body.push_str("<article class=\"topic\">\n<h2>Other files</h2>\n");
        for l in &rest {
            shown.push(out::text(l));
            if let Line::Move(Move { kind: MoveKind::Approve { path }, .. }) = l {
                body.push_str(&approve_block(r, path, None, "Approve", token));
            } else {
                body.push_str(&format!("{}\n", out::html(l)));
            }
        }
        body.push_str("</article>\n");
    }

    body.push_str(&check_card(r, &files, &shown));

    body.push_str(&format!(
        "<details class=\"unban\"><summary>Unban a phrase</summary>\n<form method=\"post\" action=\"/unban\"><textarea name=\"lines\" rows=\"4\" placeholder=\"One phrase per line\"></textarea><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Unban</button></form>\n</details>\n",
        esc(token)
    ));
    page("Overview", "/", &body, false)
}

fn topic_card(r: &Records, f: &File, token: &str, shown: &mut Vec<String>) -> String {
    let rf = f.rf;
    let mut s = String::from("<article class=\"topic\">\n");
    let topic = rf.topic.as_deref().filter(|t| !t.is_empty()).unwrap_or(&rf.rel);
    s.push_str(&format!("<h2>{}</h2>\n", esc(topic)));
    let status = show::topic_file_status_line(&r.led, rf);
    let file_st = crate::status::topic_file_status(&r.led, rf);
    s.push_str(&format!(
        "<div class=\"path state {}\">{}</div>\n",
        if file_st.awaits_approval() { "waiting" } else { "done" },
        st(&status, false, None)
    ));

    // What the approval signs stands above its button.
    let approve = f.report.moves.iter().find_map(|m| match &m.kind {
        MoveKind::Approve { path } => Some(path.clone()),
        _ => None,
    });
    if let Some(path) = &approve {
        s.push_str(&approve_block(r, path, Some(rf), &format!("Approve {}", topic), token));
    } else {
        // Questions of the plan with no body or not asked yet, as the file
        // form prints their status lines (a question new since the file's
        // approval carries the mark of it).
        let unasked: Vec<Line> = show::file_form_lines(r, &rf.rel, Views::default())
            .unwrap_or_default()
            .into_iter()
            .filter(|l| {
                matches!(
                    l,
                    Line::Status {
                        status: Status::Question { status: QuestionStatus::NoBody | QuestionStatus::NotAsked, .. },
                        ..
                    }
                )
            })
            .collect();
        if !unasked.is_empty() {
            s.push_str("<ul class=\"unasked-list\">\n");
            for l in &unasked {
                s.push_str(&format!("<li>{}</li>\n", st(l, false, None)));
            }
            s.push_str("</ul>\n");
        }
    }

    // The findings and moves of the file that no question below carries.
    let open_like = |q: &Q| q.listed();
    let placed = |q: &Q| open_like(q) || q.status == QuestionStatus::Refused;
    let moves = data::unplaced(f, &placed);
    let move_lines = |keep: &dyn Fn(&Move) -> bool| -> Vec<Line> {
        moves.iter().filter(|m| keep(m)).map(|m| Line::Move((*m).clone())).collect()
    };
    let builds = move_lines(&|m| matches!(m.kind, MoveKind::Build { .. }));
    let others = move_lines(&|m| !matches!(m.kind, MoveKind::Build { .. }) && m.actor() != Mover::Owner);
    let findings: Vec<Line> = f.report.findings.iter().cloned().map(Line::Finding).collect();
    shown.extend(findings.iter().map(out::text));
    s.push_str(&lines(&builds));
    if !findings.is_empty() {
        s.push_str(&format!("<div class=\"failures\">\n{}</div>\n", lines(&findings)));
    }
    s.push_str(&lines(&others));

    // Asked questions: a link where `answer` takes it, what it waits on
    // where it does not.
    let open: Vec<&Q> = f.qs.iter().filter(|q| open_like(q)).collect();
    if !open.is_empty() {
        s.push_str("<ol class=\"open-list\">\n");
        for q in open {
            s.push_str(&open_item(q, false));
        }
        s.push_str("</ol>\n");
    }
    let refused: Vec<&Q> = f.qs.iter().filter(|q| q.status == QuestionStatus::Refused).collect();
    if !refused.is_empty() {
        s.push_str("<ul class=\"refused-list\">\n");
        for q in refused {
            s.push_str(&format!(
                "<li>{}{}</li>\n",
                st(&q.line, false, None),
                notes(q)
            ));
        }
        s.push_str("</ul>\n");
    }

    // Decisions: the heading and the option picked, as `adr show` lists them.
    let answered = show::decision_lines(rf, &r.led);
    let count = answered.iter().filter(|l| matches!(l, Line::Record { depth: 1, .. })).count();
    if count > 0 {
        s.push_str(&format!(
            "<details class=\"answered\"><summary>{}</summary>\n<dl>\n",
            plural(count, "answered question", "answered questions")
        ));
        for l in &answered {
            match l {
                Line::Record { depth: 1, .. } => s.push_str(&format!("<dt>{}</dt>", out::html(l))),
                _ => s.push_str(&format!("<dd>{}</dd>\n", out::html(l))),
            }
        }
        s.push_str("</dl></details>\n");
    }
    let gone: Vec<&Q> = f
        .qs
        .iter()
        .filter(|q| matches!(q.status, QuestionStatus::Superseded | QuestionStatus::Withdrawn | QuestionStatus::Dropped))
        .collect();
    if !gone.is_empty() {
        s.push_str(&format!(
            "<details class=\"answered\"><summary>{}</summary>\n",
            plural(gone.len(), "question out of the plan", "questions out of the plan")
        ));
        for q in gone {
            s.push_str(&format!("<div>{}</div>\n", st(&q.line, false, None)));
        }
        s.push_str("</details>\n");
    }
    s.push_str("</article>\n");
    s
}

/// The moves that name a question and are not the owner's: what it waits
/// on, the agent's next step. The owner's answer is the link or the form.
pub(super) fn notes(q: &Q) -> String {
    let moves: Vec<Line> = q.moves.iter().filter(|m| m.actor() != Mover::Owner).cloned().map(Line::Move).collect();
    lines(&moves)
}

/// One asked question of a list: its status as badges, the heading as a
/// link to the answer page where `answer` takes it (the owner's answer move
/// is that link), and what it waits on where it does not.
pub(super) fn open_item(q: &Q, current: bool) -> String {
    let class = if current { " class=\"current\"" } else { "" };
    if q.answerable() {
        let href = format!("/next?show={}", percent_encode(q.id()));
        format!("<li{}>{}\n{}</li>\n", class, st(&q.line, false, Some(&href)), notes(q))
    } else {
        format!(
            "<li{}><span class=\"label waiting\">waiting</span> {}\n{}</li>\n",
            class,
            st(&q.line, false, None),
            notes(q)
        )
    }
}

fn terms_card(r: &Records, token: &str, shown: &mut Vec<String>) -> Option<String> {
    let tf = r.terms_file.as_ref()?;
    let (_, report) = show::terms_report(r, tf);
    let mut s = String::from("<article class=\"topic\">\n<h2>Terms</h2>\n");
    s.push_str(&format!("<p class=\"path\">{}</p>\n", esc(repo::TERMS_PATH)));
    let status = crate::status::terms_file_status(&r.led, tf);
    let unpaired = tf.terms.iter().filter(|t| !t.retired && show::term_dependents(r, t).is_empty()).count();
    s.push_str("<div class=\"summary-card\">");
    s.push_str(&format!("<span class=\"count\">{}</span>", plural(tf.terms.len(), "term", "terms")));
    if unpaired > 0 {
        s.push_str(&format!("<span class=\"muted\">{} paired by nothing</span>", unpaired));
    }
    s.push_str("<a class=\"more\" href=\"/terms\">Open the terms &rarr;</a></div>\n");
    let st_line = Line::Status {
        subject: repo::TERMS_PATH.to_string(),
        status: Status::File(status),
        mark: None,
    };
    s.push_str(&format!(
        "<div class=\"state {}\">{}</div>\n",
        if status.awaits_approval() { "waiting" } else { "done" },
        st(&st_line, false, None)
    ));
    for m in &report.moves {
        if let MoveKind::Approve { path } = &m.kind {
            s.push_str(&approve_block(r, path, None, "Approve terms", token));
        } else {
            s.push_str(&format!("{}\n", out::html(&Line::Move(m.clone()))));
        }
    }
    if !report.findings.is_empty() {
        shown.extend(report.findings.iter().map(|f| out::text(&Line::Finding(f.clone()))));
        let fl: Vec<Line> = report.findings.iter().cloned().map(Line::Finding).collect();
        s.push_str(&format!("<div class=\"failures\">\n{}</div>\n", lines(&fl)));
    }
    s.push_str("</article>\n");
    Some(s)
}

/// What `adr check` prints that no card above holds: failures, then notes.
fn check_card(r: &Records, files: &[File], shown: &[String]) -> String {
    let report = crate::commands::check::collect_from(r, &files.iter().map(|f| f.judged.as_slice()).collect::<Vec<_>>());
    let fresh = |fs: Vec<Finding>| -> Vec<Line> {
        fs.into_iter().map(Line::Finding).filter(|l| !shown.contains(&out::text(l))).collect()
    };
    // One group in the one order `adr check` prints: failures and notes are
    // not told apart on the terminal, so they are not on the page.
    let mut found: Vec<Line> =
        report.stops.iter().map(|m| out::refusal(crate::rules::Command::Check, m)).collect();
    found.extend(fresh(report.failures.into_iter().chain(report.notes).collect()));
    if found.is_empty() {
        return String::new();
    }
    let mut s = String::from("<article class=\"topic check\">\n<h2>Check</h2>\n");
    s.push_str(&format!("<div class=\"failures\">\n{}</div>\n", lines(&found)));
    s.push_str("</article>\n");
    s
}

/// The change beyond the latest approval of `path` as `adr show <path>`
/// prints it, the move that approves it and its button: the page never
/// offers to approve what it does not show (per "Does show print what a
/// file holds beyond its latest approval?"). `rf` is the topic file, whose
/// questions carry their neighbours under them.
pub(super) fn approve_block(r: &Records, path: &str, rf: Option<&RecordFile>, label: &str, token: &str) -> String {
    let mut s = change_block(r, path, rf);
    s.push_str(&format!(
        "<form method=\"post\" action=\"/approve\" class=\"inline\"><input type=\"hidden\" name=\"file\" value=\"{}\"><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">{}</button></form>\n",
        esc(path),
        esc(token),
        esc(label)
    ));
    s
}

/// The parts of `path` that stand beyond its latest approval, one line each:
/// a chip (`+` new, `-` struck, `changed`), the status badges and the
/// heading, the neighbours of a question folded under it, and the lines of
/// the change folded under "show the change".
fn change_block(r: &Records, path: &str, rf: Option<&RecordFile>) -> String {
    let Ok(all) = show::file_form_lines(r, path, Views::default()) else { return String::new() };
    struct Group {
        head: Option<Line>,
        records: Vec<Line>,
        /// The reader's words under the question, shown beside it.
        words: Vec<Line>,
        keep: bool,
        question: Option<String>,
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut first = true;
    // Lines before the first status line after the file's own: the file's
    // change as a whole.
    let mut cur = Group { head: None, records: Vec::new(), words: Vec::new(), keep: false, question: None };
    for l in all {
        match l {
            Line::Move(_) | Line::Finding(_) => {}
            Line::Status { ref subject, ref status, ref mark } => {
                if first {
                    // The file's own status line, shown on the card.
                    first = false;
                    continue;
                }
                groups.push(std::mem::replace(&mut cur, Group { head: None, records: Vec::new(), words: Vec::new(), keep: false, question: None }));
                cur.keep = mark.is_some()
                    || match status {
                        Status::Question { status, .. } => {
                            matches!(status, QuestionStatus::NoBody | QuestionStatus::NotAsked)
                        }
                        Status::File(s) => *s != FileStatus::Approved,
                        Status::Term(t) => {
                            !matches!(t, crate::status::TermStatus::Approved | crate::status::TermStatus::Retired)
                        }
                    };
                if let Status::Question { .. } = status {
                    cur.question = Some(subject.clone());
                }
                cur.head = Some(l.clone());
            }
            Line::Record { ref text, .. } => {
                if text.starts_with("+ ") || text.starts_with("- ") {
                    cur.keep = true;
                }
                cur.records.push(l.clone());
            }
            Line::Word { .. } => cur.words.push(l),
            other => cur.records.push(other),
        }
    }
    groups.push(cur);
    let mut s = String::new();
    // The neighbours of the file's questions, computed once, the first time
    // a changed question needs them.
    let neighbours_of_file = std::cell::OnceCell::new();
    for g in groups {
        if !g.keep || (g.head.is_none() && g.records.is_empty()) {
            continue;
        }
        s.push_str("<div class=\"chg\">");
        if let Some(h) = &g.head {
            let chip = match h {
                Line::Status { mark: Some(crate::approval::Side::Added), .. } => "+",
                Line::Status { mark: Some(crate::approval::Side::Removed), .. } => "\u{2212}",
                _ => "changed",
            };
            s.push_str(&format!("<span class=\"chip\">{}</span> {}", chip, st(h, false, None)));
        } else {
            s.push_str("<span class=\"chip\">changed</span> <span class=\"subject\">the file as a whole</span>");
        }
        s.push_str("\n");
        if let (Some(rf), Some(q)) = (rf, &g.question) {
            let map: &HashMap<String, Vec<String>> = neighbours_of_file.get_or_init(|| show::neighbours_map(r, rf));
            if let Some(n) = map.get(q).filter(|n| !n.is_empty()) {
                s.push_str(&neighbours(n));
            }
        }
        if !g.words.is_empty() {
            s.push_str(&lines(&g.words));
        }
        if !g.records.is_empty() {
            s.push_str(&format!("<details class=\"chgd\"><summary>show the change</summary>\n{}</details>\n", lines(&g.records)));
        }
        s.push_str("</div>\n");
    }
    if s.is_empty() {
        return s;
    }
    format!("<div class=\"change\">\n{}</div>\n", s)
}
