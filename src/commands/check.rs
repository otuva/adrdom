use crate::banned;
use crate::ledger::{self, Ledger};
use crate::record::{self, Kind, RecordFile};
use crate::repo;
use crate::state::{self, rel_path};
use crate::terms;
use crate::view;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

struct Failure {
    check: String,
    /// The decision a failure is about, kept so the page can group by it.
    #[allow(dead_code)]
    id: Option<String>,
    message: String,
}

fn push_failures(out: &mut Vec<String>, fs: Vec<Failure>) {
    for f in fs {
        out.push(format!("{}: {}", f.check, f.message));
    }
}

/// Everything `check` reports, computed against `root` with the default
/// (non-staged, no message file) banned-word scan. Used by `run` (which
/// layers the CLI's `--staged`/`--message` modes on top) and by the `serve`
/// page, which always wants the plain repo-wide scan.
/// The full check report for the owner's page, reusing a tie index the
/// caller already computed so the repo's tracked files are scanned once.
pub fn collect_with_ties(root: &Path, ties: &HashMap<String, Vec<String>>) -> view::CheckReport {
    build(root, false, None, Some(ties))
}

fn build(
    root: &Path,
    staged: bool,
    message: Option<&Path>,
    ties: Option<&HashMap<String, Vec<String>>>,
) -> view::CheckReport {
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = ledger::read(&ledger_path);

    let files = repo::list_decision_files(root);
    let mut records = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    for f in &files {
        match record::load_in(root, f) {
            Ok(rf) => records.push(rf),
            Err(e) => notes.push(format!("parse: {}: cannot read: {}", f.display(), e)),
        }
    }

    let mut failures: Vec<String> = Vec::new();
    push_failures(&mut failures, check_parse(&records));
    push_failures(&mut failures, check_ids(&records, &led));
    push_failures(&mut failures, check_ledger(&led, &records, root, &ledger_path));
    push_failures(&mut failures, check_banned_ledger(&led, root));
    push_failures(&mut failures, check_answers(&records, &led));
    push_failures(&mut failures, check_banned(root, staged, message));
    push_failures(&mut failures, check_install(root));
    let (plan_failures, awaiting) = check_plan(root, &records, &led);
    push_failures(&mut failures, plan_failures);
    let (terms_failures, terms_awaiting, terms_notes) = check_terms(root, &records, &led, staged);
    push_failures(&mut failures, terms_failures);
    let (tie_failures, tied_count) = check_ties(root, &records, &led, ties);
    push_failures(&mut failures, tie_failures);

    if !awaiting.is_empty() {
        notes.push(format!(
            "plan: {} file(s) awaiting approval: {}",
            awaiting.len(),
            awaiting.join(", ")
        ));
    }
    if terms_awaiting {
        notes.push("terms: awaiting approval".to_string());
    }
    notes.extend(terms_notes);
    for d in check_done(root, &records, &led) {
        notes.push(format!("done: {}", d));
    }
    notes.push(format!("ties: {} tied", tied_count));

    view::CheckReport { failures, notes }
}

pub fn run(staged: bool, message: Option<PathBuf>) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("check: not inside a git repository");
            return 1;
        }
    };

    let report = build(&root, staged, message.as_deref(), None);
    let exit_code = if report.failures.is_empty() { 0 } else { 1 };

    for f in &report.failures {
        println!("{}", f);
    }
    for n in &report.notes {
        println!("{}", n);
    }

    exit_code
}

/// An id whose latest state is `superseded` or `withdrawn` is no longer
/// demanded in any record (4.13/6.1): the record it once lived in was
/// allowed to drop it.
fn dropped_state(led: &Ledger, id: &str) -> bool {
    matches!(
        led.latest_state(id).map(|r| r.event.as_str()),
        Some("superseded") | Some("withdrawn")
    )
}

/// A file's plan/context approval state: its ledger-relative path, whether
/// its current plan hash matches the latest `approved` row for that path,
/// and likewise for context/`context`.
fn file_approval(root: &Path, rf: &RecordFile, led: &Ledger) -> (String, bool, bool) {
    let rel = rel_path(root, &rf.path);
    let plan_h = record::plan_hash(rf);
    let ctx_h = record::context_hash(rf);
    let plan_ok = led
        .latest_for_path("approved", &rel)
        .map(|r| r.hash == plan_h)
        .unwrap_or(false);
    let ctx_ok = led
        .latest_for_path("context", &rel)
        .map(|r| r.hash == ctx_h)
        .unwrap_or(false);
    (rel, plan_ok, ctx_ok)
}

fn check_parse(records: &[RecordFile]) -> Vec<Failure> {
    let mut out = Vec::new();
    for rf in records {
        for e in &rf.shape_errors {
            out.push(Failure {
                check: "parse".to_string(),
                id: None,
                message: format!("{}: {}", rf.rel, e),
            });
        }
        for sec in &rf.sections {
            for e in &sec.parse_errors {
                out.push(Failure {
                    check: "parse".to_string(),
                    id: sec.id.clone(),
                    message: format!("{}: {}: {}", rf.rel, sec.heading_text, e),
                });
            }
            for e in &sec.shape_errors {
                out.push(Failure {
                    check: "parse".to_string(),
                    id: sec.id.clone(),
                    message: format!("{}: {}: {}", rf.rel, sec.heading_text, e),
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
            // Skeleton headings never fail anything, including ids.
            if sec.is_skeleton {
                continue;
            }
            match &sec.id {
                None => out.push(Failure {
                    check: "ids".to_string(),
                    id: None,
                    message: format!(
                        "{}: {}: missing id, run adr ask",
                        rf.rel,
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
                                rf.rel,
                                sec.heading_text
                            ),
                        });
                    } else {
                        seen.insert(
                            id.clone(),
                            format!("{}:{}", rf.rel, sec.heading_text),
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
        if dropped_state(led, &id) {
            continue;
        }
        if !seen.contains_key(&id) {
            out.push(Failure {
                check: "ids".to_string(),
                id: Some(id.clone()),
                message: format!("ledger id {} not found in any record", id),
            });
        }
    }

    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if let Some(row) = led.latest_state(id) {
                if row.event == "withdrawn" {
                    out.push(Failure {
                        check: "ids".to_string(),
                        id: Some(id.clone()),
                        message: format!(
                            "withdrawn id {} still present in {}",
                            id,
                            rf.rel
                        ),
                    });
                }
            }
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
        if row.event == "asked" && !all_ids.contains(&row.id) && !dropped_state(led, &row.id) {
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
                                rf.rel,
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
                                rf.rel,
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
                                    rf.rel,
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
    let entries = banned::read_entries(&banned_path);
    if entries.is_empty() {
        return out;
    }
    let hash_map: HashMap<String, Option<String>> = entries.into_iter().collect();
    let tf = state::load_terms(root);

    for (name, content) in collect_targets(root, staged, message) {
        for (i, line) in content.lines().enumerate() {
            let mut seen: HashSet<String> = HashSet::new();
            for tok in banned::tokenize(line) {
                let h = record::sha256_hex(&tok);
                let Some(uuid_opt) = hash_map.get(&h) else {
                    continue;
                };
                if !seen.insert(tok.clone()) {
                    continue;
                }
                let mut msg = format!("{}:{}: banned word \"{}\"", name, i + 1, tok);
                if let Some(uuid) = uuid_opt {
                    if let Some(term_name) = tf.as_ref().and_then(|f| {
                        f.terms
                            .iter()
                            .find(|t| t.id.as_deref() == Some(uuid.as_str()))
                            .map(|t| t.name.clone())
                    }) {
                        msg.push_str(&format!(" (term {})", term_name));
                    }
                }
                out.push(Failure {
                    check: "banned".to_string(),
                    id: None,
                    message: msg,
                });
            }
        }
    }
    out
}

/// The ban list against the ledger's `banned`/`unbanned` history: every
/// hash the ledger currently considers banned (a `banned` row with no
/// later `unbanned` row) must be in the file, and every hash in the file
/// must trace back to some `banned` row (regardless of a later `unbanned`
/// row, which would otherwise have removed it from the file already).
fn check_banned_ledger(led: &Ledger, root: &Path) -> Vec<Failure> {
    let mut out = Vec::new();
    let banned_path = root.join("adrdom/state/banned.sha256");
    let file_hashes = banned::read_set(&banned_path);

    let mut ever_banned: HashSet<String> = HashSet::new();
    let mut currently_banned: HashSet<String> = HashSet::new();
    for row in &led.rows {
        match row.event.as_str() {
            "banned" => {
                ever_banned.insert(row.hash.clone());
                currently_banned.insert(row.hash.clone());
            }
            "unbanned" => {
                currently_banned.remove(&row.hash);
            }
            _ => {}
        }
    }

    let mut missing_from_file: Vec<&String> = currently_banned
        .iter()
        .filter(|h| !file_hashes.contains(*h))
        .collect();
    missing_from_file.sort();
    for h in missing_from_file {
        out.push(Failure {
            check: "banned".to_string(),
            id: None,
            message: format!(
                "{} in ledger, missing from adrdom/state/banned.sha256",
                hash_prefix(h)
            ),
        });
    }

    let mut missing_from_ledger: Vec<&String> = file_hashes
        .iter()
        .filter(|h| !ever_banned.contains(*h))
        .collect();
    missing_from_ledger.sort();
    for h in missing_from_ledger {
        out.push(Failure {
            check: "banned".to_string(),
            id: None,
            message: format!("{} in file, no ledger row", hash_prefix(h)),
        });
    }

    out
}

fn hash_prefix(h: &str) -> &str {
    &h[..h.len().min(12)]
}

pub(crate) fn collect_targets(
    root: &Path,
    staged: bool,
    message: Option<&Path>,
) -> Vec<(String, String)> {
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

    if !root.join(".claude").is_dir() {
        return out;
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
                "{} cannot be parsed; owner runs: adr init --claude",
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
            ".claude/settings.json lacks deny entry {}; owner runs: adr init --claude",
            entry
        ),
    }
}

/// `plan`: per file, compares the current plan/context hash against the
/// latest `approved`/`context` ledger rows for that path. Returns the
/// failures plus the sorted, deduped list of paths still awaiting approval
/// (printed as a summary line by the caller, not a failure by itself).
fn check_plan(root: &Path, records: &[RecordFile], led: &Ledger) -> (Vec<Failure>, Vec<String>) {
    let mut out = Vec::new();
    let mut awaiting: Vec<String> = Vec::new();
    let mut heading_files: HashMap<String, Vec<String>> = HashMap::new();

    for rf in records {
        let (rel, plan_ok, ctx_ok) = file_approval(root, rf, led);
        if !plan_ok || !ctx_ok {
            awaiting.push(rel.clone());
        }
        if !plan_ok {
            for sec in &rf.sections {
                if !sec.is_skeleton {
                    out.push(Failure {
                        check: "plan".to_string(),
                        id: sec.id.clone(),
                        message: format!(
                            "{}: body under unapproved heading \"{}\"",
                            rel, sec.heading_text
                        ),
                    });
                }
            }
        }
        for sec in &rf.sections {
            if sec.kind == Kind::Question {
                heading_files
                    .entry(sec.heading_text.clone())
                    .or_default()
                    .push(rel.clone());
            }
        }
    }

    let mut headings: Vec<&String> = heading_files.keys().collect();
    headings.sort();
    for heading in headings {
        let mut paths = heading_files[heading].clone();
        paths.sort();
        paths.dedup();
        if paths.len() > 1 {
            out.push(Failure {
                check: "plan".to_string(),
                id: None,
                message: format!(
                    "duplicate heading \"{}\" in {} and {}",
                    heading, paths[0], paths[1]
                ),
            });
        }
    }

    awaiting.sort();
    awaiting.dedup();
    (out, awaiting)
}

/// `terms`: shape/approval failures on `adrdom/terms.md` (read from the
/// index when `staged` and the file is staged there, otherwise the working
/// tree). Missing or unapproved is a failure once any record file exists,
/// and a note otherwise. Every asked/answered section's terms line, and
/// every non-retired entry's own terms line, is checked for an existing,
/// non-retired uuid on each pair -- never by re-resolving the underlying
/// text. Circular definitions fail. Returns the failures, whether the
/// terms file itself is still awaiting approval (only meaningful when no
/// record exists yet), and any notes ("paired by nothing").
fn check_terms(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    staged: bool,
) -> (Vec<Failure>, bool, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    let has_records = !repo::list_decision_files(root).is_empty();

    let Some(content) = read_terms_content(root, staged) else {
        if has_records {
            out.push(Failure {
                check: "terms".to_string(),
                id: None,
                message: "adrdom/terms.md missing".to_string(),
            });
        }
        return (out, false, notes);
    };

    let lines = record::split_lines(&content);
    let tf = terms::parse(&lines);

    for e in &tf.errors {
        out.push(Failure {
            check: "terms".to_string(),
            id: None,
            message: format!("adrdom/terms.md: {}", e),
        });
    }

    let file_hash = terms::file_hash(&tf);
    let approved = led
        .latest_for_path("terms", "adrdom/terms.md")
        .map(|r| r.hash == file_hash)
        .unwrap_or(false);
    let mut awaiting = false;
    if !approved {
        if has_records {
            out.push(Failure {
                check: "terms".to_string(),
                id: None,
                message: "adrdom/terms.md not approved; owner runs: adr approve adrdom/terms.md"
                    .to_string(),
            });
        } else {
            awaiting = true;
        }
    }

    // Every asked/answered section: must have a terms line; each uuid on
    // it must name an entry that exists and is not retired. No
    // re-resolution of the section text against today's terms.
    for rf in records {
        let rel = rel_path(root, &rf.path);
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            let st = led.latest_state(id).map(|r| r.event.as_str());
            if !matches!(st, Some("asked") | Some("answered")) {
                continue;
            }
            let Some(tl) = sec.terms_line else {
                out.push(Failure {
                    check: "terms".to_string(),
                    id: Some(id.clone()),
                    message: format!("{}: {}: no terms line", rel, sec.heading_text),
                });
                continue;
            };
            let pairs = terms::parse_terms_line(rf.lines[tl].trim_end()).unwrap_or_default();
            for (_, uuid) in &pairs {
                match tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str())) {
                    None => out.push(Failure {
                        check: "terms".to_string(),
                        id: Some(id.clone()),
                        message: format!(
                            "{}: {}: entry {} not found",
                            rel, sec.heading_text, uuid
                        ),
                    }),
                    Some(t) if t.retired => out.push(Failure {
                        check: "terms".to_string(),
                        id: Some(id.clone()),
                        message: format!(
                            "{}: {}: bound term \"{}\" is retired",
                            rel, sec.heading_text, t.name
                        ),
                    }),
                    Some(_) => {}
                }
            }
        }
    }

    // Every non-retired entry's own terms line: the same two checks.
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        let Some(tl) = &t.terms_line else { continue };
        let pairs = terms::parse_terms_line(tl).unwrap_or_default();
        for (_, uuid) in &pairs {
            match tf.terms.iter().find(|o| o.id.as_deref() == Some(uuid.as_str())) {
                None => out.push(Failure {
                    check: "terms".to_string(),
                    id: None,
                    message: format!("adrdom/terms.md: \"{}\": entry {} not found", t.name, uuid),
                }),
                Some(o) if o.retired => out.push(Failure {
                    check: "terms".to_string(),
                    id: None,
                    message: format!(
                        "adrdom/terms.md: \"{}\": bound term \"{}\" is retired",
                        t.name, o.name
                    ),
                }),
                Some(_) => {}
            }
        }
    }

    // Circular definitions.
    for cycle in terms::cycles(&tf) {
        let mut chain = cycle.clone();
        if let Some(first) = cycle.first() {
            chain.push(first.clone());
        }
        out.push(Failure {
            check: "terms".to_string(),
            id: None,
            message: format!("adrdom/terms.md: circular definition: {}", chain.join(" -> ")),
        });
    }

    // Note (never a failure): every non-retired entry that no in-force
    // decision's terms line, and no non-retired entry's own terms line,
    // pairs against.
    let mut used: HashSet<String> = HashSet::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_in_force(led, id) {
                continue;
            }
            let Some(tl) = sec.terms_line else { continue };
            if let Some(pairs) = terms::parse_terms_line(rf.lines[tl].trim_end()) {
                for (_, uuid) in pairs {
                    used.insert(uuid);
                }
            }
        }
    }
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        let Some(tl) = &t.terms_line else { continue };
        if let Some(pairs) = terms::parse_terms_line(tl) {
            for (_, uuid) in pairs {
                used.insert(uuid);
            }
        }
    }
    let mut unused: Vec<String> = tf
        .terms
        .iter()
        .filter(|t| !t.retired)
        .filter(|t| t.id.as_ref().map(|id| !used.contains(id)).unwrap_or(true))
        .map(|t| t.name.clone())
        .collect();
    unused.sort();
    if !unused.is_empty() {
        notes.push(format!(
            "terms: {} entries paired by nothing: {}",
            unused.len(),
            unused.join(", ")
        ));
    }

    (out, awaiting, notes)
}

/// `adrdom/terms.md`'s content for `check`: from the index when `staged`
/// and the file is staged there, otherwise the working tree. `None` when
/// the file exists in neither place.
fn read_terms_content(root: &Path, staged: bool) -> Option<String> {
    if staged {
        if let Some(list) = repo::git(root, &["diff", "--cached", "--name-only"]) {
            if list.lines().any(|p| p == "adrdom/terms.md") {
                return repo::git(root, &["show", ":adrdom/terms.md"]);
            }
        }
    }
    std::fs::read_to_string(root.join("adrdom/terms.md")).ok()
}

/// `done`: every file whose plan and context are approved and whose every
/// h3/principle section carries an answer line (4.15).
fn check_done(root: &Path, records: &[RecordFile], led: &Ledger) -> Vec<String> {
    let mut out = Vec::new();
    for rf in records {
        let (rel, plan_ok, ctx_ok) = file_approval(root, rf, led);
        if !plan_ok || !ctx_ok {
            continue;
        }
        if rf.sections.iter().all(|s| s.answer_line.is_some()) {
            out.push(rel);
        }
    }
    out.sort();
    out
}

/// Every id that appears after `adr: ` in a tracked, non-`adrdom/` file, as
/// a full uuid token, mapped to every `path:line` location it was found at
/// (4.10-adjacent bookkeeping used by both `check`'s `ties` check and the
/// page's decision view for a ties count per id).
pub fn tie_index(root: &Path) -> HashMap<String, Vec<String>> {
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
    tie_ids
}

fn check_ties(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    ties: Option<&HashMap<String, Vec<String>>>,
) -> (Vec<Failure>, usize) {
    let mut out = Vec::new();

    let in_force: HashSet<String> = records
        .iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.clone()))
        .filter(|id| matches!(led.latest_state(id), Some(row) if row.event == "answered"))
        .collect();

    let owned;
    let tie_ids: &HashMap<String, Vec<String>> = match ties {
        Some(t) => t,
        None => {
            owned = tie_index(root);
            &owned
        }
    };

    let mut tied_count = 0;

    for id in &in_force {
        if tie_ids.contains_key(id) {
            tied_count += 1;
        } else {
            out.push(Failure {
                check: "ties".to_string(),
                id: Some(id.clone()),
                message: format!("{}: no tie", id),
            });
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

    (out, tied_count)
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
