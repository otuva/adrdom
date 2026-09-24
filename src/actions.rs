//! One code path for every owner act: the CLI and the page both call these (7.2 A).
use crate::depends::{self, DepTarget};
use crate::ledger::{self, Ledger};
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state::{self, TermsLoad};
use crate::terms;
use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use uuid::Uuid;

pub struct Outcome {
    pub summary: String,
}

/// The terms.md gates shared by `do_answer` and `do_refuse` (5): missing,
/// present but unreadable/shape-invalid, or not approved, each refusing with
/// the same message either caller would print. `rel` is the topic file's
/// path, named in the not-approved message exactly as both callers already
/// print it. On success, the loaded, approved terms file is returned (only
/// `do_answer` has further use for it).
fn require_terms(root: &Path, led: &Ledger, rel: &str) -> Result<terms::TermsFile, String> {
    match state::load_terms_checked(root) {
        TermsLoad::Missing => Err(
            "adrdom/terms.md missing; agent writes it, owner runs: adr approve adrdom/terms.md"
                .to_string(),
        ),
        TermsLoad::LoadError(e) => Err(format!("adrdom/terms.md: {}", e)),
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
            if !state::terms_approved(led, &tf) {
                return Err(format!(
                    "{}: terms not approved; owner runs: adr approve adrdom/terms.md",
                    rel
                ));
            }
            Ok(tf)
        }
    }
}

pub fn do_answer(root: &Path, id: &str, letter: char) -> Result<Outcome, String> {
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
        return Err(format!("id {} not found in any topic file", id));
    };
    let rel = state::rel_path(root, &rf.path);

    // Refuse when terms.md is missing, has shape errors, or is not
    // approved: the same messages `ask` uses, prefixed `answer:` by the
    // caller in `commands::answer::run`. The loaded, approved terms file is
    // kept for the terms-line validity check below.
    let tf = require_terms(root, &led, &rel)?;

    let cur_hash;
    let supersedes_val;
    {
        let sec = &rf.sections[idx];
        if sec.is_refused() {
            return Err(format!("\"{}\" is already refused", sec.heading_text));
        }
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
        let Some(terms_line_idx) = sec.terms_line else {
            return Err(format!("{}: no terms line; agent re-asks", id));
        };
        let Some(pairs) = terms::parse_terms_line(rf.lines[terms_line_idx].trim_end()) else {
            return Err(format!("{}: malformed terms line; agent re-asks", id));
        };
        for (_, uuid) in &pairs {
            match tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str())) {
                None => {
                    return Err(format!(
                        "{}: terms line names term {} that no longer exists; agent re-asks",
                        id, uuid
                    ));
                }
                Some(t) if t.retired => {
                    return Err(format!(
                        "{}: terms line names retired term \"{}\"; agent re-asks",
                        id, t.name
                    ));
                }
                _ => {}
            }
        }
        supersedes_val = sec.supersedes.clone();
        for old in &supersedes_val {
            // A supersedes target must be current (4.14 A): latest state
            // `answered`. An already superseded id (or anything else)
            // refuses; the agent rewrites the supersedes line to name the
            // decision that actually superseded it.
            match led.latest_state(old) {
                Some(row) if row.event == "answered" => {}
                _ => {
                    return Err(format!(
                        "supersedes target {} is not a current decision; agent fixes the supersedes line",
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

    record::save(&rf).map_err(|e| format!("failed to write topic file: {}", e))?;

    let mut rows = vec![ledger::Row {
        time: ledger::now_rfc3339(),
        event: "answered".to_string(),
        id: id.to_string(),
        hash: cur_hash,
        detail: letter.to_string(),
        version: crate::version::VERSION.to_string(),
    }];
    for old in &supersedes_val {
        rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "superseded".to_string(),
            id: old.clone(),
            hash: "-".to_string(),
            detail: id.to_string(),
            version: crate::version::VERSION.to_string(),
        });
    }
    ledger::append(&ledger_path, &rows).map_err(|e| format!("failed to update ledger: {}", e))?;

    Ok(Outcome {
        summary: format!("answered: {}\n  {}", heading_for_print, sentence_for_print),
    })
}

/// `adr answer <id> refuse` (8.1 A). The refusals shared with the letter
/// path (id not found, already answered, terms, hash state) apply; the
/// letter-specific option check and the section's own terms-line/pairing
/// checks do not, since a refusal commits no decision sentence bound to a
/// term. On success: `**Answer:** refused` is appended at the end of the
/// hash range, and a `refused` row is written (hash = the section hash
/// before the insert, detail = heading). A `supersedes:` line on a refused
/// question is never acted on (no target is checked, no `superseded` row
/// is written); its presence only earns a printed note.
pub fn do_refuse(root: &Path, id: &str) -> Result<Outcome, String> {
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
        return Err(format!("id {} not found in any topic file", id));
    };
    let rel = state::rel_path(root, &rf.path);

    // Same terms.md gates as the letter path (missing/shape/not approved);
    // the section's own terms-line/pairing checks are not shared (5).
    require_terms(root, &led, &rel)?;

    let cur_hash;
    let supersedes_val;
    {
        let sec = &rf.sections[idx];
        if sec.is_refused() {
            return Err(format!("\"{}\" is already refused", sec.heading_text));
        }
        if sec.answer_line.is_some() {
            return Err(format!("\"{}\" is already answered", sec.heading_text));
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
        supersedes_val = sec.supersedes.clone();
    }

    let heading_for_print;
    let mut note: Option<String> = None;
    {
        let sec = &rf.sections[idx];
        let (_, hash_end) = record::hash_range(&rf.lines, sec);
        heading_for_print = sec.heading_text.clone();
        if !supersedes_val.is_empty() {
            note = Some(format!(
                "answer: {}: supersedes ignored on a refused question",
                heading_for_print
            ));
        }
        rf.lines.splice(
            hash_end..hash_end,
            [String::new(), "**Answer:** refused".to_string()],
        );
    }

    record::save(&rf).map_err(|e| format!("failed to write topic file: {}", e))?;

    let row = ledger::Row {
        time: ledger::now_rfc3339(),
        event: "refused".to_string(),
        id: id.to_string(),
        hash: cur_hash,
        detail: heading_for_print.clone(),
        version: crate::version::VERSION.to_string(),
    };
    ledger::append(&ledger_path, &[row]).map_err(|e| format!("failed to update ledger: {}", e))?;

    let mut summary = format!(
        "answer: refused \"{}\"; agent re-plans, owner runs: adr approve {}",
        heading_for_print, rel
    );
    if let Some(n) = note {
        summary.push('\n');
        summary.push_str(&n);
    }

    Ok(Outcome { summary })
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
    /// `<rel>: "<heading>"` for a decision, or `term "<name>"` for a term:
    /// in both cases the recorded pairing, read from the dependent's own
    /// `terms:` line exactly as it stood in the file before this approve,
    /// never re-resolved from today's definition/section text.
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

    // Fresh definition pairs for every term (the new file, after
    // `mint_ids`, never re-derived from a stale terms line in the file).
    // Computed once and reused for the dependents check below, the write
    // in step 6, and step 7's term-dependent print.
    let pairs_per_entry: Vec<Vec<(String, String)>> = (0..tf.terms.len())
        .map(|i| terms::definition_pairs(&tf, i))
        .collect();

    // Every current decision's terms line, and every non-retired term's
    // fresh definition pairs, is a dependent.
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
            if !state::is_current(led, id) {
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
        // The recorded pairing, exactly as decisions' frozen dependents are
        // read: the term's own `terms:` line as it already stood in the
        // file before this approve, not a fresh re-resolve (9.3 B;
        // terms-walk: "definitions are dependents for retire/move
        // refusals"). A term with no terms line yet has no dependents.
        let Some(pairs) = t.terms_line.as_deref().and_then(terms::parse_terms_line) else {
            continue;
        };
        dependents.push(Dependent {
            source: format!("term \"{}\"", t.name),
            pairs,
        });
    }

    // Step 4: every pair on a dependent must still hold in the new file.
    // `lookup` answers "what does this exact word/phrase resolve to" for
    // every dependent pair, built once rather than once per pair.
    let lookup = terms::phrase_lookup(&tf.terms);
    let mut refusals: Vec<String> = Vec::new();
    for dep in &dependents {
        for (word, uuid) in &dep.pairs {
            let Some(target) = tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str()))
            else {
                refusals.push(format!(
                    "approve: adrdom/terms.md: term {} removed but carried by {}; owner retires instead",
                    uuid, dep.source
                ));
                continue;
            };
            if target.retired {
                refusals.push(format!(
                    "approve: adrdom/terms.md: term \"{}\" retired but carried by {}; owner supersedes first",
                    target.name, dep.source
                ));
                continue;
            }
            let still_resolves = lookup.get(word) == Some(uuid);
            if !still_resolves {
                refusals.push(format!(
                    "approve: adrdom/terms.md: word \"{}\" moved off term \"{}\", carried by {}",
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

    // Step 6: write the file. `terms:` lines use the fresh pairs computed
    // above, one term at a time, in the same order as `tf.terms`.
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
            version: crate::version::VERSION.to_string(),
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
                            if !state::is_current(led, id) {
                                continue;
                            }
                            let Some(tl) = sec.terms_line else { continue };
                            let pairs =
                                terms::parse_terms_line(rf.lines[tl].trim_end()).unwrap_or_default();
                            if pairs.iter().any(|(_, pid)| pid == tid) {
                                out.push(format!("  current: {}: {}", frel, sec.heading_text));
                            }
                        }
                    }
                    for (i, t2) in tf.terms.iter().enumerate() {
                        if t2.retired {
                            continue;
                        }
                        if pairs_per_entry[i].iter().any(|(_, pid)| pid == tid) {
                            out.push(format!("  term: {}", t2.name));
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

/// A topic file: parse, refuse on shape errors; resolve every `depends:`
/// entry (9.1 A: the file holds ids, the agent writes words, and `approve`
/// is what turns one into the other); check every ledger id that now
/// appears in no topic file, refusing on a current (answered) one, before
/// anything is written -- withdrawing an asked/refused one and allowing a
/// superseded one silently happen after, since neither is a refusal; only
/// then mint an id for every heading that lacks one, compute plan hash and
/// context hash over the rewritten file, write an `approved` row if it
/// differs from the latest for the path, a `context` row likewise, and
/// finally write the withdrawn rows. Nothing is written to disk (file or
/// ledger) when any check refuses.
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
        .map(|e| format!("approve: {}: {}; owner fixes the topic file", rel, e))
        .collect();
    for sec in &rf.sections {
        for e in &sec.shape_errors {
            shape_msgs.push(format!(
                "approve: {}: \"{}\": {}; owner fixes the topic file",
                rel, sec.heading_text, e
            ));
        }
    }
    if !shape_msgs.is_empty() {
        return Err(shape_msgs.join("\n"));
    }

    // 8.3/8.6 B: every section, skeleton or not, needs at least one
    // `depends:` line naming what the question stands on, before anything
    // about it is resolved or written.
    let mut depends_msgs: Vec<String> = Vec::new();
    for sec in &rf.sections {
        if sec.depends.is_empty() {
            depends_msgs.push(format!(
                "approve: {}: \"{}\" has no depends line; agent writes what the question stands on",
                rel, sec.heading_text
            ));
        }
    }
    if !depends_msgs.is_empty() {
        return Err(depends_msgs.join("\n"));
    }

    // Resolve every entry (8.3/9.1 A). An `Id` entry is kept as written and
    // must already be a heading id in this file, a non-retired term id, or a
    // current decision. A `Text` entry ending in `?` is a heading reference,
    // whose dangling/cycle/self-dep is `derive_tiers`'s single error
    // (computed at parse time into `rf.derive_error`); any other `Text`
    // entry must resolve to exactly one non-retired term. A `Text` entry
    // (of either shape) found inside a section whose latest ledger state is
    // `answered`/`refused`/`superseded` refuses instead of being silently
    // rewritten (9.1 A point 5): that section is frozen -- its decision is
    // already closed, one way or another -- and an edit there is the
    // agent's to remove, not approve's to carry forward.
    let terms_list: Vec<terms::Term> =
        state::load_terms(root).map(|tf| tf.terms).unwrap_or_default();
    // Built once per command run (not per depends entry): `resolve_dep`
    // used to call `terms::resolve_word`, which rebuilt this same map from
    // scratch on every entry.
    let phrase_lookup = terms::phrase_lookup(&terms_list);
    let mut resolve_msgs: Vec<String> = Vec::new();
    for sec in &rf.sections {
        let frozen = sec
            .id
            .as_ref()
            .map(|id| {
                matches!(
                    led.latest_state(id).map(|r| r.event.as_str()),
                    Some("answered") | Some("refused") | Some("superseded")
                )
            })
            .unwrap_or(false);
        for d in &sec.depends {
            if frozen && matches!(d, record::Dep::Text { .. }) {
                resolve_msgs.push(format!(
                    "approve: {}: \"{}\": answered; agent removes the edit",
                    rel, sec.heading_text
                ));
                continue;
            }
            if let record::Dep::Text { text, .. } = d {
                if text.trim().ends_with('?') {
                    // Dangling/cycle/self-dep is reported once, below, via
                    // `rf.derive_error`.
                    continue;
                }
            }
            if let Err(e) = depends::resolve_dep(d, &rf, &terms_list, &phrase_lookup, led) {
                resolve_msgs.push(format!(
                    "approve: {}",
                    depends::dep_refusal_text(rel, &sec.heading_text, &e)
                ));
            }
        }
    }
    if let Some(err) = &rf.derive_error {
        resolve_msgs.push(format!(
            "approve: {}",
            record::derive_error_text(rel, err, None)
        ));
    }
    if !resolve_msgs.is_empty() {
        return Err(resolve_msgs.join("\n"));
    }

    // Cap (8.8 A): no derived tier may hold more than `TIER_CAP` questions.
    // `rf.tiers` is guaranteed `Some` here: a `derive_error` would already
    // have refused above.
    let tiers: &Vec<usize> = rf
        .tiers
        .as_ref()
        .expect("derive_error refuses above whenever tiers is None");
    if let Some((tier, count)) = record::over_cap(tiers) {
        return Err(format!(
            "approve: {}: tier {} holds {} questions, the cap is {}; agent moves questions under a higher heading or splits the topic",
            rel, tier, count, record::TIER_CAP
        ));
    }

    // Ledger ids (asked/answered/refused/superseded/withdrawn) that now
    // appear in no topic file at all: withdraw the asked/refused ones,
    // refuse on a current (answered, not superseded) one, allow a
    // superseded one silently. Computed and checked here, before any write
    // below, so a refusal on this leaves both the topic file and the
    // ledger exactly as they were (the function's own contract: nothing is
    // written when a check refuses). Reads `file` itself straight off disk
    // like every other topic file -- it is still unwritten at this point,
    // so its ids are exactly `rf`'s; minting new ids for skeleton headings
    // below never removes an id already in the ledger's `state_ids`, so
    // reading before that rewrite changes nothing this check depends on.
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
            "asked" | "answered" | "refused" | "superseded" | "withdrawn"
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
            // A refused id absent from every file is withdrawn exactly like
            // a struck asked one (8.2 A): the refusal already closed it
            // with no decision, so there's nothing left to protect.
            Some(r) if r.event == "asked" || r.event == "refused" => {
                withdrawn_rows.push(ledger::Row {
                    time: ledger::now_rfc3339(),
                    event: "withdrawn".to_string(),
                    id: id.clone(),
                    hash: "-".to_string(),
                    detail: r.detail.clone(),
                    version: crate::version::VERSION.to_string(),
                });
            }
            Some(r) if r.event == "answered" => {
                let heading = led
                    .latest_asked(id)
                    .map(|a| a.detail.clone())
                    .unwrap_or_else(|| id.clone());
                refusals.push(format!(
                    "approve: id {} (\"{}\") is answered and current but missing from every topic file; owner supersedes it or restores the heading",
                    id, heading
                ));
            }
            _ => {}
        }
    }

    if !refusals.is_empty() {
        return Err(refusals.join("\n"));
    }

    // Every check above passed: mint an id for every heading that lacks one
    // (skeletons included, same insertion shape `ask` used to write: blank,
    // `id:` line, blank), reparse so every heading's id is available, then
    // rewrite every `Text` depends entry to the id it resolves to, keeping
    // any `=set` suffix, in place (a single-line replace, never a shift).
    let mut work_lines = rf.lines.clone();
    let needs_id: Vec<bool> = rf.sections.iter().map(|s| s.id.is_none()).collect();
    for i in (0..rf.sections.len()).rev() {
        if needs_id[i] {
            let start = rf.sections[i].start;
            let new_id = Uuid::new_v4().to_string();
            work_lines.splice(
                start + 1..start + 1,
                [String::new(), format!("id: {}", new_id), String::new()],
            );
        }
    }
    let mut work_rf = record::build(rf.path.clone(), rel.to_string(), work_lines);

    let sections_snapshot = work_rf.sections.clone();
    for sec in &sections_snapshot {
        let mut dep_i = 0;
        for j in sec.start + 1..sec.end {
            if !work_rf.lines[j].starts_with("depends: ") {
                continue;
            }
            let d = &sec.depends[dep_i];
            dep_i += 1;
            if let record::Dep::Text { set, .. } = d {
                if let Ok(Some(target)) =
                    depends::resolve_dep(d, &work_rf, &terms_list, &phrase_lookup, led)
                {
                    let id = match target {
                        DepTarget::Heading(idx) => work_rf.sections[idx].id.clone(),
                        DepTarget::Term(id) | DepTarget::Decision(id) => Some(id),
                    };
                    if let Some(id) = id {
                        let suffix = set
                            .as_ref()
                            .map(|s| format!("={}", record::canonical_set(s)))
                            .unwrap_or_default();
                        work_rf.lines[j] = format!("depends: {}{}", id, suffix);
                    }
                }
            }
        }
    }
    // Reparse once more so `plan_hash`/`context_hash` and everything below
    // sees the file exactly as it will be written.
    let work_rf = record::build(work_rf.path.clone(), rel.to_string(), work_rf.lines.clone());

    let plan_hash = record::plan_hash(&work_rf);
    let context_hash = record::context_hash(&work_rf);

    record::save(&work_rf).map_err(|e| format!("approve: failed to write {}: {}", file.display(), e))?;

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
            version: crate::version::VERSION.to_string(),
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
            version: crate::version::VERSION.to_string(),
        });
        messages.push(format!("approve: {}: context approved", rel));
    } else {
        messages.push(format!("approve: {}: context unchanged", rel));
    }

    ledger::append(ledger_path, &rows)
        .map_err(|e| format!("approve: failed to update ledger: {}", e))?;

    // `withdrawn_rows` was computed, and any refusal already returned,
    // above -- before either write this function has made so far.
    if !withdrawn_rows.is_empty() {
        ledger::append(ledger_path, &withdrawn_rows)
            .map_err(|e| format!("approve: failed to update ledger: {}", e))?;
        for r in &withdrawn_rows {
            messages.push(format!("approve: withdrawn: {}", r.detail));
        }
    }

    // Printed last, after a successful record approval (spec §3): the
    // tiers this file now derives to, one `N (count)` per non-empty tier.
    let tiers_str = record::tier_sizes(tiers)
        .into_iter()
        .map(|(t, c)| format!("{} ({})", t, c))
        .collect::<Vec<_>>()
        .join(" ");
    messages.push(format!("approve: tiers: {}", tiers_str).trim_end().to_string());

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
            version: crate::version::VERSION.to_string(),
        })
        .collect();
    ledger::append(&ledger_path, &rows).map_err(|e| format!("failed to update ledger: {}", e))?;

    Ok(Outcome {
        summary: format!("removed {} entries", removed.len()),
    })
}
