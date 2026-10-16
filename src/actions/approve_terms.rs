//! Approve of the terms file.
use super::{finding, findings_of, stop, Outcome, Stops};
use crate::ledger::{self, Ledger};
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::rules::{terms_refusals, Class, Command, Finding};
use crate::state;
use crate::terms;
use std::collections::HashMap;
use std::path::Path;

/// `adrdom/terms.md` (per "Does the owner approve the terms file as a
/// whole?"): the six steps below, in order, nothing written to disk before
/// step 6 (a shape/cycle/dependent/banned refusal leaves the file
/// byte-identical to what was on disk before the call). Steps 1, 3, 4 and 5
/// (plus the definition-length check above them) are `terms_refusals`,
/// shared with `check`.
pub(super) fn do_approve_terms(
    root: &Path,
    file: &Path,
    led: &Ledger,
    ledger_path: &Path,
    entry: Option<&str>,
) -> Result<Outcome, Stops> {
    // The command that runs this approve again, named after a failure.
    let again = || {
        entry
            .map(msg::approve_entry_cmd)
            .unwrap_or_else(|| crate::moves::Cmd::fixed("adr approve adrdom/terms.md"))
    };
    // Step 1: parse.
    let mut tf = terms::load(file).map_err(|e| {
        stop(
            Command::Approve,
            msg::read_failed(
                &file.display().to_string(),
                e,
                msg::after_fix_owner_runs(again()),
            ),
        )
    })?;

    // Every topic file, parsed once here and reused by `terms_refusals`
    // (no re-reading them from disk itself).
    let records: Vec<RecordFile> = record::load_all(root);
    // Files whose plan is approved, judged on the files as written (before
    // any normalization).
    let approved_plans: std::collections::HashSet<String> = records
        .iter()
        .filter(|rf| state::plan_approved(led, rf, &rf.rel))
        .map(|rf| rf.rel.clone())
        .collect();

    // Step 2: mint ids in memory only; nothing is written to disk yet.
    terms::mint_ids(&mut tf);

    // Steps 1 (shape, post-mint), 3 (cycles), 4 (dependents), 5 (banned):
    // every refusal, all at once.
    let (refusals, pairs_per_entry) = terms_refusals(
        root,
        &records,
        &tf,
        led,
        &repo::Source::WorkingTree,
        true,
        &approved_plans,
    );
    let mut refusals = refusals;
    // Only an approve that retires or removes a term needs to read the
    // asked questions' terms lines; one that only adds or rewords does not.
    if state::approve_retires_or_removes(led, &tf) {
        for (rel, heading) in state::unreadable_asked_terms_lines(&records, led) {
            refusals.push(Finding::said(
                Class::Terms,
                msg::asked_terms_line_unreadable(&rel, &heading),
            ));
        }
    }
    if !refusals.is_empty() {
        return Err(findings_of(refusals));
    }

    // Per "What does approve take for a terms file holding several entries
    // that wait?" (da0dc8b1 A): the name of one entry that waits, new,
    // changed or removed, in any order; with no name, only a file none of
    // whose entries waits (it differs in order or blank lines alone).
    let waiting: Vec<String> = state::waiting_terms(led, &tf)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    match entry {
        None if !waiting.is_empty() => {
            return Err(finding(
                Class::Terms,
                msg::approve_terms_names_no_entry(&waiting),
            ));
        }
        Some(name) if !waiting.iter().any(|w| w == name) => {
            return Err(finding(
                Class::Terms,
                msg::approve_terms_entry_not_waiting(name, &waiting),
            ));
        }
        _ => {}
    }
    let named = |name: &str| entry.is_none_or(|e| e == name);
    // The whole file is approved once no entry waits after this run.
    let file_settles = waiting.iter().all(|w| entry == Some(w.as_str()));

    // Which terms need a new `term` row: new ones, and those changed since
    // their latest row; of a named approve, the named entry alone.
    let term_rows_due: Vec<(String, String, String)> = tf
        .terms
        .iter()
        .filter(|t| named(&t.name))
        .filter_map(|t| {
            t.id.clone()
                .map(|id| (id, t.name.clone(), terms::term_hash(t)))
        })
        .filter(|(id, _, hash)| {
            led.latest_for_id("term", id)
                .is_none_or(|r| r.hash != *hash)
        })
        .collect();
    let removed_rows_due: Vec<(String, String)> = state::removed_terms(led, &tf)
        .into_iter()
        .filter(|(_, name)| named(name))
        .collect();

    // Step 6: write the file. `terms:` lines use the fresh pairs computed
    // above, one term at a time, in the same order as `tf.terms`.
    let mut new_lines = tf.lines.clone();
    terms::write_terms_lines(&mut new_lines, &tf, &pairs_per_entry);
    tf.lines = new_lines;

    // The files as they stand, put back when the ledger refuses the rows:
    // never a terms file or terms brief rewritten for rows no ledger holds.
    let terms_before = std::fs::read_to_string(&tf.path).ok();
    let terms_changed = terms_before.as_deref() != Some(tf.lines.join("\n").as_str());
    terms::save(&tf).map_err(|e| {
        stop(
            Command::Approve,
            msg::write_failed(
                &file.display().to_string(),
                e,
                msg::after_fix_owner_runs(again()),
            ),
        )
    })?;

    // Per "Does approve write the terms brief?" (fcab4bf9 A): written on
    // every run that leaves the terms file approved, whether or not the
    // file changed, so a repository whose terms file is approved gets its
    // terms brief from one run. A named approve that leaves other entries
    // waiting writes no brief: the file is not approved yet, and a brief
    // written from it would hold definitions the owner has not approved
    // (the problem fcab4bf9 settles); the brief of the last approval stays.
    let brief_path = root.join(crate::repo::TERMS_BRIEF_PATH);
    let brief_before = std::fs::read_to_string(&brief_path).ok();
    let brief_changed =
        file_settles && brief_before.as_deref() != Some(terms::brief_text(&tf).as_str());
    let write_brief = || -> std::io::Result<()> {
        if let Some(dir) = brief_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&brief_path, terms::brief_text(&tf))
    };
    if file_settles {
        write_brief().map_err(|e| {
            stop(
                Command::Approve,
                msg::write_failed(
                    crate::repo::TERMS_BRIEF_PATH,
                    e,
                    msg::after_fix_owner_runs(again()),
                ),
            )
        })?;
    }

    let write_time = ledger::write_time(led);
    let term_rows: Vec<ledger::Row> = term_rows_due
        .iter()
        .map(|(id, name, hash)| ledger::Row {
            time: write_time.clone(),
            event: "term".to_string(),
            id: id.clone(),
            hash: hash.clone(),
            detail: name.clone(),
            version: crate::version::VERSION.to_string(),
        })
        .collect();
    let append_failed = |e: std::io::Error| {
        stop(
            Command::Approve,
            msg::update_failed("ledger", e, msg::after_fix_owner_runs(again())),
        )
    };

    // The file already matching the ledger's latest `terms` row writes
    // nothing. A ledger from before `term` rows existed gets them at its
    // next real approval (no migration code in the tool). The file's own
    // `terms` row is written once no entry waits: until then ask, answer
    // and check stop on it (per "Does the tool stop while the terms file
    // waits for approval?", aa475aa3 A).
    let mut written: Vec<ledger::Row> = Vec::new();
    if !state::terms_approved(led, &tf) {
        let mut rows = Vec::new();
        if file_settles {
            rows.push(ledger::Row {
                time: write_time.clone(),
                event: "terms".to_string(),
                id: "-".to_string(),
                hash: terms::file_hash(&tf),
                detail: crate::repo::TERMS_PATH.to_string(),
                version: crate::version::VERSION.to_string(),
            });
        }
        rows.extend(term_rows);
        for (id, name) in removed_rows_due {
            rows.push(ledger::Row {
                time: write_time.clone(),
                event: "term".to_string(),
                id,
                hash: state::TERM_REMOVED.to_string(),
                detail: name,
                version: crate::version::VERSION.to_string(),
            });
        }
        if let Err(e) = ledger::append(ledger_path, &rows) {
            if let Some(before) = &terms_before {
                let _ = std::fs::write(&tf.path, before);
            }
            if file_settles {
                match &brief_before {
                    Some(before) => {
                        let _ = std::fs::write(&brief_path, before);
                    }
                    None => {
                        let _ = std::fs::remove_file(&brief_path);
                    }
                }
            }
            return Err(append_failed(e));
        }
        written = rows;
    }

    // The terms brief is a file of its own that no row covers: written
    // whenever its text changed (per "What does a command that writes
    // print?", 069a6b4a A, one line per event it wrote).
    let mut outcome = Outcome::of_rows(&written, &HashMap::new());
    if terms_changed && written.is_empty() {
        outcome.events.push(crate::out::Event::FileWritten {
            path: crate::repo::TERMS_PATH.to_string(),
        });
    }
    if brief_changed {
        outcome.events.push(crate::out::Event::FileWritten {
            path: crate::repo::TERMS_BRIEF_PATH.to_string(),
        });
    }
    Ok(outcome)
}
