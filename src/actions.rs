//! One code path for every owner act: the CLI and the page both call these (7.2 A).
use crate::ledger::{self, Ledger};
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state::{self, TermsLoad};
use crate::terms;
use std::collections::{BTreeSet, HashSet};
use std::path::Path;

pub struct Outcome {
    pub summary: String,
}

pub fn do_lock(root: &Path, id: &str, letter: char) -> Result<Outcome, String> {
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);

    let files = repo::list_decision_files(root);
    let mut found = None;
    for f in &files {
        if let Ok(rf) = record::load(f) {
            if let Some(idx) = rf.sections.iter().position(|s| s.id.as_deref() == Some(id)) {
                found = Some((rf, idx));
                break;
            }
        }
    }
    let Some((mut rf, idx)) = found else {
        return Err(format!("id {} not found in any record", id));
    };
    let rel = state::rel_path(root, &rf.path);

    // Refuse when terms.md is missing, has shape errors, or is not
    // approved: the same messages `ask` uses, prefixed `lock:` by the
    // caller in `commands::lock::run`.
    match state::load_terms_checked(root) {
        TermsLoad::Missing => {
            return Err(
                "adrdom/terms.md missing; agent writes it, owner runs: adr approve adrdom/terms.md"
                    .to_string(),
            );
        }
        TermsLoad::LoadError(e) => {
            return Err(format!("adrdom/terms.md: {}", e));
        }
        TermsLoad::Ok(tf) => {
            if !tf.errors.is_empty() {
                let msg = tf
                    .errors
                    .iter()
                    .map(|e| format!("adrdom/terms.md: {}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(msg);
            }
            if !state::terms_approved(&led, &tf) {
                return Err(format!(
                    "{}: terms not approved; owner runs: adr approve adrdom/terms.md",
                    rel
                ));
            }
        }
    }

    let cur_hash;
    let supersedes_val;
    {
        let sec = &rf.sections[idx];
        if sec.answer_line.is_some() {
            return Err(format!("\"{}\" is already answered", sec.heading_text));
        }
        if !sec.options.iter().any(|o| o.letter == letter) {
            return Err(format!(
                "letter {} does not name an option in \"{}\"",
                letter, sec.heading_text
            ));
        }
        cur_hash = record::section_hash(&rf.lines, sec);
        match led.latest_state(id) {
            Some(row) if row.event == "asked" && row.hash == cur_hash => {}
            _ => {
                return Err(format!(
                    "\"{}\" is not currently in the 'asked' state at its current hash",
                    sec.heading_text
                ));
            }
        }
        if sec.terms_line.is_none() {
            return Err(format!("{}: no terms line; agent re-asks", id));
        }
        supersedes_val = sec.supersedes.clone();
        for old in &supersedes_val {
            // In force, or already superseded by an earlier lock: a later
            // lock naming the same older id is accepted as lineage.
            match led.latest_state(old) {
                Some(row) if row.event == "answered" || row.event == "superseded" => {}
                _ => {
                    return Err(format!(
                        "supersedes target {} is not in force and was never superseded",
                        old
                    ));
                }
            }
        }
    }

    let heading_for_print;
    let sentence_for_print;
    {
        let sec = &rf.sections[idx];
        let (_, hash_end) = record::hash_range(&rf.lines, sec);
        heading_for_print = sec.heading_text.clone();
        sentence_for_print = sec
            .options
            .iter()
            .find(|o| o.letter == letter)
            .and_then(|o| o.sentence.clone())
            .unwrap_or_default();
        rf.lines.splice(
            hash_end..hash_end,
            [String::new(), format!("**Answer:** {}", letter)],
        );
    }

    record::save(&rf).map_err(|e| format!("failed to write record: {}", e))?;

    let mut rows = vec![ledger::Row {
        time: ledger::now_rfc3339(),
        event: "answered".to_string(),
        id: id.to_string(),
        hash: cur_hash,
        detail: letter.to_string(),
    }];
    for old in &supersedes_val {
        rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "superseded".to_string(),
            id: old.clone(),
            hash: "-".to_string(),
            detail: id.to_string(),
        });
    }
    ledger::append(&ledger_path, &rows).map_err(|e| format!("failed to update ledger: {}", e))?;

    Ok(Outcome {
        summary: format!("locked: {}\n  {}", heading_for_print, sentence_for_print),
    })
}

pub fn do_approve(root: &Path, file: &Path) -> Result<Outcome, String> {
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);
    let rel = state::rel_path(root, file);

    if rel == "adrdom/terms.md" {
        do_approve_terms(root, file, &led, &ledger_path)
    } else {
        do_approve_record(root, file, &rel, &led, &ledger_path)
    }
}

/// One dependent's pairs (`word=uuid`), together with a description of
/// where they were carried, for approve's step 4.
struct Dependent {
    /// `<rel>: "<heading>"` for a decision, or `entry "<name>"` for an
    /// entry's own (stale, machine-written) terms line.
    source: String,
    pairs: Vec<(String, String)>,
}

/// `adrdom/terms.md`: the seven steps of 4 in the spec, in order, nothing
/// written to disk before step 6 (a shape/cycle/dependent/banned refusal
/// leaves the file byte-identical to what was on disk before the call).
fn do_approve_terms(
    root: &Path,
    file: &Path,
    led: &Ledger,
    ledger_path: &Path,
) -> Result<Outcome, String> {
    // Step 1: parse.
    let mut tf =
        terms::load(file).map_err(|e| format!("approve: cannot read {}: {}", file.display(), e))?;

    // The pre-existing latest `terms` row, captured before anything below
    // can change what "latest" means, for step 7's before/after compare.
    let prev_terms_row = led.latest_for_path("terms", "adrdom/terms.md").cloned();

    // Step 2: mint ids in memory only; nothing is written to disk yet.
    let inserted = terms::mint_ids(&mut tf);

    if !tf.errors.is_empty() {
        let msg = tf
            .errors
            .iter()
            .map(|e| format!("approve: adrdom/terms.md: {}; owner fixes adrdom/terms.md", e))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(msg);
    }

    // Step 3: definition pairings; a self-pairing or any cycle refuses.
    let cycles = terms::cycles(&tf);
    if !cycles.is_empty() {
        let msg = cycles
            .iter()
            .map(|cycle| {
                let mut chain = cycle.clone();
                chain.push(cycle[0].clone());
                format!(
                    "approve: adrdom/terms.md: circular definition: {}; agent defines one of them from plainer words",
                    chain.join(" -> ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Err(msg);
    }

    // Every in-force decision, and every non-retired entry's own
    // (pre-existing, stale-until-step-6) terms line, is a dependent.
    let files = repo::list_decision_files(root);
    let mut loaded: Vec<(String, RecordFile)> = Vec::new();
    for f in &files {
        let Ok(rf) = record::load(f) else { continue };
        loaded.push((state::rel_path(root, f), rf));
    }

    let mut dependents: Vec<Dependent> = Vec::new();
    for (frel, rf) in &loaded {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_in_force(led, id) {
                continue;
            }
            let Some(tl) = sec.terms_line else { continue };
            let Some(pairs) = terms::parse_terms_line(rf.lines[tl].trim_end()) else {
                continue;
            };
            dependents.push(Dependent {
                source: format!("{}: \"{}\"", frel, sec.heading_text),
                pairs,
            });
        }
    }
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        let Some(line) = &t.terms_line else { continue };
        let Some(pairs) = terms::parse_terms_line(line) else {
            continue;
        };
        dependents.push(Dependent {
            source: format!("entry \"{}\"", t.name),
            pairs,
        });
    }

    // Step 4: every pair on a dependent must still hold in the new file.
    let mut refusals: Vec<String> = Vec::new();
    for dep in &dependents {
        for (word, uuid) in &dep.pairs {
            let Some(target) = tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str()))
            else {
                refusals.push(format!(
                    "approve: adrdom/terms.md: entry {} removed but carried by {}; owner retires instead",
                    uuid, dep.source
                ));
                continue;
            };
            if target.retired {
                refusals.push(format!(
                    "approve: adrdom/terms.md: entry \"{}\" retired but carried by {}; owner supersedes first",
                    target.name, dep.source
                ));
                continue;
            }
            let still_resolves = terms::resolve(word, &tf.terms)
                .iter()
                .any(|(w, id)| w == word && id == uuid);
            if !still_resolves {
                refusals.push(format!(
                    "approve: adrdom/terms.md: word \"{}\" moved off entry \"{}\", carried by {}",
                    word, target.name, dep.source
                ));
            }
        }
    }
    if !refusals.is_empty() {
        return Err(refusals.join("\n"));
    }

    // Step 5: banned hits in terms.md's own text, from the single ban list.
    let banned_path = root.join("adrdom/state/banned.sha256");
    let banned_entries = crate::banned::read_entries(&banned_path);
    if !banned_entries.is_empty() {
        let hash_map: std::collections::HashMap<String, Option<String>> =
            banned_entries.into_iter().collect();
        let mut banned_refusals: Vec<String> = Vec::new();
        for (i, line) in tf.lines.iter().enumerate() {
            for tok in crate::banned::tokenize(line) {
                let h = record::sha256_hex(&tok);
                if let Some(uuid_opt) = hash_map.get(&h) {
                    let suffix = uuid_opt
                        .as_ref()
                        .and_then(|u| {
                            tf.terms
                                .iter()
                                .find(|t| t.id.as_deref() == Some(u.as_str()))
                                .map(|t| t.name.clone())
                        })
                        .map(|n| format!(" (term {})", n))
                        .unwrap_or_default();
                    banned_refusals.push(format!(
                        "approve: adrdom/terms.md:{}: banned word \"{}\"{}; owner edits adrdom/terms.md to remove it",
                        i + 1,
                        tok,
                        suffix
                    ));
                }
            }
        }
        if !banned_refusals.is_empty() {
            return Err(banned_refusals.join("\n"));
        }
    }

    // Step 6: write the file. `terms:` lines are computed fresh from each
    // entry's current definition, one entry at a time, in the same order
    // as `tf.terms`.
    let pairs_per_entry: Vec<Vec<(String, String)>> = (0..tf.terms.len())
        .map(|i| terms::definition_pairs(&tf, i))
        .collect();
    let mut new_lines = tf.lines.clone();
    terms::write_terms_lines(&mut new_lines, &tf, &pairs_per_entry);
    tf.lines = new_lines;

    terms::save(&tf).map_err(|e| format!("approve: failed to write {}: {}", file.display(), e))?;

    let mut out: Vec<String> = Vec::new();
    if inserted > 0 {
        out.push(format!(
            "approve: adrdom/terms.md: {} missing id(s) filled",
            inserted
        ));
    }

    if state::terms_approved(led, &tf) {
        out.push("approve: adrdom/terms.md: already approved".to_string());
    } else {
        let row = ledger::Row {
            time: ledger::now_rfc3339(),
            event: "terms".to_string(),
            id: "-".to_string(),
            hash: terms::file_hash(&tf),
            detail: "adrdom/terms.md".to_string(),
        };
        ledger::append(ledger_path, &[row])
            .map_err(|e| format!("approve: failed to update ledger: {}", e))?;
        out.push("approve: adrdom/terms.md: approved".to_string());
    }

    // Step 7: informational, never a refusal. Only when the version on
    // HEAD is genuinely the one just superseded (its hash matches the
    // ledger row that was latest before this call) do we have anything
    // trustworthy to diff definitions against.
    if let Some(prev_row) = &prev_terms_row {
        if let Some(head_content) = repo::git(root, &["show", "HEAD:adrdom/terms.md"]) {
            let old_tf = terms::parse(&record::split_lines(&head_content));
            if terms::file_hash(&old_tf) == prev_row.hash {
                for t in &tf.terms {
                    let Some(tid) = &t.id else { continue };
                    let Some(old_t) = old_tf.terms.iter().find(|o| o.id.as_deref() == Some(tid.as_str()))
                    else {
                        continue;
                    };
                    if old_t.definition == t.definition {
                        continue;
                    }
                    out.push(format!("approve: term \"{}\" definition changed", t.name));
                    for (frel, rf) in &loaded {
                        for sec in &rf.sections {
                            let Some(id) = &sec.id else { continue };
                            if !state::is_in_force(led, id) {
                                continue;
                            }
                            let Some(tl) = sec.terms_line else { continue };
                            let pairs =
                                terms::parse_terms_line(rf.lines[tl].trim_end()).unwrap_or_default();
                            if pairs.iter().any(|(_, pid)| pid == tid) {
                                out.push(format!("  in force: {}: {}", frel, sec.heading_text));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Outcome {
        summary: out.join("\n"),
    })
}

/// A record file: parse, refuse on shape errors; compute plan hash and
/// context hash; write an `approved` row if it differs from the latest for
/// the path, a `context` row likewise; then for every ledger id that now
/// appears in no record: withdraw an asked one, refuse on an in-force one,
/// allow a superseded one silently.
fn do_approve_record(
    root: &Path,
    file: &Path,
    rel: &str,
    led: &Ledger,
    ledger_path: &Path,
) -> Result<Outcome, String> {
    let rf =
        record::load(file).map_err(|e| format!("approve: cannot read {}: {}", file.display(), e))?;

    let mut shape_msgs: Vec<String> = rf
        .shape_errors
        .iter()
        .map(|e| format!("approve: {}: {}; owner fixes the record", rel, e))
        .collect();
    for sec in &rf.sections {
        for e in &sec.shape_errors {
            shape_msgs.push(format!(
                "approve: {}: \"{}\": {}; owner fixes the record",
                rel, sec.heading_text, e
            ));
        }
    }
    if !shape_msgs.is_empty() {
        return Err(shape_msgs.join("\n"));
    }

    let plan_hash = record::plan_hash(&rf);
    let context_hash = record::context_hash(&rf);

    let mut rows: Vec<ledger::Row> = Vec::new();
    let mut messages: Vec<String> = Vec::new();

    let plan_changed = led
        .latest_for_path("approved", rel)
        .map(|r| r.hash.as_str())
        != Some(plan_hash.as_str());
    if plan_changed {
        rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "approved".to_string(),
            id: "-".to_string(),
            hash: plan_hash.clone(),
            detail: rel.to_string(),
        });
        messages.push(format!("approve: {}: plan approved", rel));
    } else {
        messages.push(format!("approve: {}: plan unchanged", rel));
    }

    let context_changed = led
        .latest_for_path("context", rel)
        .map(|r| r.hash.as_str())
        != Some(context_hash.as_str());
    if context_changed {
        rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "context".to_string(),
            id: "-".to_string(),
            hash: context_hash.clone(),
            detail: rel.to_string(),
        });
        messages.push(format!("approve: {}: context approved", rel));
    } else {
        messages.push(format!("approve: {}: context unchanged", rel));
    }

    ledger::append(ledger_path, &rows)
        .map_err(|e| format!("approve: failed to update ledger: {}", e))?;

    // Ledger ids (asked/answered/superseded/withdrawn) that now appear in
    // no record at all: withdraw the asked ones, refuse on an in-force
    // (answered, not superseded) one, allow a superseded one silently.
    let all_files = repo::list_decision_files(root);
    let mut present_ids: HashSet<String> = HashSet::new();
    for f in &all_files {
        if let Ok(rf2) = record::load(f) {
            for sec in &rf2.sections {
                if let Some(id) = &sec.id {
                    present_ids.insert(id.clone());
                }
            }
        }
    }

    let mut state_ids: BTreeSet<String> = BTreeSet::new();
    for row in &led.rows {
        if matches!(
            row.event.as_str(),
            "asked" | "answered" | "superseded" | "withdrawn"
        ) {
            state_ids.insert(row.id.clone());
        }
    }

    let mut refusals: Vec<String> = Vec::new();
    let mut withdrawn_rows: Vec<ledger::Row> = Vec::new();
    for id in &state_ids {
        if present_ids.contains(id) {
            continue;
        }
        match led.latest_state(id) {
            Some(r) if r.event == "asked" => {
                withdrawn_rows.push(ledger::Row {
                    time: ledger::now_rfc3339(),
                    event: "withdrawn".to_string(),
                    id: id.clone(),
                    hash: "-".to_string(),
                    detail: r.detail.clone(),
                });
            }
            Some(r) if r.event == "answered" => {
                let heading = led
                    .latest_asked(id)
                    .map(|a| a.detail.clone())
                    .unwrap_or_else(|| id.clone());
                refusals.push(format!(
                    "approve: id {} (\"{}\") is answered and in force but missing from every record; owner supersedes it or restores the heading",
                    id, heading
                ));
            }
            _ => {}
        }
    }

    if !refusals.is_empty() {
        return Err(refusals.join("\n"));
    }

    if !withdrawn_rows.is_empty() {
        ledger::append(ledger_path, &withdrawn_rows)
            .map_err(|e| format!("approve: failed to update ledger: {}", e))?;
        for r in &withdrawn_rows {
            messages.push(format!("approve: withdrawn: {}", r.detail));
        }
    }

    Ok(Outcome {
        summary: messages.join("\n"),
    })
}

pub fn do_unban(root: &Path, input: &str) -> Result<Outcome, String> {
    let banned_path = root.join("adrdom/state/banned.sha256");
    let ledger_path = root.join("adrdom/state/ledger.tsv");

    let mut to_remove: HashSet<String> = HashSet::new();
    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        for tok in crate::banned::tokenize(line) {
            to_remove.insert(record::sha256_hex(&tok));
        }
    }

    let existing = crate::banned::read_entries(&banned_path);
    let mut removed: Vec<String> = existing
        .iter()
        .filter(|(h, _)| to_remove.contains(h))
        .map(|(h, _)| h.clone())
        .collect();
    removed.sort();

    if removed.is_empty() {
        return Ok(Outcome {
            summary: "nothing to remove".to_string(),
        });
    }

    let mut remaining: Vec<(String, Option<String>)> = existing
        .into_iter()
        .filter(|(h, _)| !to_remove.contains(h))
        .collect();
    remaining.sort_by(|a, b| a.0.cmp(&b.0));
    let mut content = String::new();
    for (h, u) in &remaining {
        content.push_str(&crate::banned::format_line(h, u));
        content.push('\n');
    }
    std::fs::write(&banned_path, content)
        .map_err(|e| format!("failed to update banned list: {}", e))?;

    let rows: Vec<ledger::Row> = removed
        .iter()
        .map(|h| ledger::Row {
            time: ledger::now_rfc3339(),
            event: "unbanned".to_string(),
            id: "-".to_string(),
            hash: h.clone(),
            detail: String::new(),
        })
        .collect();
    ledger::append(&ledger_path, &rows).map_err(|e| format!("failed to update ledger: {}", e))?;

    Ok(Outcome {
        summary: format!("removed {} entries", removed.len()),
    })
}
