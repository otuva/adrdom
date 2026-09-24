pub mod answer;
pub mod approve;
pub mod ask;
pub mod ban;
pub mod check;
pub mod install;
pub mod serve;
pub mod show;
pub mod unban;
pub mod uninstall;

/// The five commands only the owner runs (matches the brief's "Commands"
/// section and `repo::owner_authorized`'s callers): the one list the Claude
/// deny entries and the Codex rules are both derived from.
pub const OWNER_COMMANDS: &[&str] = &["answer", "approve", "unban", "serve", "uninstall"];

/// The deny entries `adr install` installs into `.claude/settings.json` and
/// `adr check`'s `install` check verifies are present: two `Bash` patterns
/// per `OWNER_COMMANDS` entry, in that order, then the two `adrdom/state/**`
/// entries -- the exact strings and order already installed, so existing
/// installs stay valid.
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
