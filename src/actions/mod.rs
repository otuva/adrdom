//! One code path for every owner act: the CLI and the page both call these
//! (per "Does the adr binary serve the ui?").
mod answer;
mod approve;
mod approve_record;
mod approve_terms;

pub use answer::{do_answer, do_refuse};
pub use approve::do_approve;

use crate::ledger::{self, Ledger};
use crate::msg;
use crate::out::Line;
use crate::record::{self, RecordFile};
use crate::rules::{self, Class, Command, Finding};
use crate::state::{self, TermsLoad};
use crate::terms;
use std::collections::HashMap;
use std::path::Path;

/// What an owner act wrote: one event per ledger row it appended, in the
/// order written (per "What does a command that writes print?", 069a6b4a
/// A). The terminal prints each as one line; the page renders the same
/// events.
pub struct Outcome {
    pub events: Vec<crate::out::Event>,
    /// Findings on what the owner gave the act that stopped none of it
    /// (a word given to unban that was not banned), each with its move:
    /// printed apart from the events, on standard error.
    pub input_findings: Vec<Line>,
}

impl Outcome {
    /// The events of `rows`, each named in words by `names` (`event_names_in`).
    pub fn of_rows(rows: &[ledger::Row], names: &HashMap<String, String>) -> Outcome {
        Outcome {
            events: rows
                .iter()
                .filter_map(|r| crate::out::Event::from_row(r, names))
                .collect(),
            input_findings: Vec::new(),
        }
    }
}

/// What stopped an act, line by line: a finding whole as `Line::Finding`,
/// anything else as `Line::Stop` of the command that was stopped.
pub type Stops = Vec<Line>;

/// `text` as the stop of `cmd`, one line per line of `text`.
pub(super) fn stop(cmd: Command, text: impl std::fmt::Display) -> Stops {
    crate::out::stop(cmd, &text.to_string())
}

/// What a message function said of a finding, as the one line of a stop.
pub(super) fn finding(class: Class, said: msg::Said) -> Stops {
    vec![Line::Finding(Finding::said(class, said))]
}

/// `findings`, each whole.
pub(super) fn findings_of(findings: Vec<Finding>) -> Stops {
    findings.into_iter().map(Line::Finding).collect()
}

/// Every question heading of `all` and term name of `terms` by
/// identifier: what turns the identifiers of the rows an act wrote into
/// words, from what the act already holds; and for a question no topic file holds (a superseded
/// section struck) the heading its `asked` row carries.
pub fn event_names_in(
    all: &[RecordFile],
    terms: &[terms::Term],
    led: &Ledger,
) -> HashMap<String, String> {
    let mut names = crate::view::heading_names(all, led);
    names.extend(
        terms
            .iter()
            .filter_map(|t| Some((t.id.clone()?, t.name.clone()))),
    );
    names
}

/// The missing/unreadable half of the terms.md gate, shared by every
/// `require_terms` caller: missing entirely, or present but a read/shape
/// error, each refuses the same way regardless of which caller hit it. The
/// next move is the agent's either way (agents edit `adrdom/terms.md`, per
/// "ask's next step is right" -- there is no owner re-run that fixes a
/// file only an edit can fix).
fn load_terms_or_refuse(root: &Path, cmd: Command) -> Result<terms::TermsFile, Stops> {
    match state::load_terms_checked(root) {
        TermsLoad::Missing => Err(finding(Class::Terms, msg::terms_missing())),
        TermsLoad::LoadError(e) => Err(stop(
            cmd,
            msg::terms_load_error(
                &e,
                msg::fix_then_same_command("adrdom/terms.md so it can be read"),
            ),
        )),
        TermsLoad::Ok(tf) => Ok(tf),
    }
}

/// The strict terms.md gates (5): missing, present but unreadable/
/// shape-invalid, or not approved, each refusing with the same message the
/// caller would print. `rel` is the topic file's path, named in the
/// not-approved message exactly as the caller already prints it. On
/// success, the loaded, approved terms file is returned. The one gate for
/// `ask`, `answer`, `refuse`, and approve-of-a-topic-file (per "Does the
/// tool stop while the terms file waits for approval?", aa475aa3 A).
pub fn require_terms(
    root: &Path,
    led: &Ledger,
    rel: &str,
    cmd: Command,
) -> Result<terms::TermsFile, Stops> {
    let tf = load_terms_or_refuse(root, cmd)?;
    if !tf.errors.is_empty() {
        return Err(tf
            .errors
            .iter()
            .flat_map(|e| finding(Class::Terms, msg::terms_shape_error(e)))
            .collect());
    }
    if !state::terms_approved(led, &tf) {
        // What approve of the terms file would refuse comes first: the
        // owner's approve is named only when approve would take the file.
        let findings = rules::terms_file_findings(
            root,
            &record::load_all(root),
            &terms::minted_copy(&tf),
            led,
        );
        if !findings.is_empty() {
            return Err(findings_of(findings));
        }
        let waiting = state::waiting_terms(led, &tf);
        if waiting.is_empty() {
            return Err(finding(
                Class::Terms,
                msg::terms_not_approved(rel, crate::status::terms_file_status(led, &tf)),
            ));
        }
        return Err(waiting
            .iter()
            .flat_map(|(name, st)| {
                finding(Class::Terms, msg::terms_entry_waits(Some(rel), name, *st))
            })
            .collect());
    }
    Ok(tf)
}

/// Exactly one ASCII uppercase character; anything else (empty, more than
/// one character, lowercase, a digit, punctuation) is not a letter. Shared
/// by the CLI (`commands/answer.rs`) and the page (`commands/serve/mod.rs`)
/// so "answer takes that version from the terminal and the page alike"
/// extends to how the letter itself is read: the page refuses a lowercase
/// or multi-character selection the same way the CLI does.
pub fn parse_letter(s: &str) -> Option<char> {
    let mut chars = s.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if c.is_ascii_uppercase() {
        Some(c)
    } else {
        None
    }
}

pub fn do_unban(root: &Path, input: &str) -> Result<Outcome, Stops> {
    let banned_path = root.join(crate::banned::BAN_LIST);
    let ledger_path = root.join(crate::repo::LEDGER_PATH);
    let led = ledger::read(&ledger_path);
    // What is banned is what the ledger says is banned (per "Does check hold
    // the ban list equal to the ledger?"), the set `ban` reads too: a ban
    // list that drifted from it, or cannot be read, changes nothing here.
    let banned_now = crate::banned::currently_banned(&led);
    let is_banned = |h: &str| banned_now.iter().any(|(b, _)| b == h);

    // Each line given is one word (or phrase) and the hashes `ban` wrote
    // for it: the line, and each run of letters in it.
    let mut to_remove: Vec<String> = Vec::new();
    let mut not_banned = 0;
    for line in input.lines() {
        let hashes: Vec<String> = crate::banned::tokenize(line)
            .iter()
            .map(|t| record::sha256_hex(t))
            .collect();
        if hashes.is_empty() {
            continue;
        }
        let hit: Vec<String> = hashes.into_iter().filter(|h| is_banned(h)).collect();
        if hit.is_empty() {
            not_banned += 1;
        }
        for h in hit {
            if !to_remove.contains(&h) {
                to_remove.push(h);
            }
        }
    }
    to_remove.sort();
    // The words given that were not banned: a finding on the input, as a
    // count of the words given, never dropped without a word.
    let input_findings: Vec<Line> = if not_banned == 0 {
        Vec::new()
    } else {
        stop(Command::Unban, msg::unban_not_banned(not_banned))
    };

    if to_remove.is_empty() {
        return Ok(Outcome {
            events: Vec::new(),
            input_findings,
        });
    }

    // One time for every row this run writes (per "Does a command write a
    // time later than the last ledger row?", aefa7f3c-70c3-412e-8d73-16ff9e2b703a
    // A), whichever hash each row names. The ledger first: a ban list left
    // behind by a failed write after it is brought back by the move check
    // names (`adr ban --rebuild`), from the ledger.
    let write_time = ledger::write_time(&led);
    let rows: Vec<ledger::Row> = to_remove
        .iter()
        .map(|h| ledger::Row {
            time: write_time.clone(),
            event: "unbanned".to_string(),
            id: "-".to_string(),
            hash: h.clone(),
            detail: String::new(),
            version: crate::version::VERSION.to_string(),
        })
        .collect();
    ledger::append(&ledger_path, &rows).map_err(|e| {
        stop(
            Command::Unban,
            msg::update_failed(
                "ledger",
                e,
                msg::after_fix_owner_runs(crate::moves::Cmd::fixed("adr unban")),
            ),
        )
    })?;
    let remaining: Vec<(String, Option<String>)> = banned_now
        .into_iter()
        .filter(|(h, _)| !to_remove.contains(h))
        .collect();
    crate::banned::write_sorted(&banned_path, remaining).map_err(|e| {
        stop(
            Command::Unban,
            msg::update_failed(
                "banned list",
                e,
                msg::after_fix_agent_runs(crate::moves::Cmd::fixed("adr ban --rebuild")),
            ),
        )
    })?;

    let mut outcome = Outcome::of_rows(&rows, &HashMap::new());
    outcome.input_findings = input_findings;
    Ok(outcome)
}
