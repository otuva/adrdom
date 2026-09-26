use super::Failure;
use crate::msg;
use crate::repo::Source;
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
                    let matches = section.len() == brief_lines.len()
                        && section
                            .iter()
                            .zip(brief_lines.iter())
                            .all(|(a, b)| a.trim_end() == b.trim_end());
                    if !matches {
                        out.push(Failure {
                            check: "install".to_string(),
                            id: None,
                            message: msg::agents_md_stale(),
                        });
                    }
                }
                None => out.push(Failure {
                    check: "install".to_string(),
                    id: None,
                    message: msg::agents_md_missing_brief(),
                }),
            }
        }
        None => out.push(Failure {
            check: "install".to_string(),
            id: None,
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
                    id: None,
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
                    id: None,
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
                        id: None,
                        message: msg::codex_rules_stale(),
                    });
                }
            }
            None => out.push(Failure {
                check: "install".to_string(),
                id: None,
                message: msg::codex_rules_missing(),
            }),
        }
    }

    out
}

fn missing_deny_entry(entry: &str) -> Failure {
    Failure {
        check: "install".to_string(),
        id: None,
        message: msg::missing_deny_entry(entry),
    }
}
