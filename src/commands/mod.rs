pub mod answer;
pub mod approve;
pub mod ban;
pub mod check;
pub mod install;
pub mod plan;
pub mod read;
pub mod serve;
pub mod show;
pub mod unban;
pub mod uninstall;

/// Per "Does a command that writes the record run in a satellite?" (no): in a
/// satellite, prints the refusal naming the primary's path and returns true.
/// The caller returns 1 before it reads or writes anything.
pub(crate) fn refused_in_satellite(
    root: &std::path::Path,
    cmd: crate::rules::Command,
    args: &[&str],
    actor: crate::msg::Actor,
) -> bool {
    use crate::rules::Command;
    let Some(primary) = crate::satellite::primary_of(root) else {
        return false;
    };
    // The move runs from the primary, so the file the command names is
    // printed from the primary's root when it lies inside it (the other
    // arguments, the entry of approve, as given).
    let mut shown: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    if matches!(cmd, Command::Plan | Command::Approve) {
        if let (Some(file), Ok(record)) = (
            shown.first_mut(),
            std::fs::canonicalize(crate::repo::main_root(root).join(&primary)),
        ) {
            if let Ok(abs) = crate::satellite::absolute(std::path::Path::new(file.as_str())) {
                if let Ok(inside) = abs.strip_prefix(&record) {
                    *file = crate::satellite::relative_path(&record, &record.join(inside));
                }
            }
        }
    }
    let shown: Vec<&str> = shown.iter().map(String::as_str).collect();
    crate::out::eprint_stop(
        cmd,
        &crate::msg::record_elsewhere(
            cmd.as_str(),
            &shown,
            actor,
            &crate::satellite::from_checkout(root, &primary),
        ),
    );
    true
}

/// The five commands only the owner runs (matches the brief's "Commands"
/// section and `repo::owner_authorized`'s callers): the one list the Claude
/// deny entries and the Codex rules are both derived from.
pub const OWNER_COMMANDS: &[&str] = &["answer", "approve", "unban", "serve", "uninstall"];

/// The deny entries `adr install` installs into `.claude/settings.json` and
/// `adr check`'s `install` check verifies are present: two `Bash` patterns
/// per `OWNER_COMMANDS` entry, then the two `adrdom/state/**` entries.
/// `check` (`commands/check/install.rs`) compares this list against
/// `.claude/settings.json` by membership only, not by order, so an install
/// with the entries in a different order (or interleaved with owner-added
/// ones) still passes.
pub fn deny_entries() -> Vec<String> {
    let mut out = Vec::new();
    for cmd in OWNER_COMMANDS {
        out.push(format!("Bash(adr {}*)", cmd));
        out.push(format!("Bash(* adr {}*)", cmd));
    }
    out.push("Edit(adrdom/state/**)".to_string());
    out.push("Write(adrdom/state/**)".to_string());
    out
}
