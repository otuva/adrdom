use super::Failure;
use crate::commands::install::TIE_OPEN;
use crate::ledger::Ledger;
use crate::moves::{Move, MoveKind};
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo::{self, Source};
use crate::rules::{Class, Finding};
use crate::terms::Term;
use std::collections::HashMap;
use std::path::Path;

/// The brief asset's sentence lines against the decisions and terms they
/// state (adr-code: de5ed5b1-3a83-4403-8953-941cb7e94390). Runs only for a
/// tracked `assets/brief.md` that holds a from-comment at all, so a
/// repository with no such file, or an unrelated one, is left alone.
pub(super) fn check_brief(
    root: &Path,
    source: &Source,
    records: &[RecordFile],
    terms: &[Term],
    led: &Ledger,
    role: crate::session::Role,
) -> (Vec<Finding>, Vec<Finding>) {
    let (mut failures, mut notes) = (Vec::new(), Vec::new());
    let tracked = repo::git_paths(root, &["ls-files"]).is_some_and(|l| l.iter().any(|p| p == msg::BRIEF_ASSET));
    if !tracked {
        return (failures, notes);
    }
    let Some(text) = source.read(root, msg::BRIEF_ASSET) else { return (failures, notes) };
    if !text.contains("<!-- from:") {
        return (failures, notes);
    }

    // Where each question sits, for the heading a finding names.
    let mut headings: HashMap<&str, &str> = HashMap::new();
    for rf in records {
        for s in &rf.sections {
            if let Some(id) = &s.id {
                headings.insert(id.as_str(), s.heading_text.as_str());
            }
        }
    }
    let term_of: HashMap<&str, &Term> = terms.iter().filter_map(|t| t.id.as_deref().map(|id| (id, t))).collect();

    let write = Move::new(MoveKind::BriefTieWrite).in_session(role).clause();
    let fix = Move::new(MoveKind::BriefTieFix).in_session(role).clause();
    let mut fenced = false;
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let line = raw.trim_end();
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || line.is_empty() || line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        let Some(at) = line.rfind(TIE_OPEN) else {
            if line.contains("<!-- from:") {
                failures.push(Failure::said(Class::Brief, msg::brief_comment_malformed(n, "does not end the line", fix.clone())));
            } else {
                failures.push(Failure::said(Class::Brief, msg::brief_line_unmarked(n, write.clone())));
            }
            continue;
        };
        let comment = &line[at + TIE_OPEN.len()..];
        let Some(inner) = comment.strip_suffix("-->").filter(|c| !c.contains("-->") && !c.contains("<!--")) else {
            failures.push(Failure::said(Class::Brief, msg::brief_comment_malformed(n, "does not end the line", fix.clone())));
            continue;
        };
        let tokens: Vec<&str> = inner.split(", ").map(str::trim).collect();
        if inner.trim().is_empty() {
            failures.push(Failure::said(Class::Brief, msg::brief_comment_malformed(n, "is empty", fix.clone())));
            continue;
        }
        for tok in tokens {
            if !record::is_uuid(tok) {
                failures.push(Failure::said(Class::Brief, msg::brief_comment_unknown(n, tok, false, fix.clone())));
            } else if let Some(t) = term_of.get(tok) {
                if t.retired {
                    let mv = Move::new(MoveKind::BriefTieRetie { to: Vec::new() }).in_session(role);
                    notes.push(Finding::said(Class::Brief, msg::brief_comment_stale(n, tok, Some(&t.name), true, mv.clause())));
                }
            } else if headings.contains_key(tok) || led.latest_state(tok).is_some() {
                if !crate::state::is_current(led, tok) {
                    let heading = headings
                        .get(tok)
                        .copied()
                        .or_else(|| led.latest_asked(tok).map(|r| r.detail.as_str()).filter(|d| !d.is_empty()));
                    let to: Vec<(String, String)> = crate::state::current_successors(led, tok)
                        .into_iter()
                        .filter_map(|s| headings.get(s.as_str()).map(|h| (h.to_string(), s.clone())))
                        .collect();
                    let mv = Move::new(MoveKind::BriefTieRetie { to }).in_session(role);
                    notes.push(Finding::said(Class::Brief, msg::brief_comment_stale(n, tok, heading, false, mv.clause())));
                }
            } else {
                failures.push(Failure::said(Class::Brief, msg::brief_comment_unknown(n, tok, true, fix.clone())));
            }
        }
    }
    (failures, notes)
}
