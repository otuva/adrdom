use super::Failure;
use std::path::Path;

pub(super) fn check_install(root: &Path) -> Vec<Failure> {
    let mut out = Vec::new();

    // AGENTS.md and CLAUDE.md are checked unconditionally, regardless of
    // whether .claude/ exists.
    let agents_content = std::fs::read_to_string(root.join("AGENTS.md"));
    match &agents_content {
        Ok(content) => {
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
                            message: "AGENTS.md brief differs from this binary's; run: adr install"
                                .to_string(),
                        });
                    }
                }
                None => out.push(Failure {
                    check: "install".to_string(),
                    id: None,
                    message: "AGENTS.md lacks the adrdom brief; run: adr install".to_string(),
                }),
            }
        }
        Err(_) => out.push(Failure {
            check: "install".to_string(),
            id: None,
            message: "AGENTS.md lacks the adrdom brief; run: adr install".to_string(),
        }),
    }

    if let Ok(content) = std::fs::read_to_string(root.join("CLAUDE.md")) {
        if !content
            .lines()
            .any(|l| l == crate::commands::install::CLAUDE_MD_IMPORT)
        {
            out.push(Failure {
                check: "install".to_string(),
                id: None,
                message: "CLAUDE.md does not import AGENTS.md; run: adr install".to_string(),
            });
        }
    }

    if root.join(".claude").is_dir() {
        let settings_path = root.join(".claude/settings.json");
        let content = std::fs::read_to_string(&settings_path).unwrap_or_default();
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
                    message: format!(
                        "{} cannot be parsed; owner runs: adr install --claude",
                        settings_path.display()
                    ),
                }),
            }
        }
    }

    // .codex/rules/adrdom.rules (9.2 A note, Codex flag): checked only when
    // .codex/ exists, same as the Claude deny entries above.
    if root.join(".codex").is_dir() {
        let rules_path = root.join(".codex/rules/adrdom.rules");
        match std::fs::read_to_string(&rules_path) {
            Ok(content) => {
                if content != crate::commands::install::codex_rules_content() {
                    out.push(Failure {
                        check: "install".to_string(),
                        id: None,
                        message: ".codex/rules/adrdom.rules differs from this binary's; owner runs: adr install --codex"
                            .to_string(),
                    });
                }
            }
            Err(_) => out.push(Failure {
                check: "install".to_string(),
                id: None,
                message: ".codex/rules/adrdom.rules missing; owner runs: adr install --codex"
                    .to_string(),
            }),
        }
    }

    out
}

fn missing_deny_entry(entry: &str) -> Failure {
    Failure {
        check: "install".to_string(),
        id: None,
        message: format!(
            ".claude/settings.json lacks deny entry {}; owner runs: adr install --claude",
            entry
        ),
    }
}
