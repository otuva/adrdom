//! The closing lines and failures of `answer`, `ban`, `unban`, `install` and `uninstall`, the settings.json shape, the embedding model and the install of `adrdom/.gitignore`.

use super::*;

// --- install / uninstall: the closing line ---------------------------------

// --- answer.rs / ban.rs / unban.rs / install.rs / uninstall.rs -------------

/// `id_arg` is the `<id>@<version>` the owner gave.
pub fn invalid_letter(letter: &str, id_arg: &str) -> String {
    refuse(
        format!(
            "invalid letter {}; an answer is one uppercase letter, or the word refuse",
            letter
        ),
        Next::owner_runs(command(&["answer", id_arg]).then("<letter>")),
    )
}

pub fn no_such_term(name: &str) -> String {
    refuse(
        format!("no term named \"{}\" in adrdom/terms.md", name),
        Next::agent(Act::Rewrites(Part::Term)),
    )
}

/// `ban --term` with no terms file to look the name up in.
pub fn no_terms_file_for_term(name: &str) -> String {
    refuse(
        format!(
            "cannot look up \"{}\": adrdom/terms.md is missing or cannot be read",
            name
        ),
        Next::agent(Act::Writes(Part::File(crate::repo::TERMS_PATH.to_string()))),
    )
}

pub fn already_in_use() -> Said {
    said(
        &crate::rule::BAN_NOT_ON_USED_WORD,
        "refusing: already in use",
        Next::agent(Act::Removes(Part::Word)),
    )
}

/// One place a word `ban` was given already stands: its file and line,
/// with the move that clears it, as check names a banned word's place.
pub fn in_use_at(location: &str, again: Cmd) -> Said {
    said(
        &crate::rule::BAN_NOT_ON_USED_WORD,
        format!("{}: holds a word given", location),
        Next::agent_then_runs(Act::Rewrites(Part::Line), again),
    )
}

// --- settings.json shape (install --claude / check / uninstall) -----------

/// The move on a failure of an owner command the owner cannot clear by
/// running it again (a file it cannot read or write): the agent fixes what
/// the error names, then the owner runs `command`.
pub fn after_fix_owner_runs(command: Cmd) -> Next {
    Next::agent(Act::Rewrites(Part::Pending)).and_owner_runs(command)
}

/// The agent's move after a command of its own failed to read or write what
/// the error names: fix that, then run `command` again.
pub fn after_fix_agent_runs(command: Cmd) -> Next {
    Next::agent_then_runs(Act::Rewrites(Part::Pending), command)
}

/// The agent's move on a topic file `cmd` could not read at `rel`: a path
/// that does not exist is named again, any other failure is fixed first.
pub fn topic_file_unread(cmd: &str, rel: &str, e: &std::io::Error) -> Next {
    if e.kind() == std::io::ErrorKind::NotFound {
        // The file meant is not known, so the command is not named.
        Next::agent(Act::Rewrites(Part::Path))
    } else {
        after_fix_agent_runs(command(&[cmd, rel]))
    }
}

/// The move of a failure inside a step several commands share (the
/// embedding cache, the reader's file, the terms file as loaded): fix `what`,
/// then run the command that stopped, named at the head of its line, again.
pub fn fix_then_same_command(what: Part) -> Next {
    Next::agent(Act::Rewrites(what)).then(Act::RunsAgain)
}

/// The agent's move on a file a command cannot take as it stands: fix the
/// file at `path` so `so` holds, then run `command` again.
pub fn fix_then(path: &str, command: Cmd) -> Next {
    Next::agent_then_runs(Act::Rewrites(Part::File(path.to_string())), command)
}

pub fn settings_not_json_object(path: &str, next: Next) -> String {
    refuse(
        format!("{} is not a JSON object", path),
        next.on(Part::File(path.to_string())),
    )
}

pub fn settings_permissions_not_object(path: &str, next: Next) -> String {
    refuse(
        format!("{}: permissions is not an object", path),
        next.on(Part::File(path.to_string())),
    )
}

pub fn settings_deny_not_array(path: &str, next: Next) -> String {
    refuse(
        format!("{}: permissions.deny is not an array", path),
        next.on(Part::File(path.to_string())),
    )
}

// --- the embedding model and the install of adrdom/.gitignore -----------------

pub fn gitignore_differs() -> Said {
    said(
        &crate::rule::GITIGNORE_MATCHES_INSTALL,
        "adrdom/.gitignore differs from what install writes",
        Next::agent_runs(Cmd::adr("install")),
    )
}

pub fn gitignore_missing() -> Said {
    said(
        &crate::rule::GITIGNORE_MATCHES_INSTALL,
        "adrdom/.gitignore is missing",
        Next::agent_runs(Cmd::adr("install")),
    )
}

/// The embedding model is not in this build.
#[cfg(not(feature = "model"))]
pub fn model_missing() -> String {
    refuse(
        "model: the embedding model is missing".to_string(),
        Next::agent(Act::Builds(Part::Program("adr".into()))),
    )
}

/// The one step that fetches what planning needs ("Does install fetch the
/// embedding model?", e1c305af A).
#[cfg(feature = "model")]
fn install_planning() -> Next {
    Next::agent_runs(Cmd::adr("install --planning"))
}

/// The embedding model is not in the user cache directory.
#[cfg(feature = "model")]
pub fn model_not_installed(path: &str) -> String {
    refuse(
        format!(
            "model: the embedding model is not installed; looked in {}",
            path
        ),
        install_planning(),
    )
}

/// The onnxruntime library is not in the user cache directory.
#[cfg(feature = "model")]
pub fn onnxruntime_not_installed(path: &str) -> String {
    refuse(
        format!(
            "model: the onnxruntime library is not installed; looked for {}",
            path
        ),
        install_planning(),
    )
}

/// No user cache directory can be found on this machine.
#[cfg(feature = "model")]
pub fn cache_dir_missing() -> String {
    refuse(
        "model: no user cache directory is known; set HOME or XDG_CACHE_HOME".to_string(),
        Next::agent(Act::RunsAgain),
    )
}

/// The embedding model, present in the user cache directory, cannot be loaded:
/// its directory goes and the install step fetches it again.
#[cfg(feature = "model")]
pub fn model_load_failed(dir: &str, e: impl Display) -> String {
    refuse(
        format!("model: cannot load the embedding model from {}: {}", dir, e),
        Next::agent_then_runs(
            Act::Removes(Part::File(dir.to_string())),
            Cmd::adr("install --planning"),
        ),
    )
}

/// The onnxruntime library, present in the user cache directory, does not
/// load; the install step checks it and fetches it again.
#[cfg(feature = "model")]
pub fn onnxruntime_load_failed(path: &str, e: impl Display) -> String {
    refuse(
        format!("model: cannot load the onnxruntime library {}: {}", path, e),
        install_planning(),
    )
}

/// The install step found the onnxruntime library in the user cache
/// directory unloadable and removed it.
#[cfg(feature = "model")]
pub fn onnxruntime_removed(path: &str, e: impl Display) -> String {
    refuse(
        format!(
            "model: removed the onnxruntime library {}, which does not load: {}",
            path, e
        ),
        install_planning(),
    )
}

/// The onnxruntime library the install step just fetched, matching its pinned
/// hash, does not load on this machine.
#[cfg(feature = "model")]
pub fn onnxruntime_unusable(path: &str, e: impl Display) -> String {
    refuse(
        format!(
            "model: the fetched onnxruntime library {} does not load: {}",
            path, e
        ),
        Next::not_yet("an onnxruntime build this machine can load"),
    )
}

/// The onnxruntime release has no library for this platform.
#[cfg(feature = "model")]
pub fn onnxruntime_no_build(platform: &str) -> String {
    refuse(
        format!("model: onnxruntime has no build for {}", platform),
        Next::not_yet("an onnxruntime build for this platform"),
    )
}

/// Fetching the onnxruntime library or the model failed.
#[cfg(feature = "model")]
pub fn model_fetch_failed(what: &str, e: impl Display) -> String {
    refuse(
        format!("model: cannot fetch {}: {}", what, e),
        Next::agent(Act::RunsAgain),
    )
}

pub fn embed_failed(e: impl Display) -> String {
    refuse(
        format!("model: cannot embed the texts: {}", e),
        Next::agent(Act::RunsAgain),
    )
}

/// The reader is not on this machine: the program of the installed agent
/// (`claude` or `codex`) is not on PATH.
pub fn reader_missing(program: &str) -> String {
    refuse(
        format!("reader: the {} program is missing", program),
        Next::agent(Act::Installs(Part::Program(program.to_string()))),
    )
}

/// The repository was installed for no agent, so there is no program to
/// read with.
pub fn reader_no_agent() -> String {
    refuse(
        "reader: no agent is installed for this repository".to_string(),
        Next::agent(Act::Runs(Cmd::adr("install --claude"))),
    )
}

/// The reader gave no usable word for a pair of texts; nothing of that
/// call is kept.
/// The reader's own error is flattened to one line: a stack trace of
/// several lines never runs into the move.
pub fn reader_failed(neighbour: &str, e: impl Display) -> String {
    refuse(
        format!(
            "reader: no word for the neighbour {}: {}",
            neighbour,
            crate::reader::flatten(&e.to_string())
        ),
        Next::agent(Act::RunsAgain),
    )
}

pub fn reader_write_failed(e: impl Display) -> String {
    write_failed(
        crate::repo::READER_PATH,
        e,
        fix_then_same_command(Part::File(crate::repo::READER_PATH.to_string())),
    )
}

pub fn neighbours_write_failed(e: impl Display) -> String {
    write_failed(
        crate::repo::NEIGHBOURS_PATH,
        e,
        fix_then_same_command(Part::File(crate::repo::NEIGHBOURS_PATH.to_string())),
    )
}

pub fn malformed_reader_row(lineno: usize, raw: &str) -> Said {
    said(
        &crate::rule::READER_ROW_FIELDS,
        format!("line {}: malformed row in the reader file: {}", lineno, raw),
        Next::owner(Act::Removes(Part::RowAt(lineno))),
    )
}

/// A row of HEAD's reader file is not in the file byte for byte.
pub fn reader_row_edited() -> Said {
    said(
        &crate::rule::READER_ROWS_UNEDITED,
        "a row of HEAD's reader file is not in the file",
        Next::owner(Act::Restores(Part::Row)),
    )
}

pub fn reader_merge_row_edited() -> Said {
    said(
        &crate::rule::MERGE_LEAVES_READER_ROWS_UNEDITED,
        "merge changes a reader row of a merged branch",
        Next::owner(Act::Restores(Part::Row)),
    )
}

pub fn reader_push_row_edited(local_ref: &str, remote_ref: &str) -> Said {
    said(
        &crate::rule::PUSH_LEAVES_READER_ROWS_UNEDITED,
        format!(
            "push of {} would change a reader row the remote's {} holds",
            local_ref, remote_ref
        ),
        Next::owner(Act::Restores(Part::Row)),
    )
}

pub fn reader_since_row_edited(commit: &str) -> Said {
    said(
        &crate::rule::SINCE_LEAVES_READER_ROWS_UNEDITED,
        format!("reader file changes a row {} already holds", commit),
        Next::owner(Act::Restores(Part::Row)),
    )
}

/// `adr approve <path>` on a path the ledger never approved: the file was
/// never approved, so its removal needs no owner act; the agent commits the
/// deletion.
pub fn removal_not_a_topic_file(rel: &str) -> String {
    refuse(
        format!(
            "{} is absent and was never approved: its removal needs no owner act and there is nothing to approve",
            rel
        ),
        Next::agent_runs(Cmd::adr("check")),
    )
}

/// A removed topic file still held a current decision or an open question.
pub fn removal_holds_open(rel: &str, hold: &crate::moves::RemovalHold) -> Said {
    said(
        &crate::rule::FILE_REMOVAL_KEEPS_LIVE_QUESTIONS,
        format!(
            "{} cannot be removed: {} is {}",
            rel,
            named(&hold.id, Some(&hold.heading)),
            crate::moves::hold_words(hold.status)
        ),
        crate::moves::removal_next(rel, std::slice::from_ref(hold)),
    )
}

/// A commit in a planner's session that stages paths outside adrdom: the
/// first `named` paths, then "and N more".
pub fn planner_commit_outside(paths: &[String], named: usize) -> Said {
    let shown = paths
        .iter()
        .take(named)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let rest = paths.len().saturating_sub(named);
    let more = if rest > 0 {
        format!(" and {} more", rest)
    } else {
        String::new()
    };
    said(
        &crate::rule::PLANNER_COMMIT_STAYS_IN_ADRDOM,
        format!(
            "this planner's commit stages {}{} outside adrdom",
            shown, more
        ),
        Next::builder(Act::RunsAgain),
    )
}

/// The roles file of a session with an identifier cannot be read, at a commit.
pub fn planner_roles_unreadable(path: &str, err: &std::io::Error) -> Said {
    said(
        &crate::rule::PLANNER_COMMIT_STAYS_IN_ADRDOM,
        format!(
            "{} cannot be read, so the role of this session is unknown: {}",
            path, err
        ),
        Next::owner(Act::Rewrites(Part::File(path.to_string()))),
    )
}

/// The staged paths of a planner's commit could not be read from git.
pub fn planner_staged_unreadable() -> Said {
    said(
        &crate::rule::PLANNER_COMMIT_STAYS_IN_ADRDOM,
        "the staged paths of this planner's commit could not be read from git",
        Next::agent(Act::RunsAgain),
    )
}
