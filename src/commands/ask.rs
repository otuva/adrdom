use crate::banned;
use crate::ledger;
use crate::record;
use crate::repo;
use crate::state::{self, TermsLoad};
use crate::terms;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// The first banned-word hit in `text` against the ban list, tokenizing
/// each line the way `ban`/`check` do: the word as `banned::tokenize`
/// returns it (already lowercased), and the term name when the hash's
/// entry carries a uuid that still names a current term.
fn first_banned_hit(
    text: &str,
    hash_map: &HashMap<String, Option<String>>,
    terms_list: &[terms::Term],
) -> Option<(String, Option<String>)> {
    for line in text.split('\n') {
        for tok in banned::tokenize(line) {
            let h = record::sha256_hex(&tok);
            if let Some(uuid_opt) = hash_map.get(&h) {
                let name = uuid_opt.as_ref().and_then(|u| {
                    terms_list
                        .iter()
                        .find(|t| t.id.as_deref() == Some(u.as_str()))
                        .map(|t| t.name.clone())
                });
                return Some((tok, name));
            }
        }
    }
    None
}

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

    // Refusals 1-3: adrdom/terms.md missing, unreadable/shape errors, or
    // not approved. These come before anything about the topic file itself is
    // even loaded, since none of them depend on it.
    let tf = match state::load_terms_checked(&root) {
        TermsLoad::Missing => {
            eprintln!(
                "ask: adrdom/terms.md missing; agent writes it, owner runs: adr approve adrdom/terms.md"
            );
            return 1;
        }
        TermsLoad::LoadError(e) => {
            eprintln!("ask: adrdom/terms.md: {}", e);
            return 1;
        }
        TermsLoad::Ok(tf) => {
            if !tf.errors.is_empty() {
                for e in &tf.errors {
                    eprintln!("ask: adrdom/terms.md: {}", e);
                }
                return 1;
            }
            tf
        }
    };
    if !state::terms_approved(&led, &tf) {
        eprintln!(
            "ask: {}: terms not approved; owner runs: adr approve adrdom/terms.md",
            rel
        );
        return 1;
    }
    let terms_list: Vec<terms::Term> = tf.terms.clone();

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
        .map(|e| format!("ask: {}: {}; owner fixes the topic file", rel, e))
        .collect();
    for sec in &rf.sections {
        for e in &sec.shape_errors {
            shape_msgs.push(format!(
                "ask: {}: \"{}\": {}; owner fixes the topic file",
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

    // Refusal 4: plan / context must be approved before ask will touch the
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

    // Derive-error refusal (8.4/8.7): a derivation failure means `approve`
    // would also refuse on this same file, so the fix has to happen before
    // either command can move: point the owner at the topic file first, the
    // re-approval second.
    if let Some(err) = &rf.derive_error {
        eprintln!("{}", derive_error_message(&rel, err));
        return 1;
    }

    // Cap refusal (8.8 A): any derived tier over `TIER_CAP`, file-wide
    // (mirrors approve's cap check, prefixed `ask:`).
    if let Some(tiers) = &rf.tiers {
        if let Some((n, m)) = record::over_cap(tiers) {
            eprintln!(
                "ask: {}: tier {} holds {} questions, the cap is {}; agent moves questions under a higher heading or splits the topic",
                rel, n, m, record::TIER_CAP
            );
            return 1;
        }
    }

    // The open tier (8.4 A): the lowest tier holding a non-skeleton
    // question without a letter answer (a refused question counts as
    // without one). Every candidate is non-skeleton and has no letter
    // answer by construction, so whenever any candidate exists, it names a
    // tier in this set and `open_tier` is never `None`.
    let open_tier: Option<usize> = rf
        .sections
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.is_skeleton && s.answer_letter().is_none())
        .filter_map(|(i, _)| rf.tier_of(i))
        .min();

    // Answer sets mandatory (8.7 A): before asking any candidate in the
    // open tier, every section in the file with a heading dep pointing at
    // an open-tier section must carry an explicit set on that entry, and
    // every set's letters must be options the target actually has. Runs
    // once, ahead of the per-candidate loop, and only when there is
    // something new to ask in the open tier this run.
    if let Some(n) = open_tier {
        let asking_in_open_tier = rf
            .sections
            .iter()
            .enumerate()
            .any(|(i, _)| candidate[i] && rf.tier_of(i) == Some(n));
        if asking_in_open_tier {
            for (ti, target) in rf.sections.iter().enumerate() {
                if rf.tier_of(ti) != Some(n) {
                    continue;
                }
                for (di, set) in rf.dependents_of(ti) {
                    let dependent = &rf.sections[di];
                    match set {
                        None => {
                            eprintln!(
                                "ask: {}: \"{}\" depends on \"{}\" without an answer set; agent writes =A, =A|B or =* on that depends line",
                                rel, dependent.heading_text, target.heading_text
                            );
                            return 1;
                        }
                        Some(record::AnswerSet::Any) => {}
                        Some(record::AnswerSet::Letters(letters)) => {
                            for l in letters {
                                if !target.options.iter().any(|o| o.letter == *l) {
                                    eprintln!(
                                        "ask: {}: \"{}\" depends on \"{}\" names option {} which \"{}\" does not have",
                                        rel, dependent.heading_text, target.heading_text, l, target.heading_text
                                    );
                                    return 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let banned_path = root.join("adrdom/state/banned.sha256");
    let banned_entries: HashMap<String, Option<String>> =
        banned::read_entries(&banned_path).into_iter().collect();

    for (i, sec) in rf.sections.iter().enumerate() {
        if !candidate[i] {
            continue;
        }

        // A candidate that directly stands on a refused question refuses
        // with the specific dependency named, ahead of the generic tier
        // rule below (whose "waiting on" blocker, chosen by file order
        // within the open tier, need not be this candidate's own
        // dependency even when the generic rule would also fire).
        for d in &sec.depends {
            if let record::Dep::Heading { text, .. } = d {
                if let Some(target) = rf
                    .sections
                    .iter()
                    .find(|s| s.heading_text.trim() == text.trim())
                {
                    if target.is_refused() {
                        eprintln!(
                            "ask: {}: \"{}\" stands on refused \"{}\"; agent re-plans, owner runs: adr approve {}",
                            rel, sec.heading_text, target.heading_text, rel
                        );
                        return 1;
                    }
                }
            }
        }

        // Tier rule (8.4/8.5 A): a candidate outside the open tier
        // refuses, naming the tier's blocker (the first non-skeleton,
        // unanswered section in the open tier, by file order) and whether
        // that blocker is refused.
        let cand_tier = rf.tier_of(i).unwrap_or(1);
        if let Some(open_n) = open_tier {
            if cand_tier != open_n {
                let blocker = rf.sections.iter().enumerate().find(|(bi, s)| {
                    !s.is_skeleton && s.answer_letter().is_none() && rf.tier_of(*bi) == Some(open_n)
                }).map(|(_, s)| s);
                if let Some(blocker) = blocker {
                    if blocker.is_refused() {
                        eprintln!(
                            "ask: {}: \"{}\" is in tier {}; tier {} is open, waiting on refused \"{}\"; agent re-plans, owner runs: adr approve {}",
                            rel, sec.heading_text, cand_tier, open_n, blocker.heading_text, rel
                        );
                    } else {
                        eprintln!(
                            "ask: {}: \"{}\" is in tier {}; tier {} is open, waiting on \"{}\"; owner runs: adr answer {} <letter> on it first",
                            rel,
                            sec.heading_text,
                            cand_tier,
                            open_n,
                            blocker.heading_text,
                            blocker.id.as_deref().unwrap_or("<id>")
                        );
                    }
                    return 1;
                }
            }
        }

        // Moot refusal (8.7 A): a heading dep carrying a set whose target
        // is now answered with a letter outside that set.
        for d in &sec.depends {
            if let record::Dep::Heading {
                text,
                set: Some(record::AnswerSet::Letters(letters)),
            } = d
            {
                if let Some(target) = rf
                    .sections
                    .iter()
                    .find(|s| s.heading_text.trim() == text.trim())
                {
                    if let Some(c) = target.answer_letter() {
                        if !letters.contains(&c) {
                            let set_str = canonical_set(Some(&record::AnswerSet::Letters(
                                letters.clone(),
                            )))
                            .unwrap_or_default();
                            eprintln!(
                                "ask: {}: \"{}\" is moot: stands on \"{}\"={}, answered {}; agent drops or replaces the heading, owner runs: adr approve {}",
                                rel, sec.heading_text, target.heading_text, set_str, c, rel
                            );
                            return 1;
                        }
                    }
                }
            }
        }

        let text = state::section_text_excluding_terms_line(&rf.lines, sec);

        // Refusal 5: a banned word in the candidate's own text, from the
        // single hash-based ban list scanner.
        if let Some((word, term_name)) = first_banned_hit(&text, &banned_entries, &terms_list) {
            let suffix = term_name
                .map(|n| format!(" (term {})", n))
                .unwrap_or_default();
            eprintln!(
                "ask: {}: \"{}\": banned word \"{}\"{}",
                rel, sec.heading_text, word, suffix
            );
            return 1;
        }

        let pairs = terms::resolve(&text, &terms_list);

        // Refusal 6: a retired term match.
        for (word, tid) in &pairs {
            if let Some(t) = terms_list.iter().find(|t| t.id.as_deref() == Some(tid.as_str())) {
                if t.retired {
                    eprintln!(
                        "ask: {}: \"{}\" resolves \"{}\" to retired term \"{}\"; owner edits the topic file or un-retires the term",
                        rel, sec.heading_text, word, t.name
                    );
                    return 1;
                }
            }
        }

        // Refusal 7: zero pairings.
        if pairs.is_empty() {
            eprintln!(
                "ask: {}: \"{}\" pairs no term; agent adds the term to adrdom/terms.md or rewrites the question",
                rel, sec.heading_text
            );
            return 1;
        }
    }

    // Insert missing ids, bottom-to-top so earlier insertions don't shift
    // the line numbers sections below still need to be found at.
    // Only candidates get ids: a skeleton heading stays without one (4.7).
    let needs_id: Vec<bool> = rf
        .sections
        .iter()
        .enumerate()
        .map(|(i, s)| candidate[i] && s.id.is_none())
        .collect();
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

    // Write/replace the `terms:` line directly under the `id:` line for
    // every candidate, bottom-to-top so earlier sections' line numbers are
    // unaffected by a section growing or shrinking by one line. Every
    // candidate reaching this point paired at least one term (refusal 7
    // above), so the pairings are never empty here: ask never deletes a
    // terms line, and a candidate section whose id line cannot be found is
    // an error, not a silent skip.
    let mut terms_touched = false;
    for i in (0..rf.sections.len()).rev() {
        if !candidate[i] {
            continue;
        }
        let sec = rf.sections[i].clone();
        let text = state::section_text_excluding_terms_line(&rf.lines, &sec);
        let pairs = terms::resolve(&text, &terms_list);
        let new_line = match terms::format_terms_line(&pairs) {
            Some(nl) => nl,
            None => {
                // Unreachable given refusal 7 above, but never silently
                // drop a binding if it somehow happens.
                eprintln!(
                    "ask: {}: \"{}\": pairs no term at write time; owner re-runs ask",
                    rel, sec.heading_text
                );
                return 1;
            }
        };
        match sec.terms_line {
            Some(tl) => {
                if rf.lines[tl] != new_line {
                    rf.lines[tl] = new_line;
                    terms_touched = true;
                }
            }
            None => match find_prefixed_line(&rf.lines, sec.start, sec.end, "id: ") {
                Some(idx) => {
                    rf.lines.insert(idx + 1, new_line);
                    terms_touched = true;
                }
                None => {
                    eprintln!(
                        "ask: {}: \"{}\": no id line found to write the terms line after",
                        rel, sec.heading_text
                    );
                    return 1;
                }
            },
        }
    }
    if terms_touched {
        rf.sections = record::parse_sections(&rf.lines);
    }

    let mut new_rows = Vec::new();
    let mut summary: Vec<String> = Vec::new();
    for i in 0..rf.sections.len() {
        let sec = &rf.sections[i];
        let Some(id) = sec.id.clone() else { continue };
        let cur = record::section_hash(&rf.lines, sec);
        let label = match led.latest_state(&id) {
            None => Some("asked"),
            Some(row) if row.event == "asked" && row.hash != cur => Some("re-asked"),
            _ => None,
        };
        let Some(label) = label else { continue };
        let tier = rf.tier_of(i).unwrap_or(1);
        new_rows.push(ledger::Row {
            time: ledger::now_rfc3339(),
            event: "asked".to_string(),
            id: id.clone(),
            hash: cur,
            detail: sec.heading_text.clone(),
            version: crate::version::VERSION.to_string(),
        });
        summary.push(format!("{}: \"{}\" (tier {})", label, sec.heading_text, tier));

        // One `assumes` row per dependent (8.1/8.7 A): what set the
        // dependent currently declares on this question, frozen at ask
        // time so `check`'s `answers.rs` can later catch it moving.
        for (di, set) in rf.dependents_of(i) {
            let Some(set_str) = canonical_set(set) else {
                // Guaranteed not to happen: the sets-mandatory gate above
                // already refused before any row was written if a
                // dependent in this tier carried no set. Skip rather than
                // panic if it somehow does.
                continue;
            };
            let dependent = &rf.sections[di];
            new_rows.push(ledger::Row {
                time: ledger::now_rfc3339(),
                event: "assumes".to_string(),
                id: id.clone(),
                hash: record::sha256_hex(&format!("{}={}", dependent.heading_text, set_str)),
                detail: dependent.heading_text.clone(),
                version: crate::version::VERSION.to_string(),
            });
            summary.push(format!(
                "  assumes: \"{}\"={}",
                dependent.heading_text, set_str
            ));
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

/// The set on a `depends:` entry, canonicalized for both the `assumes` row
/// hash and its printed/moot-message form: `*` for `Any`, or its letters
/// sorted and joined with `|` for `Letters` (so `=B|A` and `=A|B` hash and
/// print the same). `None` when there is no set at all.
fn canonical_set(set: Option<&record::AnswerSet>) -> Option<String> {
    match set {
        None => None,
        Some(record::AnswerSet::Any) => Some("*".to_string()),
        Some(record::AnswerSet::Letters(letters)) => {
            let mut sorted = letters.clone();
            sorted.sort_unstable();
            Some(
                sorted
                    .into_iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join("|"),
            )
        }
    }
}

/// The message for a `derive_tiers` failure (record.rs's §1), reworded for
/// `ask`: the same shape approve reports (prefix `ask:` instead of
/// `approve:`), but ending in a fix-then-approve instruction rather than
/// approve's own repair suggestion, since a derive error means the plan
/// can't be approved either until the topic file is fixed.
fn derive_error_message(rel: &str, err: &record::DeriveError) -> String {
    let tail = format!("agent fixes the topic file, owner runs: adr approve {}", rel);
    match err {
        record::DeriveError::Dangling { section, entry } => format!(
            "ask: {}: \"{}\" depends on \"{}\" which is no heading in this file; {}",
            rel, section, entry, tail
        ),
        record::DeriveError::Cycle(names) => {
            let mut chain: Vec<String> = names.iter().map(|n| format!("\"{}\"", n)).collect();
            if let Some(first) = names.first() {
                chain.push(format!("\"{}\"", first));
            }
            format!("ask: {}: cycle: {}; {}", rel, chain.join(" -> "), tail)
        }
        record::DeriveError::SelfDep(h) => {
            format!("ask: {}: \"{}\" depends on itself; {}", rel, h, tail)
        }
    }
}

/// The line index of the first line in `[start+1, end)` that starts with
/// `prefix`. Used to find the `id: ` line to write the `terms: ` line
/// directly under.
fn find_prefixed_line(lines: &[String], start: usize, end: usize, prefix: &str) -> Option<usize> {
    (start + 1..end).find(|&j| lines[j].starts_with(prefix))
}
