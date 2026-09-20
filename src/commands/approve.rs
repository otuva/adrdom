use crate::ledger::{self, Ledger};
use crate::record;
use crate::repo;
use crate::state;
use crate::terms;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

pub fn run(file: PathBuf) -> i32 {
    if !repo::owner_authorized() {
        eprintln!(
            "approve: refusing: owner authorization required (interactive TTY, no CLAUDECODE)"
        );
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("approve: not inside a git repository");
            return 1;
        }
    };
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);
    let rel = state::rel_path(&root, &file);

    if rel == "adrdom/terms.md" {
        run_terms(&root, &file, &led, &ledger_path)
    } else {
        run_record(&root, &file, &rel, &led, &ledger_path)
    }
}

fn run_terms(root: &Path, file: &Path, led: &Ledger, ledger_path: &Path) -> i32 {
    let mut tf = match terms::load(file) {
        Ok(tf) => tf,
        Err(e) => {
            eprintln!("approve: cannot read {}: {}", file.display(), e);
            return 1;
        }
    };

    let inserted = terms::fill_missing_ids(&mut tf);
    if let Err(e) = terms::save(&tf) {
        eprintln!("approve: failed to write {}: {}", file.display(), e);
        return 1;
    }

    let owner_errors = terms::check_owners(&tf, root);
    if !tf.errors.is_empty() || !owner_errors.is_empty() {
        for e in tf.errors.iter().chain(owner_errors.iter()) {
            eprintln!("approve: adrdom/terms.md: {}; owner fixes adrdom/terms.md", e);
        }
        return 1;
    }

    let files = repo::list_decision_files(root);

    // Refusals: a `terms:` line on an in-force decision that no longer
    // resolves the same way against the new terms (moved alias, retired
    // term), and any newly-banned word appearing anywhere in a record.
    let mut refusals: Vec<String> = Vec::new();
    let mut retired_reported: HashSet<(String, String)> = HashSet::new();
    let mut loaded: Vec<(String, record::RecordFile)> = Vec::new();
    for f in &files {
        let Ok(rf) = record::load(f) else { continue };
        loaded.push((state::rel_path(root, f), rf));
    }

    for (frel, rf) in &loaded {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_in_force(led, id) {
                continue;
            }
            let Some(tl) = sec.terms_line else { continue };
            let old_pairs = terms::parse_terms_line(&rf.lines[tl]).unwrap_or_default();
            let text = state::section_text_excluding_terms_line(&rf.lines, sec);
            let new_pairs = terms::resolve(&text, &tf.terms);
            if new_pairs != old_pairs {
                refusals.push(format!(
                    "approve: adrdom/terms.md: \"{}\" in {} no longer resolves the same terms; owner fixes adrdom/terms.md or the record",
                    sec.heading_text, frel
                ));
            }
            for (_, tid) in &new_pairs {
                if let Some(t) = tf.terms.iter().find(|t| t.id.as_deref() == Some(tid.as_str())) {
                    if t.retired && retired_reported.insert((frel.clone(), sec.heading_text.clone()))
                    {
                        refusals.push(format!(
                            "approve: adrdom/terms.md: \"{}\" in {} is bound to retired term \"{}\"; owner fixes adrdom/terms.md or the record",
                            sec.heading_text, frel, t.name
                        ));
                    }
                }
            }
        }

        let full_text = record::join_lines(&rf.lines);
        for (line_no, word, term_name) in terms::banned_hits(&full_text, &tf.terms) {
            refusals.push(format!(
                "approve: {}:{}: banned word \"{}\" (term {}); owner edits the record to remove it",
                frel, line_no, word, term_name
            ));
        }
    }

    if !refusals.is_empty() {
        for r in &refusals {
            eprintln!("{}", r);
        }
        return 1;
    }

    // Terms whose definition changed since the last committed version:
    // print every in-force decision still bound to them.
    if let Some(old_content) = repo::git(root, &["show", "HEAD:adrdom/terms.md"]) {
        let old_lines = record::split_lines(&old_content);
        let old_tf = terms::parse(&old_lines);
        for t in &tf.terms {
            let Some(tid) = &t.id else { continue };
            let Some(old_t) = old_tf.terms.iter().find(|o| o.id.as_deref() == Some(tid.as_str()))
            else {
                continue;
            };
            if old_t.definition == t.definition {
                continue;
            }
            println!("approve: term \"{}\" definition changed", t.name);
            for (frel, rf) in &loaded {
                for sec in &rf.sections {
                    let Some(id) = &sec.id else { continue };
                    if !state::is_in_force(led, id) {
                        continue;
                    }
                    let Some(tl) = sec.terms_line else { continue };
                    let pairs = terms::parse_terms_line(&rf.lines[tl]).unwrap_or_default();
                    if pairs.iter().any(|(_, pid)| pid == tid) {
                        println!("  in force: {}: {}", frel, sec.heading_text);
                    }
                }
            }
        }
    }

    if inserted > 0 {
        println!("approve: adrdom/terms.md: {} missing id(s) filled", inserted);
    }

    if state::terms_approved(led, &tf) {
        println!("approve: adrdom/terms.md: already approved");
        return 0;
    }

    let row = ledger::Row {
        time: ledger::now_rfc3339(),
        event: "terms".to_string(),
        id: "-".to_string(),
        hash: terms::file_hash(&tf),
        detail: "adrdom/terms.md".to_string(),
    };
    if let Err(e) = ledger::append(ledger_path, &[row]) {
        eprintln!("approve: failed to update ledger: {}", e);
        return 1;
    }
    println!("approve: adrdom/terms.md: approved");
    0
}

fn run_record(root: &Path, file: &Path, rel: &str, led: &Ledger, ledger_path: &Path) -> i32 {
    let rf = match record::load(file) {
        Ok(rf) => rf,
        Err(e) => {
            eprintln!("approve: cannot read {}: {}", file.display(), e);
            return 1;
        }
    };

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
        for m in &shape_msgs {
            eprintln!("{}", m);
        }
        return 1;
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

    if let Err(e) = ledger::append(ledger_path, &rows) {
        eprintln!("approve: failed to update ledger: {}", e);
        return 1;
    }
    for m in &messages {
        println!("{}", m);
    }

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
        for r in &refusals {
            eprintln!("{}", r);
        }
        return 1;
    }

    if !withdrawn_rows.is_empty() {
        if let Err(e) = ledger::append(ledger_path, &withdrawn_rows) {
            eprintln!("approve: failed to update ledger: {}", e);
            return 1;
        }
        for r in &withdrawn_rows {
            println!("approve: withdrawn: {}", r.detail);
        }
    }

    0
}
