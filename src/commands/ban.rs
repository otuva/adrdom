use crate::banned;
use crate::commands::check::collect_targets;
use crate::ledger;
use crate::msg::{self, Actor};
use crate::out::{self, Event};
use crate::record;
use crate::repo;
use crate::rules;
use crate::rules::Command;
use crate::state;
use std::collections::HashSet;
use std::io::Read;

/// Resolves `--term <name>` to that term's uuid, matched case-insensitively
/// by name or alias. Only `state::load_terms` plus the term's `id`/`name`/
/// `aliases`/`retired` are used here, so this keeps working whatever else
/// changes in the terms file's shape. A retired term is never matched
/// (per "Does a retired term hold its words?"): this pairing sees current terms only, same as `terms::resolve`.
fn resolve_term_uuid(tf: &crate::terms::TermsFile, name: &str) -> Option<String> {
    let needle = name.trim().to_lowercase();
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        if t.name.trim().to_lowercase() == needle {
            return t.id.clone();
        }
        if t.aliases.iter().any(|a| a.trim().to_lowercase() == needle) {
            return t.id.clone();
        }
    }
    None
}

/// The arguments `ban` ran with, as a move names them: a term name of
/// several words is quoted, so the move runs as printed.
fn ban_args(term: &Option<String>, rebuild: bool) -> Vec<String> {
    match (term, rebuild) {
        (_, true) => vec!["--rebuild".to_string()],
        (Some(t), false) => vec!["--term".to_string(), t.clone()],
        (None, false) => Vec::new(),
    }
}

pub fn run(term: Option<String>, rebuild: bool) -> i32 {
    let args = ban_args(&term, rebuild);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    // The command line every move of this run names to run again, each
    // word quoted as a shell reads it.
    let mut words = vec!["ban"];
    words.extend_from_slice(&args);
    let again = crate::moves::command(&words);
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            out::eprint_stop(Command::Ban, &msg::not_a_repo("ban", &args, Actor::Agent));
            return 1;
        }
    };
    if super::refused_in_satellite(&root, Command::Ban, &args, Actor::Agent) {
        return 1;
    }
    let banned_path = root.join(banned::BAN_LIST);
    let ledger_path = root.join(repo::LEDGER_PATH);

    if rebuild {
        return run_rebuild(&banned_path, &ledger_path);
    }

    // The term's identifier, and its name as the terms file holds it (the
    // name the event line prints, whatever spelling was given).
    let (uuid, term_name): (Option<String>, Option<String>) = match &term {
        Some(name) => {
            let Some(tf) = state::load_terms(&root) else {
                out::eprint_stop(Command::Ban, &msg::no_terms_file_for_term(name));
                return 1;
            };
            match resolve_term_uuid(&tf, name) {
                Some(u) => {
                    let held = tf
                        .terms
                        .iter()
                        .find(|t| t.id.as_deref() == Some(u.as_str()))
                        .map(|t| t.name.clone());
                    (Some(u), held.or_else(|| Some(name.clone())))
                }
                None => {
                    out::eprint_stop(Command::Ban, &msg::no_such_term(name));
                    return 1;
                }
            }
        }
        None => (None, None),
    };

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        out::eprint_stop(
            Command::Ban,
            &msg::stdin_read_failed(msg::after_fix_agent_runs(again.clone())),
        );
        return 1;
    }

    // "Banned now" comes from the ledger, not the file (ce19c883-c61d-477b-
    // a6d4-fee5134c6e9a, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9): the ledger
    // is the source of what is banned, and the file is only its present
    // result. A hash the ledger already calls banned is skipped here as
    // already-banned even on a run where the file itself doesn't (yet, or
    // any more) hold it -- the file is brought back in line below either
    // way, through `banned::write_sorted` from this same set.
    let led = ledger::read(&ledger_path);
    let banned_now: Vec<(String, Option<String>)> = banned::currently_banned(&led);
    let banned_hashes: HashSet<String> = banned_now.iter().map(|(h, _)| h.clone()).collect();
    let mut new_hashes: Vec<String> = Vec::new();
    let mut seen_this_run: HashSet<String> = HashSet::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        for tok in banned::tokenize(line) {
            let h = record::sha256_hex(&tok);
            if !banned_hashes.contains(&h) && seen_this_run.insert(h.clone()) {
                new_hashes.push(h);
            }
        }
    }

    if new_hashes.is_empty() {
        // Nothing new to ban, but the file can still be stale against the
        // ledger -- a hash the ledger calls banned but the file is missing,
        // or a same-time `banned`/`unbanned` tie the ledger has since
        // settled with a later row the file never picked up. Bring it back
        // to exactly the ledger's banned set when it doesn't already match,
        // same as `--rebuild` would write it, and printed as the file it
        // wrote; left untouched (no write, no line) when it already does.
        return match bring_list_to(&banned_path, banned_now) {
            Ok(written) => {
                if written {
                    out::print_events(&[Event::FileWritten {
                        path: banned::BAN_LIST.to_string(),
                    }]);
                }
                0
            }
            Err(e) => {
                out::eprint_stop(
                    Command::Ban,
                    &msg::update_failed("banned list", e, msg::after_fix_agent_runs(again.clone())),
                );
                1
            }
        };
    }

    // Refuse before writing anything if any new word is already in use
    // anywhere the non-staged `check` scan looks. Flagged once per line,
    // even when more than one of the new words lands on it: this only
    // ever needs a yes/no per line, never `banned::hits`' per-token/term
    // detail, so a plain membership check over each line's tokens is all
    // it takes.
    let new_hash_set: HashSet<&str> = new_hashes.iter().map(|h| h.as_str()).collect();
    let mut hits: Vec<out::Line> = Vec::new();
    for (name, content) in collect_targets(&root, false, None) {
        for (i, line) in content.lines().enumerate() {
            if banned::tokenize(line)
                .iter()
                .any(|tok| new_hash_set.contains(record::sha256_hex(tok).as_str()))
            {
                hits.push(out::Line::Finding(rules::Finding::said(
                    rules::Class::Banned,
                    msg::in_use_at(&msg::location(&name, i + 1), again.clone()),
                )));
            }
        }
    }

    if !hits.is_empty() {
        let mut stop = vec![out::Line::Finding(rules::Finding::said(
            rules::Class::Banned,
            msg::already_in_use(),
        ))];
        stop.extend(hits);
        out::eprint(&stop);
        return 1;
    }

    // The ban list is written whole every time (per "Does the tool write
    // the ban list in one fixed order?"), sorted by hash: the ledger's own
    // banned set plus the ones this run adds, never an append at the end --
    // so the file ends equal to the ledger, whatever it held before this
    // run (ce19c883-c61d-477b-a6d4-fee5134c6e9a, f422c5db-13d0-4ff7-9e43-
    // b68ab44a7cb9).
    let mut entries = banned_now;
    entries.extend(new_hashes.iter().map(|h| (h.clone(), uuid.clone())));

    if let Err(e) = banned::write_sorted(&banned_path, entries) {
        out::eprint_stop(
            Command::Ban,
            &msg::update_failed("banned list", e, msg::after_fix_agent_runs(again.clone())),
        );
        return 1;
    }

    // One time for every row this run writes (per "Does a command write a
    // time later than the last ledger row?", aefa7f3c-70c3-412e-8d73-16ff9e2b703a
    // A), whichever hash each row names. Read before this run wrote the
    // ledger row below (nothing else here touches the ledger file), so the
    // earlier `led` still reflects it exactly.
    let write_time = ledger::write_time(&led);
    let rows: Vec<ledger::Row> = new_hashes
        .iter()
        .map(|h| ledger::Row {
            time: write_time.clone(),
            event: "banned".to_string(),
            id: "-".to_string(),
            hash: h.clone(),
            detail: uuid.clone().unwrap_or_default(),
            version: crate::version::VERSION.to_string(),
        })
        .collect();
    if let Err(e) = ledger::append(&ledger_path, &rows) {
        out::eprint_stop(
            Command::Ban,
            &msg::update_failed(
                crate::repo::LEDGER_PATH,
                e,
                msg::after_fix_agent_runs(again.clone()),
            ),
        );
        return 1;
    }

    // A row names a term by its identifier: the term given, when one was.
    let names: std::collections::HashMap<String, String> = uuid
        .iter()
        .zip(term_name.iter())
        .map(|(id, name)| (id.clone(), name.clone()))
        .collect();
    out::print_events(&crate::actions::Outcome::of_rows(&rows, &names).events);
    0
}

/// `adr ban --rebuild` (id f422c5db-13d0-4ff7-9e43-b68ab44a7cb9): rewrites
/// `adrdom/state/banned.sha256` as exactly the hashes the ledger currently
/// says are banned (`banned::currently_banned`, the same function `check`
/// judges the file against, so the two can never disagree), sorted through
/// `banned::write_sorted`. Reads no stdin, ignores whatever the file
/// currently holds -- conflict markers from a merge, or nothing at all --
/// and writes no ledger row: this recomputes state the ledger already
/// carries, it doesn't record a new event.
fn run_rebuild(banned_path: &std::path::Path, ledger_path: &std::path::Path) -> i32 {
    let led = ledger::read(ledger_path);
    match bring_list_to(banned_path, banned::currently_banned(&led)) {
        Ok(written) => {
            if written {
                out::print_events(&[Event::FileWritten {
                    path: banned::BAN_LIST.to_string(),
                }]);
            }
            0
        }
        Err(e) => {
            out::eprint_stop(
                Command::Ban,
                &msg::update_failed(
                    "banned list",
                    e,
                    msg::after_fix_agent_runs(crate::moves::Cmd::adr("ban --rebuild")),
                ),
            );
            1
        }
    }
}

/// Writes the ban list as exactly `entries`, sorted, unless it already
/// holds exactly that: whether it wrote. A list missing or unreadable is
/// written. The one step `ban` and `ban --rebuild` bring the list to the
/// ledger's banned set by, so both treat a missing list alike, and a list
/// already equal prints nothing (per "What does a command that writes
/// print?", 069a6b4a A: one line per file written).
fn bring_list_to(
    banned_path: &std::path::Path,
    entries: Vec<(String, Option<String>)>,
) -> std::io::Result<bool> {
    if std::fs::read_to_string(banned_path).ok() == Some(banned::sorted_content(&entries)) {
        return Ok(false);
    }
    banned::write_sorted(banned_path, entries)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
    //
    // The command line a move of ban names is the one it ran with, so it
    // runs as printed: the term and the rebuild flag kept, a term name of
    // several words quoted.
    #[test]
    fn the_arguments_a_move_names_run_as_printed() {
        let line = |term: Option<&str>, rebuild: bool| {
            let args = ban_args(&term.map(str::to_string), rebuild);
            let mut words = vec!["ban"];
            words.extend(args.iter().map(String::as_str));
            crate::moves::command(&words).to_string()
        };
        assert_eq!(line(None, false), "adr ban");
        assert_eq!(line(None, true), "adr ban --rebuild");
        assert_eq!(line(Some("widget"), false), "adr ban --term widget");
        assert_eq!(
            line(Some("answer set"), false),
            "adr ban --term 'answer set'"
        );
        assert_eq!(
            line(Some("owner's act"), false),
            "adr ban --term 'owner'\\''s act'"
        );
        assert_eq!(line(Some("a;b"), false), "adr ban --term 'a;b'");
    }
}
