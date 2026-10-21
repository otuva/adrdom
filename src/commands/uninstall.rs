use crate::commands::deny_entries;
use crate::commands::install::{
    self, CLAUDE_MD_IMPORT, CLAUDE_MD_MARKER, COMMIT_MSG_HOOK, PRE_COMMIT_HOOK, PRE_PUSH_HOOK,
};
use crate::msg::{self, Actor, Next};
use crate::out::{self, Event};
use crate::repo;
use crate::rules::Command;
use crate::satellite::{self, Located, Place};
use std::fs;
use std::path::{Path, PathBuf};

/// The event of the file at `rel` edited in place.
fn written(rel: &str) -> Event {
    Event::FileWritten {
        path: rel.to_string(),
    }
}

/// The event of the file or directory at `rel` removed.
fn removed(rel: &str) -> Event {
    Event::FileRemoved {
        path: rel.to_string(),
    }
}

/// Owner-only: removes everything `adr install` wrote outside `adrdom/`,
/// leaving `adrdom/` itself untouched. With `--satellite <path>` it removes
/// nothing but the row of the satellite no longer found at `path` (per "What
/// drops the row of a satellite the primary no longer finds at its path?",
/// 59e73422-5d55-4b4c-9793-25f78f5f9586 A).
pub fn run(satellite: Option<PathBuf>) -> i32 {
    let shown = satellite.as_ref().map(|p| p.display().to_string());
    let args: Vec<&str> = shown
        .iter()
        .flat_map(|p| ["--satellite", p.as_str()])
        .collect();
    if !repo::owner_authorized() {
        out::eprint_stop(Command::Uninstall, &msg::owner_refusal("uninstall", &args));
        return 2;
    }

    let place = match satellite::locate() {
        Located::NotARepo => {
            out::eprint_stop(
                Command::Uninstall,
                &msg::not_a_repo("uninstall", &args, Actor::Owner),
            );
            return 1;
        }
        Located::RecordMissing { primary, .. } => {
            out::eprint_stop(
                Command::Uninstall,
                &msg::record_missing("uninstall", &primary),
            );
            return 1;
        }
        Located::Here(place) => place,
    };
    if let Some(given) = satellite {
        return drop_missing(&given, &place);
    }

    // In a satellite the row it has in the primary's list goes too (per
    // "Does uninstall in a satellite drop its row from the primary?"); with
    // no record at the primary's path nothing is removed.
    let (root, primary_root) = if let Some(shown) = place.primary_shown() {
        (place.code, Some((place.record, shown)))
    } else {
        (place.code, None)
    };

    // One line per file removed or edited, or the failure that stopped it
    // (per "What does a command that writes print?", 069a6b4a A). What was
    // removed is printed on a failure too: the run that failed is the only
    // one that can say it, the next one finds it gone.
    let mut events = Vec::new();
    let mut failed = remove_all(&root, &mut events).err();
    if let (None, Some((record, shown))) = (&failed, &primary_root) {
        let sat = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
        // Per "What does a path under adrdom to another repository resolve
        // against?" (0320a987-f8b2-4105-9b27-64bf484026e6 A): between the
        // main working trees. The row is found by the directory it names, as
        // install finds the row that holds a satellite (per "Does install
        // take a satellite that holds the path of another primary?",
        // 490cdbb6-0482-41ba-8f57-36a6136b166f A), so this run lets go of
        // every row install would see.
        let here = repo::main_root(&sat);
        let path = satellite::row_at(record, &here).map_or_else(
            || satellite::relative_path(&repo::main_root(record), &here),
            |r| r.path,
        );
        match satellite::drop_row(record, &path) {
            Ok(Some(Event::FileWritten { path })) => events.push(written(&format!(
                "{}/{}",
                shown.trim_end_matches('/'),
                path
            ))),
            Ok(_) => {}
            Err(e) => {
                failed = Some(msg::write_failed(
                    repo::SATELLITES_PATH,
                    e,
                    msg::after_fix_owner_runs(crate::moves::Cmd::adr("uninstall")),
                ))
            }
        }
    }
    out::print_events(&events);
    match failed {
        Some(m) => {
            out::eprint_stop(Command::Uninstall, &m);
            1
        }
        None => 0,
    }
}

/// `uninstall --satellite <given>`, run in the primary: drops the row whose
/// directory is `given` and removes nothing else. Both paths are read by
/// their words, since the satellite is no longer there to resolve. A
/// satellite is not run from (per "Does a command that writes the record run
/// in a satellite?"), and a checkout still at the path is not dropped: its
/// own `adr uninstall` does that.
fn drop_missing(given: &Path, place: &Place) -> i32 {
    let shown = given.display().to_string();
    let main = repo::main_root(&place.record);
    let Ok(cwd) = std::env::current_dir() else {
        out::eprint_stop(Command::Uninstall, &msg::satellite_not_listed(&shown));
        return 1;
    };
    let wanted = satellite::lexical(&cwd, given);
    if let Some(primary) = place.primary_shown() {
        let rel = satellite::relative_path(&main, &wanted);
        out::eprint_stop(
            Command::Uninstall,
            &msg::record_elsewhere("uninstall", &["--satellite", &rel], Actor::Owner, &primary),
        );
        return 1;
    }
    let Some(row) = satellite::rows(&place.record)
        .into_iter()
        .find(|r| satellite::lexical(&main, Path::new(&r.path)) == wanted)
    else {
        out::eprint_stop(Command::Uninstall, &msg::satellite_not_listed(&shown));
        return 1;
    };
    if wanted.join(".git").exists() {
        out::eprint_stop(Command::Uninstall, &msg::satellite_still_there(&shown));
        return 1;
    }
    match satellite::drop_row(&place.record, &row.path) {
        Ok(event) => {
            out::print_events(&event.into_iter().collect::<Vec<_>>());
            0
        }
        Err(e) => {
            out::eprint_stop(
                Command::Uninstall,
                &msg::write_failed(
                    repo::SATELLITES_PATH,
                    e,
                    msg::after_fix_owner_runs(crate::moves::command(&[
                        "uninstall",
                        "--satellite",
                        &shown,
                    ])),
                ),
            );
            1
        }
    }
}

/// Every removal in order, stopping at the first that fails; `events`
/// holds one event for each file removed or edited up to that point.
fn remove_all(root: &Path, events: &mut Vec<Event>) -> Result<(), String> {
    events.extend(remove_agents_md_section(root).map_err(|e| {
        msg::update_failed(
            "AGENTS.md",
            e,
            msg::after_fix_owner_runs(crate::moves::Cmd::adr("uninstall")),
        )
    })?);
    events.extend(remove_claude_md_import(root).map_err(|e| {
        msg::update_failed(
            "CLAUDE.md",
            e,
            msg::after_fix_owner_runs(crate::moves::Cmd::adr("uninstall")),
        )
    })?);
    remove_hooks(root, events)?;
    remove_settings_deny(root, events)?;
    remove_codex_rules(root, events)
}

/// The text of `path`, `None` when it is not there. A file that is there
/// and cannot be read as text is a failure, named, never skipped: what
/// install wrote into it would stay without a word.
fn read_if_present(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(c) => Ok(Some(c)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Removes the marked brief section from AGENTS.md, plus the one blank
/// line right before it if there is one. Deletes the file entirely when
/// nothing but whitespace remains.
fn remove_agents_md_section(root: &Path) -> Result<Option<Event>, String> {
    let path = root.join("AGENTS.md");
    let Some(existing) = read_if_present(&path)? else {
        return Ok(None);
    };
    let parts: Vec<&str> = existing.split('\n').collect();
    let Some((begin, end)) = install::find_markers(&parts) else {
        return Ok(None);
    };
    // Each piece leaves only while it still matches what install wrote (per
    // "Does uninstall belong to the owner?"): a section edited by hand is
    // left, compared the way check compares it.
    let tag_lines: Vec<&str> = install::TAG.lines().collect();
    if !crate::text::lines_match_trimmed(&parts[begin + 1..end], &tag_lines) {
        return Ok(None);
    }

    let remove_from = if begin > 0 && parts[begin - 1].is_empty() {
        begin - 1
    } else {
        begin
    };

    let mut new_parts: Vec<&str> = Vec::new();
    new_parts.extend_from_slice(&parts[..remove_from]);
    new_parts.extend_from_slice(&parts[end + 1..]);
    let new_content = new_parts.join("\n");

    if new_content.trim().is_empty() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
        return Ok(Some(removed("AGENTS.md")));
    }
    fs::write(&path, &new_content).map_err(|e| e.to_string())?;
    Ok(Some(written("AGENTS.md")))
}

/// Removes the marker comment line and the `@AGENTS.md` line right after
/// it from CLAUDE.md, plus the one blank line install put before it if
/// present, and deletes the file when nothing else is in it (the case
/// where `adr install --claude` created it). This byte-reverts the file when it already ended with a
/// single trailing newline (the common case); a file install found with
/// no trailing newline, or with more than one blank line at the end,
/// canonicalizes to a single trailing newline instead of restoring
/// exactly, because install's blank-line insertion collapses those cases
/// to the same on-disk text. A bare `@AGENTS.md` line with no marker
/// above it is the owner's own and is left alone.
fn remove_claude_md_import(root: &Path) -> Result<Option<Event>, String> {
    let path = root.join("CLAUDE.md");
    let Some(existing) = read_if_present(&path)? else {
        return Ok(None);
    };
    let parts: Vec<&str> = existing.split('\n').collect();
    let Some(marker_idx) = parts.iter().position(|l| *l == CLAUDE_MD_MARKER) else {
        return Ok(None);
    };
    if parts.get(marker_idx + 1).copied() != Some(CLAUDE_MD_IMPORT) {
        return Ok(None);
    }

    let remove_from = if marker_idx > 0 && parts[marker_idx - 1].is_empty() {
        marker_idx - 1
    } else {
        marker_idx
    };

    let mut new_parts: Vec<&str> = Vec::new();
    new_parts.extend_from_slice(&parts[..remove_from]);
    new_parts.extend_from_slice(&parts[marker_idx + 2..]);
    let new_content = new_parts.join("\n");

    if new_content.trim().is_empty() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
        return Ok(Some(removed("CLAUDE.md")));
    }

    fs::write(&path, &new_content).map_err(|e| e.to_string())?;
    Ok(Some(written("CLAUDE.md")))
}

/// Deletes the three hooks only when their content still matches exactly
/// what `adr install` writes, removes `.githooks` once empty, and unsets
/// `core.hooksPath` only when it's still `.githooks`. A hook that is not
/// what this build of install writes (an owner's edit, or the hook of
/// another version) is left, and so is `.githooks` around it.
fn remove_hooks(root: &Path, events: &mut Vec<Event>) -> Result<(), String> {
    let hooks = root.join(".githooks");

    for (name, content_written) in [
        ("pre-commit", PRE_COMMIT_HOOK),
        ("commit-msg", COMMIT_MSG_HOOK),
        ("pre-push", PRE_PUSH_HOOK),
    ] {
        let path = hooks.join(name);
        if fs::read_to_string(&path).is_ok_and(|content| content == content_written) {
            remove(&path, fs::remove_file(&path))?;
            events.push(removed(&format!(".githooks/{}", name)));
        }
    }
    if remove(&hooks, remove_dir_when_empty(&hooks))? {
        events.push(removed(".githooks"));
    }

    // Unset only once every hook install wrote is gone: a hook left (one
    // edited by hand) keeps running where git looks for it. The change of
    // git's configuration is printed as the write it is.
    if let Some(val) = repo::git(root, &["config", "core.hooksPath"]) {
        if val.trim() == ".githooks" && !hooks.exists() {
            if repo::git(root, &["config", "--unset", "core.hooksPath"]).is_none() {
                return Err(msg::hooks_path_failed(
                    "unset",
                    msg::after_fix_owner_runs(crate::moves::Cmd::adr("uninstall")),
                ));
            }
            events.push(Event::ConfigUnset {
                key: "core.hooksPath".to_string(),
            });
        }
    }

    Ok(())
}

/// The refusal for a removal that failed, naming what was not removed.
fn remove<T>(path: &Path, done: std::io::Result<T>) -> Result<T, String> {
    done.map_err(|e| {
        msg::remove_failed(
            &path.display().to_string(),
            e,
            msg::after_fix_owner_runs(crate::moves::Cmd::adr("uninstall")),
        )
    })
}

/// Removes `dir` when it holds nothing; true when it was removed. A
/// directory that is missing or holds something is left as it is.
fn remove_dir_when_empty(dir: &Path) -> std::io::Result<bool> {
    let Ok(mut entries) = fs::read_dir(dir) else {
        return Ok(false);
    };
    if entries.next().is_some() {
        return Ok(false);
    }
    fs::remove_dir(dir)?;
    Ok(true)
}

/// Removes `.codex/rules/adrdom.rules` only when its content still matches
/// exactly what `adr install --codex` writes, then removes `.codex/rules`
/// once empty, and `.codex` once that is empty too. Leaves an
/// owner-edited or foreign file alone (mirrors `remove_hooks`).
fn remove_codex_rules(root: &Path, events: &mut Vec<Event>) -> Result<(), String> {
    let rules_dir = root.join(".codex/rules");
    let rules_path = rules_dir.join("adrdom.rules");
    let codex_dir = root.join(".codex");

    if fs::read_to_string(&rules_path)
        .is_ok_and(|content| content == install::codex_rules_content())
    {
        remove(&rules_path, fs::remove_file(&rules_path))?;
        events.push(removed(".codex/rules/adrdom.rules"));
    }
    if remove(&rules_dir, remove_dir_when_empty(&rules_dir))? {
        events.push(removed(".codex/rules"));
    }
    if remove(&codex_dir, remove_dir_when_empty(&codex_dir))? {
        events.push(removed(".codex"));
    }

    Ok(())
}

/// Removes every `deny_entries()` string from `.claude/settings.json`'s
/// `permissions.deny`, leaving every other key and entry untouched. Deletes
/// the file when nothing but what install wrote remains (an empty
/// `permissions.deny` and no other key anywhere), then removes `.claude`
/// once that leaves it empty. Does nothing when the file doesn't exist. A
/// failure is the whole refusal, its move named once.
fn remove_settings_deny(root: &Path, events: &mut Vec<Event>) -> Result<(), String> {
    const REL: &str = ".claude/settings.json";
    let failed = |e: String| {
        msg::update_failed(
            REL,
            e,
            msg::after_fix_owner_runs(crate::moves::Cmd::adr("uninstall")),
        )
    };
    let dir = root.join(".claude");
    let path = dir.join("settings.json");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path).map_err(|e| failed(e.to_string()))?;
    if content.trim().is_empty() {
        return Ok(());
    }
    let mut value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        msg::parse_failed(
            REL,
            e,
            Next::agent(msg::Act::Rewrites(msg::Part::File(REL.to_string())))
                .and_owner_runs(crate::moves::Cmd::adr("uninstall")),
        )
    })?;

    let mut any_removed = false;
    if let Some(deny) = value
        .get_mut("permissions")
        .and_then(|p| p.get_mut("deny"))
        .and_then(|d| d.as_array_mut())
    {
        for entry in deny_entries() {
            let before = deny.len();
            deny.retain(|v| v.as_str() != Some(entry.as_str()));
            any_removed |= deny.len() != before;
        }
    }

    if !any_removed {
        return Ok(());
    }

    if settings_holds_only_deny(&value) {
        fs::remove_file(&path).map_err(|e| failed(e.to_string()))?;
        // Printed whatever follows: the file is gone.
        events.push(removed(REL));
        if remove_dir_when_empty(&dir).map_err(|e| {
            msg::remove_failed(
                ".claude",
                e,
                msg::after_fix_owner_runs(crate::moves::Cmd::adr("uninstall")),
            )
        })? {
            events.push(removed(".claude"));
        }
        return Ok(());
    }

    let mut pretty = serde_json::to_string_pretty(&value).map_err(|e| failed(e.to_string()))?;
    pretty.push('\n');
    fs::write(&path, pretty).map_err(|e| failed(e.to_string()))?;
    events.push(written(REL));
    Ok(())
}

/// True when `value` holds nothing but the shape install writes: a top
/// level with only `permissions`, a `permissions` with only `deny`, and an
/// empty `deny` array.
fn settings_holds_only_deny(value: &serde_json::Value) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    if obj.len() != 1 {
        return false;
    }
    let Some(permissions) = obj.get("permissions").and_then(|p| p.as_object()) else {
        return false;
    };
    if permissions.len() != 1 {
        return false;
    }
    permissions
        .get("deny")
        .and_then(|d| d.as_array())
        .is_some_and(|deny| deny.is_empty())
}
