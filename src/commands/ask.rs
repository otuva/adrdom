use crate::ledger;
use crate::record;
use crate::repo;
use crate::state;
use crate::terms;
use std::path::PathBuf;
use uuid::Uuid;

pub fn run(record_path: PathBuf) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("ask: not inside a git repository");
            return 1;
        }
    };
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);
    let rel = state::rel_path(&root, &record_path);

    let mut rf = match record::load(&record_path) {
        Ok(rf) => rf,
        Err(e) => {
            eprintln!("ask: cannot read {}: {}", record_path.display(), e);
            return 1;
        }
    };

    // Shape errors refuse first, before anything else is even considered.
    let mut shape_msgs: Vec<String> = rf
        .shape_errors
        .iter()
        .map(|e| format!("ask: {}: {}; owner fixes the record", rel, e))
        .collect();
    for sec in &rf.sections {
        for e in &sec.shape_errors {
            shape_msgs.push(format!(
                "ask: {}: \"{}\": {}; owner fixes the record",
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

    // Refuse (and change nothing) if any answered section was edited since
    // it was answered.
    for sec in &rf.sections {
        if let Some(id) = &sec.id {
            if let Some(row) = led.latest_state(id) {
                if row.event == "answered" {
                    let cur = record::section_hash(&rf.lines, sec);
                    if cur != row.hash {
                        eprintln!(
                            "ask: {}: \"{}\" was edited after being answered; refusing",
                            record_path.display(),
                            sec.heading_text
                        );
                        return 1;
                    }
                }
            }
        }
    }

    // Plan / context / terms must be approved before ask will touch the
    // file at all.
    if !state::plan_approved(&led, &rf, &rel) {
        eprintln!(
            "ask: {}: plan not approved; owner runs: adr approve {}",
            rel, rel
        );
        return 1;
    }
    if !state::context_approved(&led, &rf, &rel) {
        eprintln!(
            "ask: {}: context not approved; owner runs: adr approve {}",
            rel, rel
        );
        return 1;
    }
    let terms_opt = state::load_terms(&root);
    if let Some(tf) = &terms_opt {
        if !state::terms_approved(&led, tf) {
            eprintln!(
                "ask: {}: terms not approved; owner runs: adr approve adrdom/terms.md",
                rel
            );
            return 1;
        }
    }

    // Sections that are about to be newly asked or re-asked this run: no
    // id yet, or an id whose current hash differs from its latest `asked`
    // row. Computed against the file exactly as loaded, before any
    // mutation, so the gates below (and any refusal) see the same file
    // the owner is looking at.
    let candidate: Vec<bool> = rf
        .sections
        .iter()
        .map(|sec| {
            if sec.is_skeleton {
                return false;
            }
            match &sec.id {
                None => true,
                Some(id) => match led.latest_state(id) {
                    None => true,
                    Some(row) if row.event == "asked" => {
                        record::section_hash(&rf.lines, sec) != row.hash
                    }
                    _ => false,
                },
            }
        })
        .collect();

    let terms_list: Vec<terms::Term> = terms_opt
        .as_ref()
        .map(|tf| tf.terms.clone())
        .unwrap_or_default();

    for (i, sec) in rf.sections.iter().enumerate() {
        if !candidate[i] {
            continue;
        }

        // Tier rule: every group before this section's group must have all
        // of its non-skeleton questions answered.
        if let Some(n) = sec.group {
            for gi in 0..n {
                let Some(g) = rf.groups.get(gi) else { continue };
                for &qi in &g.question_idx {
                    let other = &rf.sections[qi];
                    if !other.is_skeleton && other.answer_line.is_none() {
                        eprintln!(
                            "ask: {}: \"{}\" is blocked by unanswered \"{}\" in an earlier group; owner runs: adr lock {} <letter> on it first",
                            rel,
                            sec.heading_text,
                            other.heading_text,
                            other.id.as_deref().unwrap_or("<id>")
                        );
                        return 1;
                    }
                }
            }
        }

        if terms_opt.is_some() {
            let text = state::section_text_excluding_terms_line(&rf.lines, sec);
            if let Some((_, word, term_name)) =
                terms::banned_hits(&text, &terms_list).into_iter().next()
            {
                eprintln!(
                    "ask: {}: \"{}\" contains banned word \"{}\" (term {}); owner edits the record",
                    rel, sec.heading_text, word, term_name
                );
                return 1;
            }
            for (word, tid) in terms::resolve(&text, &terms_list) {
                if let Some(t) = terms_list.iter().find(|t| t.id.as_deref() == Some(tid.as_str())) {
                    if t.retired {
                        eprintln!(
                            "ask: {}: \"{}\" resolves \"{}\" to retired term \"{}\"; owner edits the record or un-retires the term",
                            rel, sec.heading_text, word, t.name
                        );
                        return 1;
                    }
                }
            }
        }
    }

    // Insert missing ids, bottom-to-top so earlier insertions don't shift
    // the line numbers sections below still need to be found at.
    let needs_id: Vec<bool> = rf.sections.iter().map(|s| s.id.is_none()).collect();
    let mut ids_inserted = false;
    for i in (0..rf.sections.len()).rev() {
        if needs_id[i] {
            let start = rf.sections[i].start;
            let new_id = Uuid::new_v4().to_string();
            rf.lines.splice(
                start + 1..start + 1,
                [String::new(), format!("id: {}", new_id), String::new()],
            );
            ids_inserted = true;
        }
    }
    if ids_inserted {
        rf.sections = record::parse_sections(&rf.lines);
    }

    // Write/replace/remove the `terms:` line directly under the `id:` line
    // for every candidate, bottom-to-top so earlier sections' line numbers
    // are unaffected by a section growing or shrinking by one line.
    let mut terms_touched = false;
    for i in (0..rf.sections.len()).rev() {
        if !candidate[i] {
            continue;
        }
        let sec = rf.sections[i].clone();
        let text = state::section_text_excluding_terms_line(&rf.lines, &sec);
        let pairs = terms::resolve(&text, &terms_list);
        let new_line = terms::format_terms_line(&pairs);
        match (sec.terms_line, new_line) {
            (Some(tl), Some(nl)) => {
                if rf.lines[tl] != nl {
                    rf.lines[tl] = nl;
                    terms_touched = true;
                }
            }
            (Some(tl), None) => {
                rf.lines.remove(tl);
                terms_touched = true;
            }
            (None, Some(nl)) => {
                if let Some(idx) = find_prefixed_line(&rf.lines, sec.start, sec.end, "id: ") {
                    rf.lines.insert(idx + 1, nl);
                    terms_touched = true;
                }
            }
            (None, None) => {}
        }
    }
    if terms_touched {
        rf.sections = record::parse_sections(&rf.lines);
    }

    let mut new_rows = Vec::new();
    let mut summary = Vec::new();
    for sec in &rf.sections {
        let Some(id) = &sec.id else { continue };
        let cur = record::section_hash(&rf.lines, sec);
        let label = match led.latest_state(id) {
            None => Some("asked"),
            Some(row) if row.event == "asked" && row.hash != cur => Some("re-asked"),
            _ => None,
        };
        if let Some(label) = label {
            new_rows.push(ledger::Row {
                time: ledger::now_rfc3339(),
                event: "asked".to_string(),
                id: id.clone(),
                hash: cur,
                detail: sec.heading_text.clone(),
            });
            summary.push(format!("{}: {}", label, sec.heading_text));
        }
    }

    if new_rows.is_empty() {
        println!("ask: nothing to do in {}", record_path.display());
        return 0;
    }

    if let Err(e) = record::save(&rf) {
        eprintln!("ask: failed to write {}: {}", record_path.display(), e);
        return 1;
    }
    if let Err(e) = ledger::append(&ledger_path, &new_rows) {
        eprintln!("ask: failed to update ledger: {}", e);
        return 1;
    }

    for s in &summary {
        println!("{}", s);
    }
    0
}

/// The line index of the first line in `[start+1, end)` that starts with
/// `prefix`. Used to find the `id: ` line to write the `terms: ` line
/// directly under.
fn find_prefixed_line(lines: &[String], start: usize, end: usize, prefix: &str) -> Option<usize> {
    (start + 1..end).find(|&j| lines[j].starts_with(prefix))
}
