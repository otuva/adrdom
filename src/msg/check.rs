//! The messages of the `check` categories.

use super::*;

// --- check/*.rs --------------------------------------------------------

pub fn check_parse_error(rel: &str, heading: &str, e: &str) -> Said {
    let found = format!("{}: {}: {}", rel, heading, e);
    if e == "duplicate id line" || e.starts_with("id line holds") {
        said(
            &rule::ID_ASSIGNED_BY_TOOL,
            found,
            Next::agent(Act::Removes("the id line".into()))
                .and_owner_runs(command(&["adr", "approve", rel])),
        )
    } else if e.starts_with("answer names missing option") {
        said(
            &rule::ANSWER_NAMES_AN_OPTION,
            found,
            Next::agent(Act::Restores("the answer line".into())),
        )
    } else {
        said(
            &rule::OPTION_IS_ONE_SENTENCE,
            found,
            Next::agent(Act::Rewrites("the option".into())),
        )
    }
}

pub fn duplicate_id(id: &str, prev: &str, rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::ID_NAMES_ONE_QUESTION,
        format!("duplicate id {} in {} and {}:{}", id, prev, rel, heading),
        Next::agent(Act::Rewrites("the copied section".into()))
            .then(Act::Removes("the id line copied with it".into()))
            .and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// An asked/refused id missing from every current topic file, struck from
/// the plan of `rel`, the file awaiting approval it was approved in: the
/// owner's approve of that file writes its withdrawn row.
pub fn removed_from_plan(id: &str, heading: &str, event: &str, rel: &str) -> Said {
    said(
        &crate::rule::ASKED_QUESTION_LEAVES_BY_APPROVAL,
        format!(
            "{} (\"{}\", {}) was removed from the plan",
            id, heading, event
        ),
        Next::owner_runs(command(&["adr", "approve", rel])),
    )
}

/// An asked/refused id whose file is removed, its removal awaiting approval:
/// `next` is the move `moves::removal_next` names for that file.
pub fn removed_with_file(id: &str, heading: &str, event: &str, next: Next) -> Said {
    said(
        &crate::rule::FILE_LEAVES_WITH_NO_LIVE_QUESTION,
        format!(
            "{} (\"{}\", {}) was removed with its file",
            id, heading, event
        ),
        next,
    )
}

/// `heading` is the heading the ledger holds for the question, from the
/// row written when it was asked.
pub fn ledger_id_not_found(id: &str, heading: Option<&str>) -> Said {
    said(
        &crate::rule::LEDGER_IDENTIFIER_IN_A_FILE,
        format!(
            "ledger id {} not found in any topic file",
            named(id, heading)
        ),
        Next::agent(Act::Restores("the question's section".into())),
    )
}

pub fn withdrawn_id_present(id: &str, heading: &str, rel: &str) -> Said {
    said(
        &crate::rule::WITHDRAWN_ID_STAYS_OUT,
        format!(
            "withdrawn id {} still present in {}",
            named(id, Some(heading)),
            rel
        ),
        Next::agent(Act::Removes(format!("the section from {}", rel))),
    )
}

pub fn refusal_without_row(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::REFUSAL_LINE_HAS_ROW,
        format!("{}: {}: refusal line without ledger row", rel, heading),
        Next::agent(Act::Removes("the answer line".into())),
    )
}

/// A hand-written `**Answer:** <letter>` line with no genuine `answered`
/// ledger row behind it at all -- distinct from `answered_text_changed`,
/// whose answer line *is* backed by a real row at that same letter, but the
/// question's own text moved since. Removing the line is only the right fix
/// here, where nothing genuine is being thrown away.
pub fn answer_without_row(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWER_LINE_HAS_ROW,
        format!("{}: {}: answer line without ledger row", rel, heading),
        Next::agent(Act::Removes("the answer line".into())),
    )
}

/// An answered section (a genuine `answered` row exists for its id, at the
/// letter its `**Answer:**` line names) whose current text no longer
/// hashes to that row: the answer is genuine, the question's text moved
/// since (per "Does check fail an answered question whose text changed?",
/// cd83e277 A). Never reported as `answer_without_row` -- that message's
/// fix, removing the answer line, would throw away a real answer over a
/// text edit; an answered question can't be re-asked either (per "Does a
/// decision change after it is answered?"), so the fix is to restore the
/// text as it was answered, from git, or to write a new question that
/// supersedes it when the change is meant.
pub fn answered_text_changed(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWERED_TEXT_FROZEN,
        format!("{}: {}: text changed since being answered", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

/// A refused section (a genuine `refused` row exists for its id) whose
/// text no longer hashes to that row: the refusal stands, the question's
/// text moved since. The refused question leaves at the next plan change,
/// so its text goes back to what was refused.
pub fn refused_text_changed(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::REFUSED_TEXT_FROZEN,
        format!("{}: {}: text changed since being refused", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

pub fn answered_row_without_line(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWER_LINE_KEPT,
        format!("{}: {}: answered row without answer line", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

pub fn refused_row_without_line(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::REFUSAL_LINE_KEPT,
        format!("{}: {}: refused row without answer line", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

/// An asked question whose text no longer hashes as asked. `report`: the
/// findings and moves of the question's file (`moves::of_unasked_question_in`), the ones
/// `show` names for it (per "Does a command compute a move of its own?",
/// aac5c313 A).
pub fn edited_after_asking(rel: &str, heading: &str, report: &crate::moves::Report) -> Said {
    said(
        &crate::rule::ASKED_TEXT_FROZEN,
        format!("{}: {}: edited after asking", rel, heading),
        report.moves_tail(Next::agent(Act::Restores(rel.to_string()))),
    )
}

pub fn answer_set_changed(rel: &str, dep_heading: &str, heading: &str) -> Said {
    said(
        &crate::rule::ANSWER_SET_FROZEN_AT_ASK,
        format!(
            "{}: {} answer set on {} changed after ask",
            rel, dep_heading, heading
        ),
        Next::agent(Act::Restores("the answer set".into())),
    )
}

/// A question whose standing ledger rulings name two or more different
/// options (per "Does check fail a question answered with two different
/// options?", e3200081-9995-4bbe-983f-2f430dc341e7): two branches each
/// answered (or one answered and one refused) the same question, and a
/// merge kept both rows. The topic file's own `**Answer:**` line only shows
/// one side, so this is caught from the ledger directly, never from the
/// file text. The move is always the same: the agent re-asks the question,
/// and the owner answers it again.
pub fn answer_conflict(rel: &str, heading: &str, id: &str, a: &str, b: &str) -> Said {
    said(
        &crate::rule::ONE_STANDING_ANSWER,
        format!(
            "{}: {}: two standing answers, {} and {}",
            rel, heading, a, b
        ),
        Next::agent_runs(command(&["adr", "ask", rel])).and_owner_runs(
            command(&["adr", "answer", &format!("{}@", id)]).then("<version> <letter>"),
        ),
    )
}

/// The `"path:line"` location a failure names, one-based.
pub fn location(name: &str, line: usize) -> String {
    format!("{}:{}", name, line)
}

/// The `" (term {})"` suffix a banned-word hit carries when the word was
/// banned with `--term`; empty when it wasn't.
pub fn term_suffix(term: Option<&str>) -> String {
    term.map(|n| format!(" (term {})", n)).unwrap_or_default()
}

pub fn banned_word_at(location: &str, suffix: &str) -> Said {
    said(
        &crate::rule::BANNED_WORD_ABSENT_FROM_FILES,
        format!("{}: banned word{}", location, suffix),
        Next::agent(Act::Rewrites("the line".into())),
    )
}

pub fn banned_missing_from_file(hash_prefix: &str) -> Said {
    said(
        &crate::rule::BAN_LIST_MATCHES_LEDGER,
        format!("{} in ledger, missing from the ban list", hash_prefix),
        Next::agent_runs(Cmd::fixed("adr ban --rebuild")),
    )
}

pub fn banned_missing_from_ledger(hash_prefix: &str) -> Said {
    said(
        &crate::rule::BAN_LIST_MATCHES_LEDGER,
        format!("{} in the ban list, no ledger row", hash_prefix),
        Next::agent_runs(Cmd::fixed("adr ban --rebuild")),
    )
}

/// A hash that stands in the file, was banned at some time, but the
/// ledger's latest row for it is `unbanned` -- the file still carries a
/// line the ledger already voided (per "Does check hold the ban list
/// equal to the ledger?"). `adr ban --rebuild` clears this: it rewrites the
/// file to exactly what the ledger currently says is banned, whatever the
/// file held before.
pub fn banned_unbanned_but_in_file(hash_prefix: &str) -> Said {
    said(
        &crate::rule::BAN_LIST_MATCHES_LEDGER,
        format!("{} in the ban list, ledger says unbanned", hash_prefix),
        Next::agent_runs(Cmd::fixed("adr ban --rebuild")),
    )
}

/// An asked/answered section that carries no `terms:` line at all. `ask`
/// always writes one, so an answered section can only lose it through an
/// edit made after the answer -- and an answered question can never be
/// re-asked (per "Does a decision change after it is answered?"), so
/// `answered` picks the legal move: re-asking while the question is still
/// only asked (`ask` will write the line the next time it runs), or
/// restoring the edit from git / superseding once it's answered (per "Does
/// check fail an answered question whose text changed?").
pub fn section_no_terms_line(rel: &str, heading: &str, answered: bool) -> Said {
    let next = if answered {
        Next::agent(Act::Restores("the terms line".into()))
    } else {
        ask_again(rel)
    };
    said(
        &crate::rule::ASKED_QUESTION_HAS_TERMS_LINE,
        format!("{}: {}: no terms line", rel, heading),
        next,
    )
}

/// approve of the terms file found an asked question whose `terms:` line
/// cannot be read, so it cannot tell which terms the question names.
pub fn asked_terms_line_unreadable(rel: &str, heading: &str) -> Said {
    said(
        &crate::rule::RETIRE_READS_ASKED_TERMS_LINES,
        format!("{}: {}: terms line cannot be read", rel, heading),
        Next::agent(Act::Restores(rel.to_string())),
    )
}

/// A section's `terms:` line is present but doesn't parse (per
/// `terms::validate_pairs`'s `Malformed`). Same answered/asked split as
/// `section_no_terms_line`.
pub fn section_terms_line_malformed(rel: &str, heading: &str, answered: bool) -> Said {
    let next = if answered {
        Next::agent(Act::Restores("the terms line".into()))
    } else {
        ask_again(rel)
    };
    said(
        &crate::rule::SECTION_TERMS_LINE_PARSES,
        format!("{}: {}: malformed terms line", rel, heading),
        next,
    )
}

/// A section's `terms:` line names a uuid that isn't in `adrdom/terms.md`
/// at all. Same split as `section_no_terms_line`: an asked section can
/// still be re-asked; an answered one is frozen, so the fix moves to
/// restoring the term in `adrdom/terms.md` (mirrors `term_removed_but_carried`)
/// or superseding the decision.
pub fn section_term_not_found(rel: &str, heading: &str, word: &str, answered: bool) -> Said {
    let next = if answered {
        Next::agent(Act::Restores("the term in adrdom/terms.md".into()))
    } else {
        ask_again(rel)
    };
    said(
        &crate::rule::ASKED_TERMS_LINE_TERM_EXISTS,
        format!("{}: {}: term {} not found", rel, heading, word),
        next,
    )
}

/// A section's `terms:` line names a term that is now retired. Same split
/// as `section_no_terms_line`/`section_term_not_found`; the answered fix
/// mirrors `term_retired_but_carried`'s decision-dependent branch.
pub fn section_term_retired(rel: &str, heading: &str, name: &str, answered: bool) -> Said {
    let next = if answered {
        Next::agent(Act::Rewrites("the term in adrdom/terms.md".into()))
    } else {
        ask_again(rel)
    };
    said(
        &crate::rule::ASKED_TERMS_LINE_TERM_IS_LIVE,
        format!("{}: {}: uses the retired term {}", rel, heading, name),
        next,
    )
}

/// A term's own `terms:` line (its definition's pairings) names a uuid
/// that no longer exists.
pub fn term_definition_term_not_found(name: &str, word: &str) -> Said {
    said(
        &crate::rule::DEFINITION_TERM_EXISTS,
        format!("adrdom/terms.md: {}: term {} not found", name, word),
        Next::agent(Act::Rewrites("the definition".into())),
    )
}

pub fn term_definition_term_retired(name: &str, retired_name: &str) -> Said {
    said(
        &crate::rule::DEFINITION_USES_LIVE_TERMS,
        format!(
            "adrdom/terms.md: {}: uses the retired term {}",
            name, retired_name
        ),
        Next::agent(Act::Rewrites("the definition".into())),
    )
}

/// A term's own `terms:` line (machine-written by `approve` from its
/// definition) doesn't parse at all (per `terms::validate_pairs`'s
/// `Malformed`) -- never an agent's line to hand-fix, since only `approve`
/// ever writes it; the move is the same re-approve that regenerates it.
pub fn term_definition_terms_line_malformed(name: &str) -> Said {
    said(
        &crate::rule::DEFINITION_TERMS_LINE_PARSES,
        format!("adrdom/terms.md: {}: malformed terms line", name),
        Next::owner_runs(Cmd::fixed("adr approve adrdom/terms.md")),
    )
}

/// `check` at a commit: the working tree holds `path` as install writes it
/// and the staged files do not; staging it is the move.
pub fn install_file_not_staged(path: &str) -> Said {
    said(
        &crate::rule::STAGED_FILES_ARE_JUDGED_FILES,
        format!("{} not staged", path),
        Next::agent_runs(command(&["git", "add", path])),
    )
}

pub fn agents_md_stale() -> Said {
    said(
        &crate::rule::AGENTS_BRIEF_MATCHES_INSTALL,
        "AGENTS.md brief differs from this binary's",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn agents_md_missing_brief() -> Said {
    said(
        &crate::rule::AGENTS_BRIEF_MATCHES_INSTALL,
        "AGENTS.md lacks the adrdom brief",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn brief_file_differs() -> Said {
    said(
        &crate::rule::AGENTS_BRIEF_MATCHES_INSTALL,
        "adrdom/state/brief.md differs from this binary's brief",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn brief_file_missing() -> Said {
    said(
        &crate::rule::AGENTS_BRIEF_MATCHES_INSTALL,
        "adrdom/state/brief.md is missing",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn claude_md_missing_import() -> Said {
    said(
        &crate::rule::AGENT_FILE_IMPORTS_AGENTS_MD,
        "CLAUDE.md does not import AGENTS.md",
        Next::agent_runs(Cmd::fixed("adr install --claude")),
    )
}

pub fn missing_deny_entry(entry: &str) -> Said {
    said(
        &crate::rule::AGENT_SETTINGS_DENY_OWNER_COMMANDS,
        format!("{} lacks deny entry {}", ".claude/settings.json", entry),
        Next::agent_runs(Cmd::fixed("adr install --claude")),
    )
}

pub fn settings_unparseable(path: &str) -> Said {
    said(
        &crate::rule::AGENT_SETTINGS_FILE_PARSES,
        format!("{} cannot be parsed", path),
        Next::agent_then_runs(
            Act::Rewrites("the file".into()),
            Cmd::fixed("adr install --claude"),
        ),
    )
}

/// `check`'s `install` check on a git hook (`.githooks/pre-commit` or
/// `.githooks/commit-msg`, per `path`): the file exists but its content
/// differs from what `adr install` writes.
pub fn hook_differs(path: &str) -> Said {
    said(
        &crate::rule::HOOK_MATCHES_INSTALL,
        format!("{} differs from what install writes", path),
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// Same check as `hook_differs`, but the hook's content is current and the
/// file is not executable, so git would skip it without a word; install
/// repairs the mode.
pub fn hook_not_executable(path: &str) -> Said {
    said(
        &crate::rule::HOOK_IS_EXECUTABLE,
        format!("{} is not executable", path),
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// Same check as `hook_differs`, but the hook file is missing outright.
pub fn hook_missing(path: &str) -> Said {
    said(
        &crate::rule::HOOK_MATCHES_INSTALL,
        format!("{} is missing", path),
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn codex_rules_stale() -> Said {
    said(
        &crate::rule::CODEX_RULES_MATCH_INSTALL,
        ".codex/rules/adrdom.rules differs from this binary's",
        Next::agent_runs(Cmd::fixed("adr install --codex")),
    )
}

pub fn codex_rules_missing() -> Said {
    said(
        &crate::rule::CODEX_RULES_MATCH_INSTALL,
        ".codex/rules/adrdom.rules missing",
        Next::agent_runs(Cmd::fixed("adr install --codex")),
    )
}

/// `check`'s `install` check on `adrdom/.gitattributes`: the file exists
/// but its content differs from what `adr install` writes.
pub fn gitattributes_differs() -> Said {
    said(
        &crate::rule::GITATTRIBUTES_MATCH_INSTALL,
        "adrdom/.gitattributes differs from what install writes",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// Same check as `gitattributes_differs`, but the file is missing
/// outright.
pub fn gitattributes_missing() -> Said {
    said(
        &crate::rule::GITATTRIBUTES_MATCH_INSTALL,
        "adrdom/.gitattributes is missing",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

pub fn malformed_ledger_row(lineno: usize, raw: &str) -> Said {
    said(
        &crate::rule::LEDGER_ROW_FIELDS,
        format!("line {}: malformed row: {}", lineno, raw),
        Next::owner(Act::Removes(format!("the row at line {}", lineno))),
    )
}

/// Per "Does check fail a ledger whose rows changed place?"
/// (c10c3a49-e993-4c9f-a46b-4a1ba3646dcf A): a HEAD row is missing or
/// changed -- a row's place in the file is free, so this never fires for
/// one that only moved.
pub fn ledger_not_append_only() -> Said {
    said(
        &crate::rule::LEDGER_ROWS_KEPT,
        "a row from HEAD's ledger is missing or changed",
        Next::owner(Act::Restores("the missing or changed row".into())),
    )
}

/// A merge (pending, via `MERGE_HEAD`, or already committed, via `HEAD`'s
/// second parent) is missing a ledger row one of the two branches wrote
/// (per "Does check at a merge look for the ledger rows of both
/// branches?"): the merge itself dropped a row instead of carrying it
/// forward, the one thing an append-only ledger can never do.
pub fn ledger_merge_drops_rows() -> Said {
    said(
        &crate::rule::MERGE_KEEPS_LEDGER_ROWS,
        "merge drops a ledger row of a merged branch",
        Next::owner(Act::Restores("the missing rows of the ledger".into())),
    )
}

/// HEAD is a merge commit, but at least one of its parents is not in this
/// repository at all -- a shallow clone, where the parent's own commit
/// object was never fetched. Distinct from a parent that is fully present
/// and simply holds no ledger file at that path (nothing to compare, no
/// failure): here there is nothing to read, so `ledger_merge_drops_rows`'s
/// own test -- every row either branch wrote survives into the merge --
/// cannot run at all, and this refuses instead of a check that never
/// looked passing anyway.
pub fn ledger_merge_parents_unreadable() -> String {
    refuse(
        "this merge commit's parents cannot be read; the clone is too shallow".to_string(),
        Next::agent_runs(Cmd::fixed("git fetch --deepen=1")),
    )
}

pub fn ledger_answered_without_asked(id: &str, heading: Option<&str>) -> Said {
    said(
        &crate::rule::ANSWER_ROW_HAS_ASKED_ROW,
        format!(
            "answered row for {} matches no asked row",
            named(id, heading)
        ),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A `dropped` row whose detail is neither `<id> <letter>` nor `<id>`.
pub fn ledger_dropped_malformed(id: &str, detail: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_ROW_NAMES_ITS_CAUSE,
        format!("dropped row for {} has the detail \"{}\", not an identifier with a letter or an identifier", id, detail),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A `dropped` row with no answer row (direct) or no dropped row (dependent) behind it.
pub fn ledger_dropped_without_cause(id: &str, cause: &str) -> Said {
    said(
        &crate::rule::DROPPED_ROW_HAS_CAUSE,
        format!("dropped row for {} has no cause: {}", id, cause),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A `dropped` row whose id and hash match no per-question approved row.
pub fn ledger_dropped_not_approved(id: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_HASH_IS_APPROVED,
        format!(
            "dropped row for {} matches no approved row of that question",
            id
        ),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A `dropped` row of a question with an earlier `answered` or `refused` row.
pub fn ledger_dropped_closed(id: &str, event: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_NOT_AFTER_CLOSE,
        format!("dropped row for {} follows its {} row", id, event),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// A dropped question that is still in a topic file.
pub fn ledger_dropped_still_in_file(id: &str, rel: &str) -> Said {
    said(
        &crate::rule::DROPPED_QUESTION_LEAVES_FILES,
        format!("{} has a dropped row but is still in {}", id, rel),
        Next::agent(Act::Removes("the question from the file".into())),
    )
}

/// A superseded question whose section is still in a topic file.
pub fn ledger_superseded_still_in_file(heading: &str, rel: &str) -> Said {
    said(
        &crate::rule::SUPERSEDED_QUESTION_LEAVES_FILES,
        format!(
            "\"{}\" is superseded but its section is still in {}",
            heading, rel
        ),
        Next::agent(Act::Removes("the question from the file".into())),
    )
}

/// At the commit that removes a dropped question: HEAD's text hashes to something else.
pub fn ledger_dropped_text_differs(id: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_HASH_IS_REMOVED_TEXT,
        format!(
            "dropped row for {} holds a hash that is not the hash of the text HEAD holds",
            id
        ),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// At the commit that removes a dropped question: HEAD's set already held the letter.
pub fn ledger_dropped_set_holds_letter(id: &str, letter: &str) -> Said {
    said(
        &crate::rule::LEDGER_DROPPED_LETTER_NOT_IN_SET,
        format!(
            "dropped row for {} names answer {}, which the set at HEAD holds",
            id, letter
        ),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// At the commit that removes a dropped dependent: HEAD's text does not stand on the named question.
pub fn ledger_dropped_not_standing_on(id: &str, on: &str) -> Said {
    said(
        &crate::rule::DROPPED_ROW_NAMES_GROUND,
        format!(
            "dropped row for {} says it stood on {}, which the question at HEAD does not",
            id, on
        ),
        Next::owner(Act::Restores("the ledger".into())),
    )
}

/// `adr check --push`, one line pre-push hands it: `local_ref`'s ledger, at
/// the sha about to be pushed, is missing a row `remote_ref` already holds
/// at its own sha (per "Does check look at the ledger before a push?",
/// 19e1c023-a678-4842-a414-bb0e45c41601 P). Same remedy as
/// `ledger_merge_drops_rows` -- the missing rows are restored, only by
/// adding them back -- since a push that drops a row is the same kind of
/// loss, just found before it reaches the remote instead of after a merge.
pub fn ledger_push_drops_rows(local_ref: &str, remote_ref: &str) -> Said {
    said(
        &crate::rule::PUSH_KEEPS_LEDGER_ROWS,
        format!(
            "push of {} would drop ledger rows the remote's {} holds",
            local_ref, remote_ref
        ),
        Next::owner(Act::Restores("the missing rows of the ledger".into())),
    )
}

/// `adr check --push`: the remote sha a pre-push line names is not a
/// commit this repository has -- a shallow clone, or a remote branch never
/// fetched -- so there is nothing local to compare it against, the same
/// blind spot `ledger_merge_parents_unreadable` refuses on for a shallow
/// merge.
pub fn ledger_push_remote_unreadable(remote_ref: &str) -> String {
    refuse(
        format!("{}'s remote commit cannot be read here", remote_ref),
        Next::agent_runs(Cmd::fixed("git fetch")),
    )
}

/// `adr check --since <commit>` (per "Does check in CI compare the ledger
/// with the commit before the push?", a26876ba-5789-4d05-965e-853d00d9edd2
/// C): the ledger being judged is missing a row `<commit>`'s own ledger
/// already holds. Same remedy as `ledger_push_drops_rows`.
pub fn ledger_since_drops_rows(commit: &str) -> Said {
    said(
        &crate::rule::SINCE_KEEPS_LEDGER_ROWS,
        format!("ledger drops a row {} already holds", commit),
        Next::owner(Act::Restores("the missing rows of the ledger".into())),
    )
}

/// `adr check --since <commit>`: `<commit>` itself is not in this
/// repository (too shallow a fetch), so its ledger cannot be read at all.
pub fn ledger_since_unreadable(commit: &str) -> String {
    refuse(
        format!("{} cannot be read here", commit),
        Next::agent_runs(Cmd::fixed("git fetch")),
    )
}

/// Per "Does check fail two ledger rows on one subject with the same
/// time?" (ce19c883-c61d-477b-a6d4-fee5134c6e9a T): two rows that are not
/// byte-identical, carry the same time, and compete for one subject --
/// `subject` already names which kind it is and the value (`"id <uuid>"`,
/// `"hash <prefix>"`, or `"file <path>"`). The remedy is always the same
/// command run again, since that writes a later row, and a later row
/// settles which one stands.
pub fn ledger_same_time_competition(time: &str, subject: &str) -> Said {
    said(
        &crate::rule::LEDGER_LATEST_ROW_IS_ONE,
        format!("two ledger rows at {} compete for {}", time, subject),
        Next::owner(Act::RunsAgain),
    )
}

/// `locations` are the `path:line` places the tag was found at; `mv` is the
/// retie move (`moves::MoveKind::Retie`) that clears it.
pub fn stale_test(id: &str, heading: Option<&str>, locations: &[String], mv: Next) -> Said {
    let at = if locations.is_empty() {
        String::new()
    } else {
        format!(", at {}", locations.join(", "))
    };
    said(
        &crate::rule::TEST_TIE_NAMES_CURRENT_DECISION,
        format!(
            "{} tied to a test and not a current decision{}",
            named(id, heading),
            at
        ),
        mv,
    )
}

/// Where a line of the brief asset sits, for a finding about it.
pub const BRIEF_ASSET: &str = "assets/brief.md";

/// A sentence line of the brief asset with no from-comment.
pub fn brief_line_unmarked(line: usize, mv: Next) -> Said {
    said(
        &crate::rule::BRIEF_SENTENCE_HAS_SOURCE,
        format!(
            "{} is a sentence with no from-comment",
            location(BRIEF_ASSET, line)
        ),
        mv,
    )
}

/// A from-comment that is empty or not the end of its line.
pub fn brief_comment_malformed(line: usize, what: &str, mv: Next) -> Said {
    said(
        &crate::rule::BRIEF_SENTENCE_HAS_SOURCE,
        format!(
            "{} holds a from-comment that {}",
            location(BRIEF_ASSET, line),
            what
        ),
        mv,
    )
}

/// A from-comment entry that is not an identifier, or names nothing.
pub fn brief_comment_unknown(line: usize, token: &str, is_id: bool, mv: Next) -> Said {
    let what = if is_id {
        "which names no decision or term"
    } else {
        "which is no identifier"
    };
    said(
        &crate::rule::BRIEF_SOURCE_NAMES_CURRENT,
        format!(
            "{} holds \"{}\" in its from-comment, {}",
            location(BRIEF_ASSET, line),
            token,
            what
        ),
        mv,
    )
}

/// A from-comment entry naming a decision that is not current, or a
/// retired term.
pub fn brief_comment_stale(
    line: usize,
    id: &str,
    name: Option<&str>,
    retired_term: bool,
    mv: Next,
) -> Said {
    let what = if retired_term {
        "a retired term"
    } else {
        "not a current decision"
    };
    said(
        &crate::rule::BRIEF_SOURCE_NAMES_CURRENT,
        format!(
            "{} names {} in its from-comment, {}",
            location(BRIEF_ASSET, line),
            named(id, name),
            what
        ),
        mv,
    )
}

pub fn duplicate_heading_in_file(heading: &str, rel: &str) -> Said {
    said(
        &rule::HEADING_ONCE_IN_FILE,
        format!("{}: the heading \"{}\" appears twice", rel, heading),
        Next::agent(Act::Rewrites("one of the headings".into())),
    )
}

pub fn duplicate_heading_across_files(heading: &str, rel1: &str, rel2: &str) -> Said {
    said(
        &crate::rule::HEADING_IN_ONE_FILE,
        format!("duplicate heading \"{}\" in {} and {}", heading, rel1, rel2),
        Next::agent(Act::Rewrites("one of the two sections".into())),
    )
}

/// The naming half of the same decision as `duplicate_heading_across_files`
/// (per "What names a topic file?"): a topic file whose own file name isn't lowercase ASCII
/// words joined by single hyphens (`repo::valid_topic_file_name`). Not part
/// of `record_refusals`: `approve` refuses it through
/// `rules::own_name_refusals` and `check` through its plan check, from
/// the one message. No `owner runs:` clause: the fixed
/// name is the agent's own choice, so there is no concrete command to name
/// yet -- the agent's move covers telling the owner what re-approval a
/// rename costs, when it costs one.
pub fn bad_topic_file_name(rel: &str, new_path: &str) -> Said {
    said(
        &crate::rule::FILE_NAME_IS_LOWERCASE_HYPHENATED,
        format!(
            "{}: file name is not lowercase words joined by single hyphens",
            rel
        ),
        Next::agent(Act::Rewrites(format!(
            "the name of {} to {}",
            rel, new_path
        )))
        .and_owner_runs(command(&["adr", "approve", new_path])),
    )
}

/// The "named by it" half of the file name rule (per "What names a topic
/// file?"): a topic file whose name is well
/// formed (`repo::valid_topic_file_name`) but is not its h1 topic text
/// lowercased and hyphenated (`repo::expected_topic_file_stem`). Read the
/// same way as `bad_topic_file_name`.
pub fn topic_file_name_mismatch(rel: &str, topic: &str) -> Said {
    said(
        &crate::rule::FILE_NAME_MATCHES_TOPIC,
        format!(
            "{}: file name does not match its topic \"{}\" in lowercase words joined by hyphens",
            rel, topic
        ),
        Next::agent(Act::Rewrites("the topic line".into()))
            .and_owner_runs(command(&["adr", "approve", rel])),
    )
}

/// The file name rule ("What names a topic file?") names a topic
/// file by its topic "in lowercase words joined by hyphens, with no number": a topic whose text lowercased and hyphenated
/// (`repo::expected_topic_file_stem`) holds a digit, or holds no letter at
/// all, has no name any file can carry, so no rename clears it and the
/// topic line itself has to change. `approve` refuses it through
/// `rules::own_name_refusals` and `check` through its plan check, from the
/// one message.
pub fn topic_unnameable(rel: &str, topic: &str) -> Said {
    said(
        &crate::rule::TOPIC_HAS_A_FILE_NAME,
        format!(
            "{}: topic \"{}\" has no file name: a name holds lowercase words only",
            rel, topic
        ),
        Next::agent(Act::Rewrites("the topic line".into()))
            .and_owner_runs(command(&["adr", "approve", rel])),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
    //
    // "every refusal names what failed, the command that clears it and who
    // runs that command". `section_term_not_found`'s answered branch used to
    // add a parenthetical suggesting the restored term be marked `retired:
    // yes` -- doing that immediately trips `section_term_retired` instead,
    // so it was never a real move. The next move is exactly the two given
    // in `section_term_retired`'s own answered branch: restore the term
    // (live), or write a question that supersedes the decision.
    #[test]
    fn section_term_not_found_answered_never_suggests_restoring_as_retired() {
        let out = section_term_not_found("x.md", "heading", "word", true).text();
        assert!(!out.contains("retired"), "{}", out);
        assert!(
            out.contains("agent restores the term in adrdom/terms.md"),
            "{}",
            out
        );
    }
}
