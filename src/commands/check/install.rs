use super::Failure;
use crate::msg;
use crate::repo::{self, Source};
use crate::rules::Class;
use std::path::Path;

const SETTINGS: &str = ".claude/settings.json";
const CODEX_RULES: &str = ".codex/rules/adrdom.rules";

/// Read through `source` -- the index when `check` is judging a commit,
/// the working tree otherwise (per "Does check at a commit read the
/// staged files?"): AGENTS.md/CLAUDE.md/settings.json/the Codex rules are
/// files `check` judges the same as any other, so a copy `adr install`
/// wrote but the owner hasn't yet staged doesn't satisfy a staged check --
/// matching the answer's own "a fix left unstaged does not count until it
/// is staged" (an agent-side classifier refusing to let an agent stage
/// these itself is environment friction outside this tool, not a reason
/// for `check` to treat them differently from any other judged file).
///
/// At a commit, a file whose staged copy fails while the working tree holds
/// what install writes is named once as not staged, with `git add` as its
/// move: running install again would write nothing, so naming install there
/// names a move that clears nothing.
pub(super) fn check_install(root: &Path, source: &Source) -> Vec<Failure> {
    check_install_in(root, source, false)
}

/// `check_install` for a satellite when `satellite`: install writes neither
/// the merge rule, the ignore rule nor the binary's brief there (per "What
/// does install write in a satellite?"), so they are not judged.
pub(super) fn check_install_in(root: &Path, source: &Source, satellite: bool) -> Vec<Failure> {
    let mut out = check_files(root, source, satellite);
    out.extend(check_hooks_path(root));
    out
}

/// git's `core.hooksPath` is a setting of the machine, not a file in the
/// index, so no staged or not-staged reading applies to it
/// (adr-code: db76c9a0-b4fc-4aa2-8afb-47cd78ef7138).
fn check_hooks_path(root: &Path) -> Option<Failure> {
    let said = match repo::hooks_path_state(root) {
        repo::HooksPath::Ours => return None,
        repo::HooksPath::Unset => msg::hooks_path_unset(),
        repo::HooksPath::Foreign(v) => msg::hooks_path_not_ours(&v),
    };
    Some(Failure::said(Class::Install, said))
}

fn check_files(root: &Path, source: &Source, satellite: bool) -> Vec<Failure> {
    let staged = judge(root, source, satellite);
    if !matches!(source, Source::Index) {
        return staged.into_iter().map(|(_, f)| f).collect();
    }
    let on_disk: Vec<&'static str> = judge(root, &Source::WorkingTree, satellite)
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    let mut out = Vec::new();
    let mut named: Vec<&'static str> = Vec::new();
    for (path, f) in staged {
        if on_disk.contains(&path) {
            out.push(f);
        } else if !named.contains(&path) {
            named.push(path);
            out.push(Failure::said(
                Class::Install,
                msg::install_file_not_staged(path),
            ));
        }
    }
    out
}

/// Every file install writes that `source` does not hold as install writes
/// it, each with the path it is about.
fn judge(root: &Path, source: &Source, satellite: bool) -> Vec<(&'static str, Failure)> {
    let mut out: Vec<(&'static str, Failure)> = Vec::new();

    // The git hooks are checked unconditionally, the same as AGENTS.md
    // below (6ef9eca6-fd8b-4211-aae7-00b37a2a9245 A, extended by the owner
    // to cover the hooks `adr install` writes, not just AGENTS.md).
    check_hook(
        &mut out,
        root,
        source,
        repo::PRE_COMMIT_HOOK_PATH,
        crate::commands::install::PRE_COMMIT_HOOK,
    );
    check_hook(
        &mut out,
        root,
        source,
        repo::COMMIT_MSG_HOOK_PATH,
        crate::commands::install::COMMIT_MSG_HOOK,
    );
    check_hook(
        &mut out,
        root,
        source,
        repo::PRE_PUSH_HOOK_PATH,
        crate::commands::install::PRE_PUSH_HOOK,
    );

    if !satellite {
        // The ledger's merge rule (Decision 8, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c):
        // checked unconditionally, the same as the git hooks above -- install
        // writes this file on every install, behind no agent flag. Whole-file
        // comparison against `GITATTRIBUTES_CONTENT`, the one source install's
        // own writer (`install_gitattributes`) uses to decide the same
        // question, so the two can never disagree about whether it's current.
        match source.read(root, repo::GITATTRIBUTES_PATH) {
            Some(content) => {
                if content != crate::commands::install::GITATTRIBUTES_CONTENT {
                    out.push((
                        repo::GITATTRIBUTES_PATH,
                        Failure::said(Class::Install, msg::gitattributes_differs()),
                    ));
                }
            }
            None => out.push((
                repo::GITATTRIBUTES_PATH,
                Failure::said(Class::Install, msg::gitattributes_missing()),
            )),
        }

        // The rule that keeps the index out of git, held like the file above.
        match source.read(root, repo::GITIGNORE_PATH) {
            Some(content) => {
                if content != crate::commands::install::GITIGNORE_CONTENT {
                    out.push((
                        repo::GITIGNORE_PATH,
                        Failure::said(Class::Install, msg::gitignore_differs()),
                    ));
                }
            }
            None => out.push((
                repo::GITIGNORE_PATH,
                Failure::said(Class::Install, msg::gitignore_missing()),
            )),
        }

        // The brief's text: install writes it on every install, and the marked
        // section of AGENTS.md names it. Judged by the comparison install's own
        // writer uses, so the two never disagree.
        match source.read(root, repo::BRIEF_PATH) {
            Some(content) => {
                if !crate::commands::install::brief_is_current(&content) {
                    out.push((
                        repo::BRIEF_PATH,
                        Failure::said(Class::Install, msg::brief_file_differs()),
                    ));
                }
            }
            None => out.push((
                repo::BRIEF_PATH,
                Failure::said(Class::Install, msg::brief_file_missing()),
            )),
        }
    }

    // AGENTS.md is checked unconditionally; CLAUDE.md's import is checked
    // only when .claude/ exists, the same condition used below for the
    // deny entries (4f925b13-ad16-48b8-acd1-3e30d6e308b6, answer A: the
    // CLAUDE.md import is Claude-specific, gated on --claude at install).
    let agents_content = source.read(root, "AGENTS.md");
    match &agents_content {
        Some(content) => {
            let parts: Vec<&str> = content.split('\n').collect();
            match crate::commands::install::find_markers(&parts) {
                Some((begin, end)) => {
                    let section = &parts[begin + 1..end];
                    let tag_lines: Vec<&str> = crate::commands::install::TAG.lines().collect();
                    let matches = crate::text::lines_match_trimmed(section, &tag_lines);
                    if !matches {
                        out.push((
                            "AGENTS.md",
                            Failure::said(Class::Install, msg::agents_md_stale()),
                        ));
                    }
                }
                None => out.push((
                    "AGENTS.md",
                    Failure::said(Class::Install, msg::agents_md_missing_brief()),
                )),
            }
        }
        None => out.push((
            "AGENTS.md",
            Failure::said(Class::Install, msg::agents_md_missing_brief()),
        )),
    }

    if source.is_dir(root, ".claude") {
        if let Some(content) = source.read(root, "CLAUDE.md") {
            if !content
                .lines()
                .any(|l| l == crate::commands::install::CLAUDE_MD_IMPORT)
            {
                out.push((
                    "CLAUDE.md",
                    Failure::said(Class::Install, msg::claude_md_missing_import()),
                ));
            }
        }

        let content = source
            .read(root, ".claude/settings.json")
            .unwrap_or_default();
        if content.trim().is_empty() {
            for entry in crate::commands::deny_entries() {
                out.push((SETTINGS, missing_deny_entry(&entry)));
            }
        } else {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(value) => {
                    let deny: Vec<&str> = value
                        .get("permissions")
                        .and_then(|p| p.get("deny"))
                        .and_then(|d| d.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();
                    for entry in crate::commands::deny_entries() {
                        if !deny.contains(&entry.as_str()) {
                            out.push((SETTINGS, missing_deny_entry(&entry)));
                        }
                    }
                }
                Err(_) => out.push((
                    SETTINGS,
                    Failure::said(
                        Class::Install,
                        msg::settings_unparseable(
                            &root.join(".claude/settings.json").display().to_string(),
                        ),
                    ),
                )),
            }
        }
    }

    // .codex/rules/adrdom.rules (per "What does install write for one
    // agent?", the Codex flag): checked only when
    // .codex/ exists, same as the Claude deny entries above.
    if source.is_dir(root, ".codex") {
        match source.read(root, ".codex/rules/adrdom.rules") {
            Some(content) => {
                if content != crate::commands::install::codex_rules_content() {
                    out.push((
                        CODEX_RULES,
                        Failure::said(Class::Install, msg::codex_rules_stale()),
                    ));
                }
            }
            None => out.push((
                CODEX_RULES,
                Failure::said(Class::Install, msg::codex_rules_missing()),
            )),
        }
    }

    out
}

/// Compares `path`'s content in `source` against `content` with
/// `text::lines_match_trimmed` -- the same comparison `create_hook` (the
/// function that writes this same content) uses to decide whether it is
/// already up to date, so `install` and `check` can never disagree about
/// whether a hook file is current -- and pushes a failure when the file is
/// missing or differs.
fn check_hook(
    out: &mut Vec<(&'static str, Failure)>,
    root: &Path,
    source: &Source,
    path: &'static str,
    content: &str,
) {
    match source.read(root, path) {
        Some(existing) => {
            if !crate::commands::install::hook_is_current(&existing, content) {
                out.push((path, Failure::said(Class::Install, msg::hook_differs(path))));
            } else if !crate::repo::hook_is_executable(&root.join(path)) {
                // Git runs the working-tree file, and only when it is
                // executable: a current hook it would skip is no hook.
                out.push((
                    path,
                    Failure::said(Class::Install, msg::hook_not_executable(path)),
                ));
            }
        }
        None => out.push((path, Failure::said(Class::Install, msg::hook_missing(path)))),
    }
}

fn missing_deny_entry(entry: &str) -> Failure {
    Failure::said(Class::Install, msg::missing_deny_entry(entry))
}
