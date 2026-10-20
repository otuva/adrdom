use crate::banned;
use crate::commands::{deny_entries, OWNER_COMMANDS};
use crate::moves::{command, Cmd};
use crate::msg::{self, Actor};
use crate::out::{self, Event};
use crate::repo;
use crate::rules::Command;
use crate::satellite::{self, Located, Place, Row};
use std::fs;
use std::path::{Path, PathBuf};

/// The agent's next move on any I/O failure while installing: fix what the
/// error names, then run `command`, the install that failed, again.
/// The install command as it was given, flags and all.
fn install_command(claude: bool, codex: bool, planning: bool) -> Cmd {
    let words = install_words(claude, codex, planning, None);
    command(&words.iter().map(String::as_str).collect::<Vec<_>>())
}

/// The words of the install command as it was given, flags and all.
fn install_words(
    claude: bool,
    codex: bool,
    planning: bool,
    satellite: Option<&Path>,
) -> Vec<String> {
    let mut w = vec!["adr".to_string(), "install".to_string()];
    if claude {
        w.push("--claude".to_string());
    }
    if codex {
        w.push("--codex".to_string());
    }
    if planning {
        w.push("--planning".to_string());
    }
    if let Some(p) = satellite {
        w.push("--satellite".to_string());
        w.push(p.display().to_string());
    }
    w
}

/// The agent brief, final at `assets/brief.md`. `adr install` writes it
/// whole to `repo::BRIEF_PATH`; `adr check`'s `install` check compares that
/// file against this text.
pub const BRIEF: &str = include_str!("../../assets/brief.md");

/// The marker a brief sentence's trailing tie comment opens with.
pub const TIE_OPEN: &str = " <!-- from:";

/// `text` with each trailing ` <!-- from: ... -->` removed from every line
/// that ends in one; everything else, line ends included, stays byte for
/// byte (adr-code: de5ed5b1-3a83-4403-8953-941cb7e94390).
pub fn strip_ties(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fenced = false;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let ends = &line[body.len()..];
        // A fenced line is left as it is, as `check` leaves it unread.
        if body.starts_with("```") {
            fenced = !fenced;
        }
        match body.rfind(TIE_OPEN) {
            Some(at)
                if !fenced && body.ends_with("-->") && body[at..].matches("-->").count() == 1 =>
            {
                out.push_str(&body[..at]);
                out.push_str(ends);
            }
            _ => out.push_str(line),
        }
    }
    out
}

/// The brief `install` writes: `BRIEF` without its tie comments.
pub fn brief() -> String {
    strip_ties(BRIEF)
}

/// The marked section of AGENTS.md: the lines `adr install` writes between
/// the marker lines below, naming the brief's file with an @ line, the same
/// shape the brief names the terms brief with (adee66f2 A). `adr check`
/// compares AGENTS.md's marked section against this text.
pub(crate) const TAG: &str = "# adrdom\nThis repository is governed by `adr`. Read its brief before you write a question, code or a reply:\n@adrdom/state/brief.md\n";

pub(crate) const BRIEF_BEGIN: &str = "<!-- adrdom brief begin -->";
pub(crate) const BRIEF_END: &str = "<!-- adrdom brief end -->";

pub(crate) const CLAUDE_MD_MARKER: &str =
    "<!-- adrdom: the next line imports the agent brief; adr uninstall removes both lines -->";
pub(crate) const CLAUDE_MD_IMPORT: &str = "@AGENTS.md";

/// The commit hooks run check as it is by hand: a pass prints nothing, and
/// every line is a finding with its move (per "Does check print a line with no
/// move on it?", bd83aa22 A). Once check passes, each hook runs the
/// repository's own hook of the same name, `.githooks/<name>.own`, with git's
/// arguments and input, and returns its exit code
/// (adr-code: 6ac1dde1-fe11-47af-861a-334dbf917b11).
pub(crate) const PRE_COMMIT_HOOK: &str = "#!/bin/sh\n# The repository's own hook goes in .githooks/pre-commit.own and runs once adr check passes.\nadr check --staged || exit $?\nown=\"$(dirname \"$0\")/pre-commit.own\"\n[ -x \"$own\" ] || exit 0\nexec \"$own\" \"$@\"\n";
pub(crate) const COMMIT_MSG_HOOK: &str = "#!/bin/sh\n# The repository's own hook goes in .githooks/commit-msg.own and runs once adr check passes.\nadr check --message \"$1\" || exit $?\nown=\"$(dirname \"$0\")/commit-msg.own\"\n[ -x \"$own\" ] || exit 0\nexec \"$own\" \"$@\"\n";
/// Decision P (19e1c023-a678-4842-a414-bb0e45c41601): `adr check --push`
/// reads git's own pre-push input from stdin. The script reads that input
/// once, one trailing newline restored (the shell drops them all, git wrote
/// one), and hands the same bytes to check and then to the own hook.
pub(crate) const PRE_PUSH_HOOK: &str = "#!/bin/sh\n# The repository's own hook goes in .githooks/pre-push.own and runs once adr check passes.\ninput=$(cat)\n[ -z \"$input\" ] || input=\"$input\n\"\nprintf '%s' \"$input\" | adr check --push || exit $?\nown=\"$(dirname \"$0\")/pre-push.own\"\n[ -x \"$own\" ] || exit 0\nprintf '%s' \"$input\" | \"$own\" \"$@\"\n";

/// The texts earlier installs wrote for each hook: the tool's own, so a hook
/// holding one is overwritten and never moved to the own hook file.
const EARLIER_PRE_COMMIT: [&str; 2] = [
    "#!/bin/sh\nadr check --staged\nexit $?\n",
    "#!/bin/sh\nadr check --staged --quiet\nexit $?\n",
];
const EARLIER_COMMIT_MSG: [&str; 2] = [
    "#!/bin/sh\nadr check --message \"$1\"\nexit $?\n",
    "#!/bin/sh\nadr check --message \"$1\" --quiet\nexit $?\n",
];
const EARLIER_PRE_PUSH: [&str; 1] = ["#!/bin/sh\nadr check --push\nexit $?\n"];

/// `adrdom/.gitattributes`'s whole content (Decision 7,
/// db56a285-946f-48b1-b073-bb1e90192185): the one merge rule that keeps
/// both branches' rows at a merge for the ledger, and a second one for the
/// reader's file ("Does the reader's file keep a row whose text left the
/// records?", a8034401-420c-45d7-8f43-60c1b9f3baf9: a merge brings a
/// dropped row back until the next append drops it again). The ban list gets no
/// such rule -- a merge conflict there is left for the owner to resolve,
/// same as any other file `install` doesn't cover. `check`'s `install`
/// check compares the file on disk against this text the same way it
/// compares `.codex/rules/adrdom.rules` against `codex_rules_content()`.
pub(crate) const GITATTRIBUTES_CONTENT: &str =
    "state/ledger.tsv merge=union\nstate/reader.tsv merge=union\n";

/// `adrdom/.gitignore`'s whole content: the embedding index under
/// `adrdom/cache` stays out of git ("Does git track a file the tool can get
/// again without the owner?"; the model lives in the user cache directory,
/// not here), and so does the roles file, which is
/// untracked (1cec44fc), and the neighbours file (60341be0).
pub(crate) const GITIGNORE_CONTENT: &str = "cache/\nstate/roles.tsv\nstate/neighbours.tsv\n";

/// `.codex/rules/adrdom.rules`'s content: one `prefix_rule` block per
/// `OWNER_COMMANDS` entry (`commands::OWNER_COMMANDS`), `decision =
/// "forbidden"`. `check`'s `install`
/// check compares the file on disk against this text (see
/// `commands/check/install.rs`), the same way it compares AGENTS.md against
/// `TAG`.
pub(crate) fn codex_rules_content() -> String {
    let mut out = String::new();
    for cmd in OWNER_COMMANDS {
        out.push_str(&format!(
            "prefix_rule(\n    pattern = [\"adr\", \"{}\"],\n    decision = \"forbidden\",\n    justification = \"adrdom: the owner runs this\",\n)\n",
            cmd
        ));
    }
    out
}

pub fn run(claude: bool, codex: bool, planning: bool, satellite: Option<PathBuf>) -> i32 {
    let root = match satellite::locate() {
        Located::NotARepo => {
            let words = install_words(claude, codex, planning, satellite.as_deref());
            let args: Vec<&str> = words[2..].iter().map(String::as_str).collect();
            out::eprint_stop(
                Command::Install,
                &msg::not_a_repo("install", &args, Actor::Agent),
            );
            return 1;
        }
        Located::RecordMissing { primary, .. } => {
            out::eprint_stop(Command::Install, &msg::record_missing("install", &primary));
            return 1;
        }
        Located::Here(place) if place.is_satellite() => {
            return run_in_satellite(&place, claude, codex, planning, satellite.as_deref());
        }
        Located::Here(place) => place.code,
    };
    if let Some(given) = satellite {
        return run_satellite_of(&root, &given);
    }

    // One line per file written, or the failure that stopped it (per "What
    // does a command that writes print?", 069a6b4a A). The files written
    // before a failure are printed too: they are on disk.
    let mut events: Vec<Event> = Vec::new();
    let result = install_all(&root, claude, codex, planning, &mut events);
    out::print_events(&events);
    match result {
        Ok(()) => 0,
        Err(stop) => {
            out::eprint_stop(Command::Install, &stop);
            1
        }
    }
}

/// Every piece install writes, in order, each file it writes pushed onto
/// `events`; the first failure stops it, as its message.
fn install_all(
    root: &Path,
    claude: bool,
    codex: bool,
    planning: bool,
    events: &mut Vec<Event>,
) -> Result<(), String> {
    let cmd = &install_command(claude, codex, planning);
    let (hooks_state, moves) = plan_hooks(root, "", cmd)?;
    for dir in [repo::DECISIONS_DIR, repo::STATE_DIR] {
        let path = root.join(dir);
        fs::create_dir_all(&path).map_err(|e| {
            msg::create_failed(
                &path.display().to_string(),
                e,
                msg::after_fix_agent_runs(cmd.clone()),
            )
        })?;
    }

    for rel in [repo::LEDGER_PATH, repo::READER_PATH, banned::BAN_LIST] {
        let path = root.join(rel);
        events.extend(create_if_missing(root, rel).map_err(|e| {
            msg::write_failed(
                &path.display().to_string(),
                e,
                msg::after_fix_agent_runs(cmd.clone()),
            )
        })?);
    }

    events.extend(install_gitattributes(root).map_err(|e| {
        msg::write_failed(
            repo::GITATTRIBUTES_PATH,
            e,
            msg::after_fix_agent_runs(cmd.clone()),
        )
    })?);
    events.extend(install_gitignore(root).map_err(|e| {
        msg::write_failed(
            repo::GITIGNORE_PATH,
            e,
            msg::after_fix_agent_runs(cmd.clone()),
        )
    })?);
    events.extend(install_brief(root).map_err(|e| {
        msg::write_failed(repo::BRIEF_PATH, e, msg::after_fix_agent_runs(cmd.clone()))
    })?);

    let hooks = root.join(".githooks");
    fs::create_dir_all(&hooks).map_err(|e| {
        msg::create_failed(
            &hooks.display().to_string(),
            e,
            msg::after_fix_agent_runs(cmd.clone()),
        )
    })?;
    for (rel, _, content, _) in HOOKS {
        move_planned(&moves, rel, "", events, cmd)?;
        events.extend(
            create_hook(root, rel, content)
                .map_err(|e| msg::write_failed(rel, e, msg::after_fix_agent_runs(cmd.clone())))?,
        );
    }

    // Without this git runs none of the hooks written above. A change of
    // git's configuration is a write: one line, when it changes.
    if hooks_state == repo::HooksPath::Unset {
        if repo::git(root, &["config", "core.hooksPath", ".githooks"]).is_none() {
            return Err(msg::hooks_path_failed(
                "set",
                msg::after_fix_agent_runs(cmd.clone()),
            ));
        }
        events.push(Event::ConfigSet {
            key: "core.hooksPath".to_string(),
            value: ".githooks".to_string(),
        });
    }

    events.extend(
        install_agents_md(root).map_err(|e| {
            msg::update_failed("AGENTS.md", e, msg::after_fix_agent_runs(cmd.clone()))
        })?,
    );
    events.extend(
        install_claude_md(root, claude).map_err(|e| {
            msg::update_failed("CLAUDE.md", e, msg::after_fix_agent_runs(cmd.clone()))
        })?,
    );

    if claude {
        events.extend(merge_settings(root, cmd)?);
    }

    if codex {
        events.extend(install_codex_rules(root).map_err(|e| {
            msg::write_failed(
                ".codex/rules/adrdom.rules",
                e,
                msg::after_fix_agent_runs(cmd.clone()),
            )
        })?);
    }

    // The planning step runs last: the repository's own files are written
    // whether or not the fetch succeeds ("Does install fetch the embedding
    // model?", e1c305af A).
    if planning {
        #[cfg(feature = "model")]
        crate::planning::run(events)?;
        #[cfg(not(feature = "model"))]
        return Err(msg::model_missing());
    }
    Ok(())
}

/// The three hooks install writes: path under the root, name, current text,
/// and the texts earlier installs wrote.
const HOOKS: [(&str, &str, &str, &[&str]); 3] = [
    (
        repo::PRE_COMMIT_HOOK_PATH,
        "pre-commit",
        PRE_COMMIT_HOOK,
        &EARLIER_PRE_COMMIT,
    ),
    (
        repo::COMMIT_MSG_HOOK_PATH,
        "commit-msg",
        COMMIT_MSG_HOOK,
        &EARLIER_COMMIT_MSG,
    ),
    (
        repo::PRE_PUSH_HOOK_PATH,
        "pre-push",
        PRE_PUSH_HOOK,
        &EARLIER_PRE_PUSH,
    ),
];

/// A hook install moves into the repository's own hook file.
struct HookMove {
    from: PathBuf,
    /// The source as events and refusals name it.
    from_shown: String,
    /// Whether the source is in `.githooks`, where the tool's hook is written
    /// over its place at once.
    in_githooks: bool,
    own: PathBuf,
    own_shown: String,
    /// Where the moved symlink points, when its target is relative: written
    /// so the link resolves to the file it resolved to before the move.
    link_to: Option<PathBuf>,
}

/// Whether `path` is a regular file, or a symlink to one: the only entries
/// that are hooks.
fn is_hook_file(path: &Path) -> bool {
    fs::metadata(path).map(|m| m.is_file()).unwrap_or(false)
}

/// Whether the hook file at `path` holds a text this tool wrote, now or in an
/// earlier install. A checkout's carriage returns aside: such a copy is still
/// the tool's, rewritten in place, never moved.
fn is_tools_hook(path: &Path, current: &str, earlier: &[&str]) -> bool {
    let Ok(existing) = fs::read_to_string(path) else {
        return false;
    };
    let existing = existing.replace("\r\n", "\n");
    hook_is_current(&existing, current) || earlier.iter().any(|t| hook_is_current(&existing, t))
}

/// What install checks before it writes anything, from `git config
/// core.hooksPath` and the hook files: the stop when another program holds
/// hooksPath (897b52a7-f6d2-467e-ba14-9f4ce4865664), and the hooks to move
/// into their own hook files (60cfa0ba-9085-43b8-9ed4-fd94808c33e0) -- a
/// `.githooks` hook that is not the tool's, and, while hooksPath is unset and
/// git ran `.git/hooks`, the hook there. A move onto a file that is there, or
/// onto a file another move fills, stops install with nothing written.
/// `shown` leads the paths under `root` in what is printed.
fn plan_hooks(
    root: &Path,
    shown: &str,
    install: &Cmd,
) -> Result<(repo::HooksPath, Vec<HookMove>), String> {
    let state = repo::hooks_path_state(root);
    if let repo::HooksPath::Foreign(v) = &state {
        return Err(msg::hooks_path_foreign(v, install.clone()));
    }
    let git_dir = match state {
        repo::HooksPath::Unset => repo::git_hooks_dir(root),
        _ => None,
    };
    let mut moves = Vec::new();
    for (rel, name, current, earlier) in HOOKS {
        let own_rel = format!("{}.own", rel);
        let own = root.join(&own_rel);
        let own_shown = format!("{}{}", shown, own_rel);
        let ours = root.join(rel);
        let mut sources: Vec<(PathBuf, String, bool)> = Vec::new();
        if is_hook_file(&ours) && !is_tools_hook(&ours, current, earlier) {
            sources.push((ours, format!("{}{}", shown, rel), true));
        }
        if let Some(dir) = &git_dir {
            let theirs = dir.join(name);
            if is_hook_file(&theirs) && !is_tools_hook(&theirs, current, earlier) {
                let shown_path = match theirs.strip_prefix(root) {
                    Ok(r) => format!("{}{}", shown, r.display()),
                    Err(_) => theirs.display().to_string(),
                };
                sources.push((theirs, shown_path, false));
            }
        }
        if sources.is_empty() {
            continue;
        }
        if fs::symlink_metadata(&own).is_ok() {
            return Err(msg::hook_move_blocked(
                &sources[0].1,
                &own_shown,
                install.clone(),
            ));
        }
        if sources.len() > 1 {
            return Err(msg::hook_move_twice(
                &sources[0].1,
                &sources[1].1,
                &own_shown,
                install.clone(),
            ));
        }
        let (from, from_shown, in_githooks) = sources.remove(0);
        moves.push(HookMove {
            from,
            from_shown,
            in_githooks,
            own,
            own_shown,
            link_to: None,
        });
    }
    retarget_links(root, &mut moves);
    Ok((state, moves))
}

/// Gives each moved symlink with a relative target the path that resolves,
/// from the own hook file's directory, to the file it resolved to before: the
/// own file of another hook this install moves, or the resolved file itself.
fn retarget_links(root: &Path, moves: &mut [HookMove]) {
    if cfg!(not(unix)) {
        return;
    }
    let Ok(canon_root) = fs::canonicalize(root) else {
        return;
    };
    let own_dir = canon_root.join(".githooks");
    // Each move's source as a path with its directory resolved.
    let sources: Vec<Option<PathBuf>> = moves
        .iter()
        .map(|m| {
            let parent = fs::canonicalize(m.from.parent()?).ok()?;
            Some(parent.join(m.from.file_name()?))
        })
        .collect();
    let mut targets: Vec<Option<PathBuf>> = Vec::new();
    for m in moves.iter() {
        let target = match fs::read_link(&m.from) {
            Ok(t) if t.is_relative() => t,
            _ => {
                targets.push(None);
                continue;
            }
        };
        let direct = m
            .from
            .parent()
            .and_then(|p| fs::canonicalize(p).ok())
            .map(|p| normalized(&p.join(&target)));
        let hook = direct.and_then(|d| {
            sources
                .iter()
                .position(|s| s.as_deref() == Some(d.as_path()))
        });
        let to = match hook {
            Some(i) => moves[i]
                .own
                .file_name()
                .map(|n| PathBuf::from(n.to_os_string())),
            None => fs::canonicalize(&m.from)
                .ok()
                .map(|r| PathBuf::from(satellite::relative_path(&own_dir, &r))),
        };
        targets.push(to);
    }
    for (m, t) in moves.iter_mut().zip(targets) {
        m.link_to = t;
    }
}

/// `path` with its `.` and `..` components folded, without touching disk.
fn normalized(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Moves a hook to its own hook file, keeping its mode: a rename, or a copy
/// and a removal when the rename crosses devices.
fn move_hook(m: &HookMove, events: &mut Vec<Event>, again: &Cmd) -> Result<(), String> {
    let failed = |e: std::io::Error| {
        msg::write_failed(&m.own_shown, e, msg::after_fix_agent_runs(again.clone()))
    };
    match &m.link_to {
        Some(to) => {
            link_to(to, &m.own).map_err(failed)?;
            if let Err(e) = fs::remove_file(&m.from) {
                let _ = fs::remove_file(&m.own);
                return Err(failed(e));
            }
        }
        None => {
            if fs::rename(&m.from, &m.own).is_err() {
                fs::copy(&m.from, &m.own).map_err(failed)?;
                if let Err(e) = fs::remove_file(&m.from) {
                    // No half-done move: a rerun would refuse on the copy.
                    let _ = fs::remove_file(&m.own);
                    return Err(failed(e));
                }
            }
        }
    }
    events.push(Event::FileWritten {
        path: m.own_shown.clone(),
    });
    if !m.in_githooks {
        events.push(Event::FileRemoved {
            path: m.from_shown.clone(),
        });
    }
    Ok(())
}

/// A symlink at `at` pointing to `to` (unix; elsewhere no link is retargeted).
fn link_to(to: &Path, at: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(to, at)
    }
    #[cfg(not(unix))]
    {
        let _ = (to, at);
        Ok(())
    }
}

/// Moves the hook found for `rel`, if one was planned.
fn move_planned(
    moves: &[HookMove],
    rel: &str,
    shown: &str,
    events: &mut Vec<Event>,
    again: &Cmd,
) -> Result<(), String> {
    let own_shown = format!("{}{}.own", shown, rel);
    match moves.iter().find(|m| m.own_shown == own_shown) {
        Some(m) => move_hook(m, events, again),
        None => Ok(()),
    }
}

/// The event of a file written at `rel`.
fn written(rel: &str) -> Option<Event> {
    Some(Event::FileWritten {
        path: rel.to_string(),
    })
}

/// Creates the empty file at `rel` when it is missing.
fn create_if_missing(root: &Path, rel: &str) -> Result<Option<Event>, String> {
    let path = root.join(rel);
    if path.exists() {
        return Ok(None);
    }
    fs::write(&path, "").map_err(|e| e.to_string())?;
    Ok(written(rel))
}

/// Writes/refreshes `rel` (a hook path relative to `root`, e.g.
/// `repo::PRE_COMMIT_HOOK_PATH`) with `content`: writes it when missing,
/// rewrites it when the content on disk differs, and leaves it untouched
/// (no event) when it already matches -- the same missing/stale/current
/// shape as `install_agents_md`. "Matches" is `text::lines_match_trimmed`,
/// the same comparison `check`'s `check_hook` judges this same file with,
/// so this call and `adr check` can never disagree about whether the file
/// on disk is already current.
fn create_hook(root: &Path, rel: &str, content: &str) -> Result<Option<Event>, String> {
    let path = root.join(rel);
    if let Ok(existing) = fs::read_to_string(&path) {
        if hook_is_current(&existing, content) {
            // Content current, but git runs a hook only when it is
            // executable (`repo::hook_is_executable`, the same test
            // `check` applies): repair the mode rather than leave a hook
            // git would silently skip.
            if repo::hook_is_executable(&path) {
                return Ok(None);
            }
            make_executable(&path)?;
            return Ok(written(rel));
        }
    }
    fs::write(&path, content).map_err(|e| e.to_string())?;
    make_executable(&path)?;
    Ok(written(rel))
}

/// Sets the mode git requires to run a hook (0o755 on unix; a no-op
/// elsewhere, where `repo::hook_is_executable` is always true).
fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = fs::metadata(path).map_err(|e| e.to_string())?;
        let mut perms = meta.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).map_err(|e| e.to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// Writes/refreshes `adrdom/.gitattributes`: written when missing or when
/// its content differs from `GITATTRIBUTES_CONTENT`, left untouched
/// (no event) when it already holds exactly that text -- the same
/// missing/stale/current shape `install_codex_rules` already uses for its
/// own whole, tool-owned file (there is no marked section inside this one
/// to preserve, unlike AGENTS.md/CLAUDE.md).
fn install_gitattributes(root: &Path) -> Result<Option<Event>, String> {
    write_whole(root, repo::GITATTRIBUTES_PATH, GITATTRIBUTES_CONTENT)
}

/// Writes/refreshes `adrdom/.gitignore`, the same way as `install_gitattributes`.
fn install_gitignore(root: &Path) -> Result<Option<Event>, String> {
    write_whole(root, repo::GITIGNORE_PATH, GITIGNORE_CONTENT)
}

/// Writes `content` as the whole file at `rel` unless it already holds
/// exactly that text.
fn write_whole(root: &Path, rel: &str, content: &str) -> Result<Option<Event>, String> {
    let path = root.join(rel);
    if fs::read_to_string(&path).is_ok_and(|c| c == content) {
        return Ok(None);
    }
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(written(rel))
}

/// Whether a brief file holding `existing` is the brief install writes: the
/// same lines, a checkout's `\r` aside. The one comparison `install` and
/// `check` judge the brief's file by.
pub(crate) fn brief_is_current(existing: &str) -> bool {
    let existing_lines: Vec<&str> = existing.lines().collect();
    let want = brief();
    let want_lines: Vec<&str> = want.lines().collect();
    crate::text::lines_match_trimmed(&existing_lines, &want_lines)
}

/// Writes the brief to `repo::BRIEF_PATH` unless the file there is current.
fn install_brief(root: &Path) -> Result<Option<Event>, String> {
    let path = root.join(repo::BRIEF_PATH);
    if fs::read_to_string(&path).is_ok_and(|c| brief_is_current(&c)) {
        return Ok(None);
    }
    fs::write(&path, brief()).map_err(|e| e.to_string())?;
    Ok(written(repo::BRIEF_PATH))
}

/// Locates the marker lines in `lines` (as produced by `content.split('\n')`,
/// so indices line up with a raw reconstruction). Both markers must match
/// exactly (no trimming), and the end marker must come after the begin
/// marker.
pub(crate) fn find_markers(lines: &[&str]) -> Option<(usize, usize)> {
    // A marker line saved with a carriage return (a file with CRLF line
    // ends) is the same marker: install, check and uninstall read it alike.
    let is = |l: &str, marker: &str| l.trim_end_matches('\r') == marker;
    let begin = lines.iter().position(|l| is(l, BRIEF_BEGIN))?;
    let end = lines[begin + 1..]
        .iter()
        .position(|l| is(l, BRIEF_END))
        .map(|i| i + begin + 1)?;
    Some((begin, end))
}

/// Whether a hook file holding `existing` is the hook `content` install
/// writes: the same lines, and no carriage return, since `sh` reads a CRLF
/// hook's first line as a command that is not there and git cannot run it.
/// The one comparison `install` and `check` judge a hook by.
pub(crate) fn hook_is_current(existing: &str, content: &str) -> bool {
    let existing_lines: Vec<&str> = existing.split('\n').collect();
    let want_lines: Vec<&str> = content.split('\n').collect();
    !existing.contains('\r') && crate::text::lines_match_trimmed(&existing_lines, &want_lines)
}

/// The marker lines plus the tag's own lines, in file order, as they
/// should appear in AGENTS.md (begin marker, then every tag line, then
/// end marker).
fn section_lines() -> Vec<&'static str> {
    let mut v = vec![BRIEF_BEGIN];
    v.extend(TAG.lines());
    v.push(BRIEF_END);
    v
}

/// Writes/refreshes the marked brief section in AGENTS.md at the repo
/// root: creates the file holding only the section when it's missing,
/// appends (after a blank line) when the file exists without markers, or
/// replaces the marked span in place when it already has one. Never
/// touches text outside the markers, and a no-op leaves the file
/// byte-identical.
fn install_agents_md(root: &Path) -> Result<Option<Event>, String> {
    let path = root.join("AGENTS.md");

    if !path.exists() {
        let content = section_lines().join("\n") + "\n";
        fs::write(&path, &content).map_err(|e| e.to_string())?;
        return Ok(written("AGENTS.md"));
    }

    let existing = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let parts: Vec<&str> = existing.split('\n').collect();

    match find_markers(&parts) {
        Some((begin, end)) => {
            let mut new_parts: Vec<&str> = Vec::new();
            new_parts.extend_from_slice(&parts[..begin]);
            new_parts.extend(section_lines());
            new_parts.extend_from_slice(&parts[end + 1..]);
            let new_content = new_parts.join("\n");

            if new_content == existing {
                return Ok(None);
            }
            fs::write(&path, &new_content).map_err(|e| e.to_string())?;
            Ok(written("AGENTS.md"))
        }
        None => {
            let mut new_content = existing;
            if !new_content.is_empty() {
                if !new_content.ends_with('\n') {
                    new_content.push('\n');
                }
                if !new_content.ends_with("\n\n") {
                    new_content.push('\n');
                }
            }
            new_content.push_str(&section_lines().join("\n"));
            new_content.push('\n');
            fs::write(&path, &new_content).map_err(|e| e.to_string())?;
            Ok(written("AGENTS.md"))
        }
    }
}

/// Adds the `@AGENTS.md` import to CLAUDE.md at the repo root, but only
/// with `--claude`: without the flag, CLAUDE.md is untouched whether it's
/// missing or already exists (4f925b13-ad16-48b8-acd1-3e30d6e308b6,
/// answer A -- nothing for one agent without a flag naming it). With
/// `--claude`, a missing CLAUDE.md is created holding only the marker and
/// the import; an existing one gets the import appended unless it already
/// has a line equal to `@AGENTS.md`.
fn install_claude_md(root: &Path, create: bool) -> Result<Option<Event>, String> {
    if !create {
        return Ok(None);
    }
    let path = root.join("CLAUDE.md");
    let created = !path.exists();

    let existing = if created {
        String::new()
    } else {
        fs::read_to_string(&path).map_err(|e| e.to_string())?
    };
    if existing.lines().any(|l| l == CLAUDE_MD_IMPORT) {
        return Ok(None);
    }

    let mut new_content = existing;
    if !new_content.is_empty() {
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        if !new_content.ends_with("\n\n") {
            new_content.push('\n');
        }
    }
    new_content.push_str(CLAUDE_MD_MARKER);
    new_content.push('\n');
    new_content.push_str(CLAUDE_MD_IMPORT);
    new_content.push('\n');

    fs::write(&path, &new_content).map_err(|e| e.to_string())?;
    Ok(written("CLAUDE.md"))
}

/// Merges `deny_entries()` into `.claude/settings.json`'s
/// `permissions.deny` array, preserving every other key and array entry.
/// Creates the directory/file if missing; writes the file only when an
/// entry is missing from it. A failure is the whole refusal, its move once:
/// a file the agent fixes so it holds what install merges into, or the
/// error to fix, then `cmd` again.
fn merge_settings(root: &Path, cmd: &Cmd) -> Result<Option<Event>, String> {
    const REL: &str = ".claude/settings.json";
    let failed = |e: String| msg::update_failed(REL, e, msg::after_fix_agent_runs(cmd.clone()));
    let fix = || msg::fix_then(REL, cmd.clone());
    let dir = root.join(".claude");
    fs::create_dir_all(&dir).map_err(|e| failed(e.to_string()))?;
    let path = dir.join("settings.json");

    let mut value: serde_json::Value = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| failed(e.to_string()))?;
        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&content).map_err(|e| msg::parse_failed(REL, e, fix()))?
        }
    } else {
        serde_json::json!({})
    };

    if !value.is_object() {
        return Err(msg::settings_not_json_object(REL, fix()));
    }
    let obj = value.as_object_mut().unwrap();

    let permissions = obj
        .entry("permissions")
        .or_insert_with(|| serde_json::json!({}));
    if !permissions.is_object() {
        return Err(msg::settings_permissions_not_object(REL, fix()));
    }
    let permissions_obj = permissions.as_object_mut().unwrap();

    let deny = permissions_obj
        .entry("deny")
        .or_insert_with(|| serde_json::json!([]));
    if !deny.is_array() {
        return Err(msg::settings_deny_not_array(REL, fix()));
    }
    let deny_arr = deny.as_array_mut().unwrap();

    let mut added = false;
    for entry in deny_entries() {
        if !deny_arr.iter().any(|v| v.as_str() == Some(entry.as_str())) {
            deny_arr.push(serde_json::Value::String(entry.clone()));
            added = true;
        }
    }
    if !added && path.exists() {
        return Ok(None);
    }

    let mut pretty = serde_json::to_string_pretty(&value).map_err(|e| failed(e.to_string()))?;
    pretty.push('\n');
    fs::write(&path, pretty).map_err(|e| failed(e.to_string()))?;

    Ok(written(REL))
}

/// Writes `.codex/rules/adrdom.rules` (creating `.codex/rules/`). Codex
/// only enforces project rules for a project the owner has marked trusted
/// in their own Codex config, which adrdom never touches.
fn install_codex_rules(root: &Path) -> Result<Option<Event>, String> {
    fs::create_dir_all(root.join(".codex/rules")).map_err(|e| e.to_string())?;
    write_whole(root, ".codex/rules/adrdom.rules", &codex_rules_content())
}

/// The satellite files, written in the satellite at `sat` from the primary's
/// files at `primary`: the primary's path, byte copies of the brief and the
/// terms brief, the hooks and the marked section of AGENTS.md (per "What
/// does install write in a satellite?"). Nothing else is written there.
/// `shown` leads each file's event: the satellite's path from where the
/// command runs, or nothing in the satellite itself.
fn install_satellite_files(
    sat: &Path,
    primary: &Path,
    primary_line: &str,
    shown: &str,
    cmd: Cmd,
    events: &mut Vec<Event>,
) -> Result<(), String> {
    let again = || msg::after_fix_agent_runs(cmd.clone());
    let at = |rel: &str| format!("{}{}", shown, rel);
    let lead = |ev: Option<Event>| match ev {
        Some(Event::FileWritten { path }) => Some(Event::FileWritten { path: at(&path) }),
        other => other,
    };
    let (hooks_state, moves) = plan_hooks(sat, shown, &cmd)?;
    let state = sat.join(repo::STATE_DIR);
    fs::create_dir_all(&state)
        .map_err(|e| msg::create_failed(&state.display().to_string(), e, again()))?;

    events.extend(
        write_bytes(
            sat,
            repo::PRIMARY_PATH,
            format!("{}\n", primary_line).as_bytes(),
        )
        .map_err(|e| msg::write_failed(&at(repo::PRIMARY_PATH), e, again()))?
        .map(|_| Event::FileWritten {
            path: at(repo::PRIMARY_PATH),
        }),
    );
    for (rel, optional) in [(repo::BRIEF_PATH, false), (repo::TERMS_BRIEF_PATH, true)] {
        let bytes = match fs::read(primary.join(rel)) {
            Ok(b) => b,
            Err(e) if optional && e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(msg::read_failed(
                    &primary.join(rel).display().to_string(),
                    e,
                    again(),
                ))
            }
        };
        events.extend(
            write_bytes(sat, rel, &bytes)
                .map_err(|e| msg::write_failed(&at(rel), e, again()))?
                .map(|_| Event::FileWritten { path: at(rel) }),
        );
    }

    let hooks = sat.join(".githooks");
    fs::create_dir_all(&hooks)
        .map_err(|e| msg::create_failed(&hooks.display().to_string(), e, again()))?;
    for (rel, _, content, _) in HOOKS {
        move_planned(&moves, rel, shown, events, &cmd)?;
        events.extend(lead(
            create_hook(sat, rel, content).map_err(|e| msg::write_failed(&at(rel), e, again()))?,
        ));
    }
    if hooks_state == repo::HooksPath::Unset {
        if repo::git(sat, &["config", "core.hooksPath", ".githooks"]).is_none() {
            return Err(msg::hooks_path_failed("set", again()));
        }
        events.push(Event::ConfigSet {
            key: "core.hooksPath".to_string(),
            value: ".githooks".to_string(),
        });
    }
    events.extend(lead(
        install_agents_md(sat).map_err(|e| msg::update_failed(&at("AGENTS.md"), e, again()))?,
    ));
    Ok(())
}

/// Writes `bytes` at `rel` under `root` unless the file already holds them.
fn write_bytes(root: &Path, rel: &str, bytes: &[u8]) -> Result<Option<()>, String> {
    let path = root.join(rel);
    if fs::read(&path).is_ok_and(|c| c == bytes) {
        return Ok(None);
    }
    fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(Some(()))
}

/// `adr install --satellite <given>` in the primary at `root` (per "Does
/// install take the path of a satellite?"): the satellite's files, then its
/// row in the primary's list.
fn run_satellite_of(root: &Path, given: &Path) -> i32 {
    let shown = given.display().to_string();
    let refuse = |text: String| {
        out::eprint_stop(Command::Install, &text);
        1
    };
    let sat = match satellite::absolute(given) {
        Ok(p) if p.join(".git").exists() => p,
        _ => return refuse(msg::satellite_not_a_repo(&shown)),
    };
    let primary = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    if sat == primary || satellite::has_record(&sat) {
        return refuse(msg::satellite_holds_record(&shown));
    }
    if !satellite::has_record(&primary) {
        return refuse(msg::satellite_without_record(&shown));
    }
    if let Some(other) = satellite::held_by_another(&sat, &primary) {
        return refuse(msg::satellite_of_another(
            &shown,
            &sat.display().to_string(),
            &other.display().to_string(),
        ));
    }

    // Per "What does a path under adrdom to another repository resolve
    // against?" (0320a987-f8b2-4105-9b27-64bf484026e6 A): both paths run
    // between the main working trees; the files are written in the
    // checkouts, and their events are shown from the one the command runs in.
    let to_primary = satellite::relative_path(&repo::main_root(&sat), &repo::main_root(&primary));
    let to_sat = satellite::relative_path(&repo::main_root(&primary), &repo::main_root(&sat));
    let shown_from = format!("{}/", satellite::relative_path(&primary, &sat));
    let mut events: Vec<Event> = Vec::new();
    let again = command(&["adr", "install", "--satellite", &shown]);
    let mut result = install_satellite_files(
        &sat,
        &primary,
        &to_primary,
        &shown_from,
        again.clone(),
        &mut events,
    );
    if result.is_ok() {
        let url = repo::git(&sat, &["remote", "get-url", "origin"])
            .map(|u| satellite::without_userinfo(u.trim()))
            .unwrap_or_default();
        result = satellite::write_row(&primary, Row { path: to_sat, url })
            .map(|ev| events.extend(ev))
            .map_err(|e| {
                msg::write_failed(repo::SATELLITES_PATH, e, msg::after_fix_agent_runs(again))
            });
    }
    out::print_events(&events);
    match result {
        Ok(()) => 0,
        Err(stop) => {
            out::eprint_stop(Command::Install, &stop);
            1
        }
    }
}

/// `adr install` in a satellite: its files again from the primary's; a flag
/// of the primary's install is refused, and no row is touched.
fn run_in_satellite(
    place: &Place,
    claude: bool,
    codex: bool,
    planning: bool,
    satellite_arg: Option<&Path>,
) -> i32 {
    let primary = place.primary.clone().unwrap_or_default();
    if let Some(given) = satellite_arg {
        // The path as the primary's root reads it, since that is where the
        // command runs.
        let Ok(abs) = satellite::absolute(given) else {
            out::eprint_stop(
                Command::Install,
                &msg::satellite_not_a_repo(&given.display().to_string()),
            );
            return 1;
        };
        let rel = satellite::relative_path(&repo::main_root(&place.record), &abs);
        out::eprint_stop(
            Command::Install,
            &msg::record_elsewhere("install", &["--satellite", &rel], Actor::Agent, &primary),
        );
        return 1;
    }
    let flag = if claude {
        Some("--claude")
    } else if codex {
        Some("--codex")
    } else if planning {
        Some("--planning")
    } else {
        None
    };
    if let Some(flag) = flag {
        out::eprint_stop(Command::Install, &msg::satellite_takes_no_flag(flag));
        return 1;
    }
    let mut events = Vec::new();
    let result = install_satellite_files(
        &place.code,
        &place.record,
        &primary,
        "",
        Cmd::fixed("adr install"),
        &mut events,
    );
    out::print_events(&events);
    match result {
        Ok(()) => 0,
        Err(stop) => {
            out::eprint_stop(Command::Install, &stop);
            1
        }
    }
}
