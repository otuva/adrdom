use crate::banned;
use crate::ledger::{self, Ledger};
use crate::record::{self, RecordFile};
use crate::repo;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

struct Failure {
    check: String,
    id: Option<String>,
    message: String,
}

pub fn run(staged: bool, message: Option<PathBuf>) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("check: not inside a git repository");
            return 1;
        }
    };
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);

    let files = repo::list_decision_files(&root);
    let mut records = Vec::new();
    for f in &files {
        match record::load(f) {
            Ok(rf) => records.push(rf),
            Err(e) => println!("parse: {}: cannot read: {}", f.display(), e),
        }
    }

    let mut failures = Vec::new();
    failures.extend(check_parse(&records));
    failures.extend(check_ids(&records, &led));
    failures.extend(check_ledger(&led, &records, &root, &ledger_path));
    failures.extend(check_answers(&records, &led));
    failures.extend(check_banned(&root, staged, message.as_deref()));
    failures.extend(check_install(&root));
    let (tie_failures, tied_count, unchecked_count) = check_ties(&root, &records, &led);
    failures.extend(tie_failures);

    let mut exit_code = 0;
    for f in &failures {
        println!("{}: {}", f.check, f.message);
        exit_code = 1;
    }

    println!("ties: {} tied, {} unchecked", tied_count, unchecked_count);

    exit_code
}

fn check_parse(records: &[RecordFile]) -> Vec<Failure> {
    let mut out = Vec::new();
    for rf in records {
        for sec in &rf.sections {
            for e in &sec.parse_errors {
                out.push(Failure {
                    check: "parse".to_string(),
                    id: sec.id.clone(),
                    message: format!("{}: {}: {}", rf.path.display(), sec.heading_text, e),
                });
            }
        }
    }
    out
}

fn check_ids(records: &[RecordFile], led: &Ledger) -> Vec<Failure> {
    let mut out = Vec::new();
    let mut seen: HashMap<String, String> = HashMap::new();
    for rf in records {
        for sec in &rf.sections {
            match &sec.id {
                None => out.push(Failure {
                    check: "ids".to_string(),
                    id: None,
                    message: format!(
                        "{}: {}: missing id, run adr ask",
                        rf.path.display(),
                        sec.heading_text
                    ),
                }),
                Some(id) => {
                    if let Some(prev) = seen.get(id) {
                        out.push(Failure {
                            check: "ids".to_string(),
                            id: Some(id.clone()),
                            message: format!(
                                "duplicate id {} in {} and {}:{}",
                                id,
                                prev,
                                rf.path.display(),
                                sec.heading_text
                            ),
                        });
                    } else {
                        seen.insert(
                            id.clone(),
                            format!("{}:{}", rf.path.display(), sec.heading_text),
                        );
                    }
                }
            }
        }
    }

    let mut ledger_ids: HashSet<String> = HashSet::new();
    for row in &led.rows {
        if (row.event == "asked" || row.event == "answered") && row.id != "-" {
            ledger_ids.insert(row.id.clone());
        }
    }
    for id in ledger_ids {
        if !seen.contains_key(&id) {
            out.push(Failure {
                check: "ids".to_string(),
                id: Some(id.clone()),
                message: format!("ledger id {} not found in any record", id),
            });
        }
    }
    out
}

fn check_ledger(
    led: &Ledger,
    records: &[RecordFile],
    root: &Path,
    ledger_path: &Path,
) -> Vec<Failure> {
    let mut out = Vec::new();
    for (lineno, raw) in &led.malformed {
        out.push(Failure {
            check: "ledger".to_string(),
            id: None,
            message: format!("line {}: malformed row: {}", lineno, raw),
        });
    }

    if let Some(head_content) = repo::git(root, &["show", "HEAD:adrdom/state/ledger.tsv"]) {
        let working = std::fs::read_to_string(ledger_path).unwrap_or_default();
        if !working.starts_with(&head_content) {
            out.push(Failure {
                check: "ledger".to_string(),
                id: None,
                message: "not append-only: working copy diverges from HEAD".to_string(),
            });
        }
    }

    let all_ids: HashSet<String> = records
        .iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.clone()))
        .collect();

    for row in &led.rows {
        if row.event == "asked" && !all_ids.contains(&row.id) {
            out.push(Failure {
                check: "ledger".to_string(),
                id: Some(row.id.clone()),
                message: format!("asked row id {} not found in any record", row.id),
            });
        }
    }

    for row in &led.rows {
        if row.event == "answered" {
            let has_match = led
                .rows
                .iter()
                .any(|r| r.event == "asked" && r.id == row.id && r.hash == row.hash);
            if !has_match {
                out.push(Failure {
                    check: "ledger".to_string(),
                    id: Some(row.id.clone()),
                    message: format!(
                        "answered row for {} has no matching earlier asked hash",
                        row.id
                    ),
                });
            }
        }
    }

    out
}

fn check_answers(records: &[RecordFile], led: &Ledger) -> Vec<Failure> {
    let mut out = Vec::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            let cur = record::section_hash(&rf.lines, sec);
            match sec.answer_line {
                Some(_) => {
                    let letter = sec.answer_letter();
                    let ok = letter
                        .map(|l| {
                            led.rows.iter().any(|r| {
                                r.event == "answered"
                                    && &r.id == id
                                    && r.hash == cur
                                    && r.detail == l.to_string()
                            })
                        })
                        .unwrap_or(false);
                    if !ok {
                        out.push(Failure {
                            check: "answers".to_string(),
                            id: Some(id.clone()),
                            message: format!(
                                "{}: {}: answer line without ledger row",
                                rf.path.display(),
                                sec.heading_text
                            ),
                        });
                    }
                }
                None => {
                    let has_answered_row = led
                        .rows
                        .iter()
                        .any(|r| r.event == "answered" && &r.id == id);
                    if has_answered_row {
                        out.push(Failure {
                            check: "answers".to_string(),
                            id: Some(id.clone()),
                            message: format!(
                                "{}: {}: answered row without answer line",
                                rf.path.display(),
                                sec.heading_text
                            ),
                        });
                    } else {
                        let latest_asked_hash = led.latest_asked(id).map(|r| r.hash.clone());
                        if latest_asked_hash.as_deref() != Some(cur.as_str()) {
                            out.push(Failure {
                                check: "answers".to_string(),
                                id: Some(id.clone()),
                                message: format!(
                                    "{}: {}: edited after asking; run adr ask",
                                    rf.path.display(),
                                    sec.heading_text
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
    out
}

fn check_banned(root: &Path, staged: bool, message: Option<&Path>) -> Vec<Failure> {
    let mut out = Vec::new();
    let banned_path = root.join("adrdom/state/banned.sha256");
    let set = banned::read_set(&banned_path);
    if set.is_empty() {
        return out;
    }

    for (name, content) in collect_targets(root, staged, message) {
        for (i, line) in content.lines().enumerate() {
            let hit = banned::tokenize(line)
                .iter()
                .any(|tok| set.contains(&record::sha256_hex(tok)));
            if hit {
                out.push(Failure {
                    check: "banned".to_string(),
                    id: None,
                    message: format!("{}:{}", name, i + 1),
                });
            }
        }
    }
    out
}

fn collect_targets(root: &Path, staged: bool, message: Option<&Path>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(msg_path) = message {
        if let Ok(content) = std::fs::read_to_string(msg_path) {
            out.push((msg_path.display().to_string(), content));
        }
        return out;
    }
    if staged {
        if let Some(list) = repo::git(
            root,
            &["diff", "--cached", "--name-only", "--diff-filter=d"],
        ) {
            for path in list.lines() {
                if skip_path(path) {
                    continue;
                }
                if let Some(content) = repo::git(root, &["show", &format!(":{}", path)]) {
                    out.push((path.to_string(), content));
                }
            }
        }
        return out;
    }

    let mut paths: Vec<String> = Vec::new();
    if let Some(list) = repo::git(root, &["ls-files"]) {
        paths.extend(list.lines().map(|s| s.to_string()));
    }
    for f in repo::list_decision_files(root) {
        if let Ok(rel) = f.strip_prefix(root) {
            let s = rel.to_string_lossy().replace('\\', "/");
            if !paths.contains(&s) {
                paths.push(s);
            }
        }
    }
    for p in paths {
        if skip_path(&p) {
            continue;
        }
        let full = root.join(&p);
        if let Ok(content) = std::fs::read_to_string(&full) {
            out.push((p, content));
        }
    }
    out
}

fn skip_path(p: &str) -> bool {
    p == "adrdom/state/banned.sha256" || p.starts_with(".git/")
}

fn check_install(root: &Path) -> Vec<Failure> {
    let mut out = Vec::new();

    if !root.join("adrdom/rules.md").is_file() {
        out.push(Failure {
            check: "install".to_string(),
            id: None,
            message: "adrdom/rules.md missing; owner runs: adr init".to_string(),
        });
    }

    let has_include = std::fs::read_to_string(root.join("CLAUDE.md"))
        .map(|c| c.lines().any(|l| l == "@adrdom/rules.md"))
        .unwrap_or(false);
    if !has_include {
        out.push(Failure {
            check: "install".to_string(),
            id: None,
            message: "CLAUDE.md lacks @adrdom/rules.md; owner runs: adr init".to_string(),
        });
    }

    let settings_path = root.join(".claude/settings.json");
    let content = std::fs::read_to_string(&settings_path).unwrap_or_default();
    if content.trim().is_empty() {
        for entry in super::DENY_ENTRIES {
            out.push(missing_deny_entry(entry));
        }
        return out;
    }
    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(value) => {
            let deny: Vec<&str> = value
                .get("permissions")
                .and_then(|p| p.get("deny"))
                .and_then(|d| d.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            for entry in super::DENY_ENTRIES {
                if !deny.contains(entry) {
                    out.push(missing_deny_entry(entry));
                }
            }
        }
        Err(_) => out.push(Failure {
            check: "install".to_string(),
            id: None,
            message: format!(
                "{} cannot be parsed; owner runs: adr init",
                settings_path.display()
            ),
        }),
    }

    out
}

fn missing_deny_entry(entry: &str) -> Failure {
    Failure {
        check: "install".to_string(),
        id: None,
        message: format!(
            ".claude/settings.json lacks deny entry {}; owner runs: adr init",
            entry
        ),
    }
}

fn check_ties(root: &Path, records: &[RecordFile], led: &Ledger) -> (Vec<Failure>, usize, usize) {
    let mut out = Vec::new();

    let in_force: HashSet<String> = records
        .iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.clone()))
        .filter(|id| matches!(led.latest_state(id), Some(row) if row.event == "answered"))
        .collect();

    let mut tie_ids: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(list) = repo::git(root, &["ls-files"]) {
        for path in list.lines() {
            if path.starts_with("adrdom/") {
                continue;
            }
            let full = root.join(path);
            let Ok(content) = std::fs::read_to_string(&full) else {
                continue;
            };
            for (i, line) in content.lines().enumerate() {
                if let Some(pos) = line.find("adr: ") {
                    let rest = &line[pos + 5..];
                    let id_tok: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
                    if is_uuid(&id_tok) {
                        tie_ids
                            .entry(id_tok)
                            .or_default()
                            .push(format!("{}:{}", path, i + 1));
                    }
                }
            }
        }
    }

    let unchecked_path = root.join("adrdom/state/unchecked.tsv");
    let mut unchecked_ids: HashSet<String> = HashSet::new();
    if let Ok(content) = std::fs::read_to_string(&unchecked_path) {
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            if let Some(id) = line.split('\t').next() {
                unchecked_ids.insert(id.to_string());
            }
        }
    }

    let mut tied_count = 0;
    let mut unchecked_count = 0;

    for id in &in_force {
        let has_tie = tie_ids.contains_key(id);
        let has_unchecked = unchecked_ids.contains(id);
        match (has_tie, has_unchecked) {
            (true, true) => out.push(Failure {
                check: "ties".to_string(),
                id: Some(id.clone()),
                message: format!("{}: both tied and listed as unchecked", id),
            }),
            (false, false) => out.push(Failure {
                check: "ties".to_string(),
                id: Some(id.clone()),
                message: format!("{}: no tie and not listed as unchecked", id),
            }),
            (true, false) => tied_count += 1,
            (false, true) => unchecked_count += 1,
        }
    }

    for id in tie_ids.keys() {
        if !in_force.contains(id) {
            out.push(Failure {
                check: "ties".to_string(),
                id: Some(id.clone()),
                message: format!("{}: stale (not an in-force decision)", id),
            });
        }
    }
    for id in &unchecked_ids {
        if !in_force.contains(id) {
            out.push(Failure {
                check: "ties".to_string(),
                id: Some(id.clone()),
                message: format!("{}: stale (not an in-force decision)", id),
            });
        }
    }

    (out, tied_count, unchecked_count)
}

/// A tie only counts when the token after `adr: ` is a full uuid
/// (8-4-4-4-12 lowercase hex); any other text after `adr:` is prose.
fn is_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&parts)
            .all(|(n, p)| p.len() == *n && p.chars().all(|c| c.is_ascii_hexdigit()))
}
