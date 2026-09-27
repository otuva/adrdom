use super::Failure;
use crate::msg;
use crate::repo::{self, Source};
use std::path::Path;

/// Read through `source` -- the index when `check` is judging a commit,
/// the working tree otherwise (per "Does check at a commit read the
/// staged files?"): AGENTS.md/CLAUDE.md/settings.json/the Codex rules are
/// files `check` judges the same as any other, so a copy `adr install`
/// wrote but the owner hasn't yet staged doesn't satisfy a staged check --
/// matching the answer's own "a fix left unstaged does not count until it
/// is staged" (an agent-side classifier refusing to let an agent stage
/// these itself is environment friction outside this tool, not a reason
/// for `check` to treat them differently from any other judged file).
pub(super) fn check_install(root: &Path, source: &Source) -> Vec<Failure> {
    let mut out = Vec::new();

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
                    let brief_lines: Vec<&str> = crate::commands::install::BRIEF.lines().collect();
                    let matches = crate::text::lines_match_trimmed(section, &brief_lines);
                    if !matches {
                        out.push(Failure {
                            check: "install".to_string(),
                            message: msg::agents_md_stale(),
                        });
                    }
                }
                None => out.push(Failure {
                    check: "install".to_string(),
                    message: msg::agents_md_missing_brief(),
                }),
            }
        }
        None => out.push(Failure {
            check: "install".to_string(),
            message: msg::agents_md_missing_brief(),
        }),
    }

    if source.is_dir(root, ".claude") {
        if let Some(content) = source.read(root, "CLAUDE.md") {
            if !content
                .lines()
                .any(|l| l == crate::commands::install::CLAUDE_MD_IMPORT)
            {
                out.push(Failure {
                    check: "install".to_string(),
                    message: msg::claude_md_missing_import(),
                });
            }
        }

        let content = source.read(root, ".claude/settings.json").unwrap_or_default();
        if content.trim().is_empty() {
            for entry in crate::commands::deny_entries() {
                out.push(missing_deny_entry(&entry));
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
                            out.push(missing_deny_entry(&entry));
                        }
                    }
                }
                Err(_) => out.push(Failure {
                    check: "install".to_string(),
                    message: msg::settings_unparseable(&root.join(".claude/settings.json").display().to_string()),
                }),
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
                    out.push(Failure {
                        check: "install".to_string(),
                        message: msg::codex_rules_stale(),
                    });
                }
            }
            None => out.push(Failure {
                check: "install".to_string(),
                message: msg::codex_rules_missing(),
            }),
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
fn check_hook(out: &mut Vec<Failure>, root: &Path, source: &Source, path: &str, content: &str) {
    match source.read(root, path) {
        Some(existing) => {
            let existing_lines: Vec<&str> = existing.split('\n').collect();
            let want_lines: Vec<&str> = content.split('\n').collect();
            let matches = crate::text::lines_match_trimmed(&existing_lines, &want_lines);
            if !matches {
                out.push(Failure {
                    check: "install".to_string(),
                    message: msg::hook_differs(path),
                });
            }
        }
        None => out.push(Failure {
            check: "install".to_string(),
            message: msg::hook_missing(path),
        }),
    }
}

fn missing_deny_entry(entry: &str) -> Failure {
    Failure {
        check: "install".to_string(),
        message: msg::missing_deny_entry(entry),
    }
}
