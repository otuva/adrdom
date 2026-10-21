//! Approve of a topic file, a blame file or a removed file, and the helpers the record approval shares.
use super::approve_record::do_approve_record;
use super::approve_terms::do_approve_terms;
use super::{finding, findings_of, stop, Outcome, Stops};
use crate::blame;
use crate::depends::{self, DepTarget};
use crate::ledger::{self, Ledger};
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::rules::{
    self, empty_plan_hash, is_topic_file_path, removal_holds, Class, Command, RemovalHolds,
};
use crate::state;
use crate::terms;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;

/// `entry` is the name of the one entry of the terms file to approve (per
/// "What does approve take for a terms file holding several entries that
/// wait?", da0dc8b1 A); approve of any other file takes none.
pub fn do_approve(root: &Path, file: &Path, entry: Option<&str>) -> Result<Outcome, Stops> {
    let ledger_path = root.join(crate::repo::LEDGER_PATH);
    let led = ledger::read(&ledger_path);
    // The path as the repository names it, read from where it was given
    // (relative to the directory the command ran in): approve takes the
    // files `show` takes and no other, so it never writes a ledger row for
    // a path no other command reads (per "Does show print what a file holds
    // past its latest approval?", which has show take every file approve
    // takes).
    let rel = approved_rel(root, file)?;
    let file = &root.join(&rel);

    if rel == crate::repo::TERMS_PATH {
        return do_approve_terms(root, file, &led, &ledger_path, entry);
    }
    if entry.is_some() {
        return Err(finding(
            Class::Terms,
            msg::approve_entry_on_other_file(&rel),
        ));
    }
    if rules::is_blame_file_path(&rel) {
        do_approve_blame(root, file, &rel, &led, &ledger_path)
    } else if is_topic_file_path(&rel) && !file.exists() {
        do_approve_removal(root, &rel, &led, &ledger_path)
    } else {
        do_approve_record(root, file, &rel, &led, &ledger_path)
    }
}

/// `file` as a path from the repository root, read the way the shell that
/// ran the command reads it (relative to the current directory unless
/// absolute, `.` and `..` resolved), or the refusal when it is outside the
/// repository or no file approve takes: a topic file under
/// adrdom/decisions, adrdom/terms.md, or a blame file under adrdom/blame.
fn approved_rel(root: &Path, file: &Path) -> Result<String, Stops> {
    let Some(rel) = state::given_rel(root, file) else {
        return Err(stop(
            Command::Approve,
            msg::approve_not_a_file_approve_takes(&file.display().to_string()),
        ));
    };
    if rules::approve_takes(&rel) {
        Ok(rel)
    } else {
        Err(stop(
            Command::Approve,
            msg::approve_not_a_file_approve_takes(&rel),
        ))
    }
}

/// `adr approve <path>` for a topic file that is gone from disk: approves
/// its removal. Refused while the file held a current decision or an open
/// question (asked and not answered, or a heading of its approved plan not
/// yet asked; see `removal_holds`), and for a path the ledger holds no
/// approved plan for: a draft never approved leaves with no owner act.
/// Writes the approved and context rows over an empty plan and empty prose,
/// and a withdrawn row for each refused id left in no file.
fn do_approve_removal(
    root: &Path,
    rel: &str,
    led: &Ledger,
    ledger_path: &Path,
) -> Result<Outcome, Stops> {
    if led.latest_for_path("approved", rel).is_none() {
        return Err(stop(Command::Approve, msg::removal_not_a_topic_file(rel)));
    }
    let all_records: Vec<RecordFile> = record::load_all(root);
    let present_ids: HashSet<String> = rules::held_ids(&all_records)
        .into_iter()
        .map(str::to_string)
        .collect();
    let RemovalHolds { held, .. } = removal_holds(root, rel, led, &present_ids);

    let refusals = findings_of(rules::judge_removed_topic_file(
        root,
        rel,
        led,
        &present_ids,
    ));
    if !refusals.is_empty() {
        return Err(refusals);
    }
    let mut withdrawn_rows: Vec<ledger::Row> = Vec::new();
    let write_time = ledger::write_time(led);
    for (id, _) in &held {
        if let Some(r) = led.latest_state(id).filter(|r| r.event == "refused") {
            withdrawn_rows.push(ledger::Row {
                time: write_time.clone(),
                event: "withdrawn".to_string(),
                id: id.clone(),
                hash: "-".to_string(),
                detail: r.detail.clone(),
                version: crate::version::VERSION.to_string(),
            });
        }
    }

    let empty = empty_plan_hash();
    let mut rows: Vec<ledger::Row> = Vec::new();
    for event in ["approved", "context"] {
        if led.latest_for_path(event, rel).map(|r| r.hash.as_str()) != Some(empty.as_str()) {
            rows.push(ledger::Row {
                time: write_time.clone(),
                event: event.to_string(),
                id: "-".to_string(),
                hash: empty.clone(),
                detail: rel.to_string(),
                version: crate::version::VERSION.to_string(),
            });
        }
    }
    rows.extend(withdrawn_rows);
    ledger::append(ledger_path, &rows).map_err(|e| {
        stop(
            Command::Approve,
            msg::update_failed(
                "ledger",
                e,
                msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
            ),
        )
    })?;
    Ok(Outcome::of_rows(&rows, &HashMap::new()))
}

/// `content` with `id_line` added as its third line, every other byte kept
/// and the new line ended as the second line is (`\r\n` or `\n`). `None`
/// when `content` has no second line ending to insert after.
fn insert_id_line(content: &str, id_line: &str) -> Option<String> {
    let first = content.find('\n')?;
    let second = first + 1 + content[first + 1..].find('\n')?;
    let eol = if content[..second].ends_with('\r') {
        "\r\n"
    } else {
        "\n"
    };
    let at = second + 1;
    Some(format!(
        "{}{}{}{}",
        &content[..at],
        id_line,
        eol,
        &content[at..]
    ))
}

/// `adrdom/blame/<name>.md` (per "Does the owner approve a blame file?",
/// e879fe72 A): parse the fixed shape, read the origin's content at the blob
/// the file names, check that the rows' ranges cover it with no gap (5c399a14
/// A) -- collecting every refusal at once, nothing written to disk on any of
/// them, the same contract `do_approve_record` keeps -- then, for a file with
/// no id line, write the identifier the tool assigns as line 3 (d2851ea1 A;
/// no row is rewritten), and write one `blame` ledger row over the hash of
/// the file as written unless it already matches the latest one for this
/// path.
fn do_approve_blame(
    root: &Path,
    file: &Path,
    rel: &str,
    led: &Ledger,
    ledger_path: &Path,
) -> Result<Outcome, Stops> {
    let content = std::fs::read_to_string(file).map_err(|e| {
        stop(
            Command::Approve,
            msg::read_failed(
                rel,
                e,
                msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
            ),
        )
    })?;
    let bf = blame::parse(
        file.to_path_buf(),
        rel.to_string(),
        record::split_lines(&content),
    );

    // One judgement of the file against its origin, the one `check` and
    // `show` read (`blame::file_state`, `rules::judge_blame_file`): approve
    // refuses on what they report, with the same moves (per "Does a command
    // refuse what check passes?", fe14991d A).
    let source = repo::Source::WorkingTree;
    let status = crate::status::blame_file_status(led, rel, &content);
    let (index, _) = blame::Index::read(root, &source, led);
    let st = blame::file_state(root, &source, &bf, status, &index);
    let refusals = findings_of(rules::judge_blame_file(&st, &bf));
    if !refusals.is_empty() {
        return Err(refusals);
    }
    // An origin that is there and cannot be read or hashed holds the file
    // from approval, as it holds `check`.
    if !st.stops.is_empty() {
        return Err(st
            .stops
            .iter()
            .flat_map(|m| stop(Command::Approve, m))
            .collect());
    }
    // The origin is gone and the file is approved (`file_state` names it
    // while the file awaits approval): the file stands at its approval, and
    // there is nothing to approve.
    if st.origin_content.is_none() {
        return Ok(Outcome::of_rows(&[], &HashMap::new()));
    }

    // The identifier goes where the blank line after the header stands; a
    // file without shape faults has that blank line as its line 3.
    let mut work_lines = bf.lines.clone();
    let minted = bf.id.is_none();
    let mut file_id = bf.id.clone().unwrap_or_default();
    let mut written: Option<String> = None;
    if minted {
        file_id = Uuid::new_v4().to_string();
        let id_line = format!("id: {}", file_id);
        work_lines.insert(2, id_line.clone());
        // Only the id line is added to the bytes on disk, with the line
        // ending of the line above it (d2851ea1 A).
        written = Some(insert_id_line(&content, &id_line).ok_or_else(|| {
            stop(
                Command::Approve,
                msg::write_failed(
                    &file.display().to_string(),
                    "no second line to insert the id line after",
                    msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
                ),
            )
        })?);
    }

    let new_hash = blame::file_hash(&work_lines);
    let unchanged = led.latest_blame(rel).map(|r| r.hash.as_str()) == Some(new_hash.as_str());

    // No row, no line: a hash the ledger already holds is no event. The row
    // is appended before the id line is written: a death between the two
    // leaves a row whose id line is missing, which check names precisely.
    let mut rows: Vec<ledger::Row> = Vec::new();
    if !unchanged {
        let row = ledger::Row {
            time: ledger::write_time(led),
            event: "blame".to_string(),
            id: file_id,
            hash: new_hash,
            detail: rel.to_string(),
            version: crate::version::VERSION.to_string(),
        };
        ledger::append(ledger_path, std::slice::from_ref(&row)).map_err(|e| {
            stop(
                Command::Approve,
                msg::update_failed(
                    "ledger",
                    e,
                    msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
                ),
            )
        })?;
        rows.push(row);
    }
    if let Some(written) = written {
        std::fs::write(&bf.path, written).map_err(|e| {
            stop(
                Command::Approve,
                msg::write_failed(
                    &file.display().to_string(),
                    e,
                    msg::after_fix_owner_runs(crate::moves::command(&["adr", "approve", &*rel])),
                ),
            )
        })?;
    }

    Ok(Outcome::of_rows(&rows, &HashMap::new()))
}

/// Inserts a freshly minted id directly after each skeleton section's
/// `depends:` line (or its `supersedes:`, `unrelated:` or `blame:` line,
/// directly under it) (per "What does a reference in a topic file carry?"; no blank padding, the
/// same bare single-line insert `approve` uses for `terms:` after `id:`). Every section here has
/// already passed `record_refusals`'s "has no depends line" check, so the search below should never
/// fail -- but that invariant lives in a different function now, so a future reorder of the two
/// reports an error instead of panicking.
pub(super) fn insert_missing_ids(
    rf: &RecordFile,
    mut work_lines: Vec<String>,
) -> Result<Vec<String>, Stops> {
    let needs_id: Vec<bool> = rf.sections.iter().map(|s| s.id.is_none()).collect();
    for i in (0..rf.sections.len()).rev() {
        if needs_id[i] {
            let sec = &rf.sections[i];
            let depends_idx = (sec.start + 1..sec.end)
                .find(|&j| work_lines[j].starts_with("depends: "))
                .ok_or_else(|| {
                    finding(Class::Plan, msg::no_depends_line_bare(&sec.heading_text))
                })?;
            // The plan lines come first, then the id: after the
            // `supersedes:`, `unrelated:` and `blame:` lines when the heading
            // has them (directly under `depends:`), else after `depends:`.
            let mut after = depends_idx;
            while work_lines.get(after + 1).is_some_and(|l| {
                l.starts_with("supersedes: ")
                    || l.starts_with("unrelated: ")
                    || l.starts_with("blame: ")
            }) {
                after += 1;
            }
            let new_id = Uuid::new_v4().to_string();
            work_lines.insert(after + 1, format!("id: {}", new_id));
        }
    }
    Ok(work_lines)
}

/// The final text one `depends:` entry rewrites to when `approve` rewrites
/// a section's single `depends:` line (per "Does a reference line hold its
/// whole list on one line?"): an already-resolved
/// `Id` entry keeps its id and set exactly as written; a `Text` entry
/// resolves to the id its target already carries in `work_rf` (every entry
/// was already validated above, so this should always find one), keeping
/// its own written text as a fallback rather than silently dropping the
/// entry on the one resolution rule this can't see (a dangling/cycle/
/// self-dep heading reference, reported once from `rf.derive_error` instead
/// of per entry).
pub(super) fn rewrite_dep_entry(
    d: &record::Dep,
    sec: &record::Section,
    work_rf: &RecordFile,
    terms_list: &[terms::Term],
    phrase_lookup: &HashMap<String, String>,
    led: &Ledger,
) -> String {
    match d {
        record::Dep::Id { id, set } => format!("{}{}", id, record::set_suffix(set)),
        record::Dep::Text { text, set } => {
            let resolved = depends::resolve_dep(d, sec, work_rf, terms_list, phrase_lookup, led)
                .ok()
                .flatten()
                .and_then(|target| match target {
                    DepTarget::Heading(idx) => work_rf.sections[idx].id.clone(),
                    DepTarget::Term(id) | DepTarget::Decision(id) | DepTarget::Struck(id) => {
                        Some(id)
                    }
                });
            match resolved {
                Some(id) => format!("{}{}", id, record::set_suffix(set)),
                None => format!("{}{}", text, record::set_suffix(set)),
            }
        }
    }
}
