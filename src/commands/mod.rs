pub mod approve;
pub mod ask;
pub mod ban;
pub mod check;
pub mod init;
pub mod lock;
pub mod serve;
pub mod show;
pub mod unban;

/// The deny entries `adr init` installs into `.claude/settings.json` and
/// `adr check`'s `install` check verifies are present. Shared so the two
/// stay in lockstep.
pub const DENY_ENTRIES: &[&str] = &[
    "Bash(adr lock*)",
    "Bash(* adr lock*)",
    "Bash(adr approve*)",
    "Bash(* adr approve*)",
    "Bash(adr unban*)",
    "Bash(* adr unban*)",
    "Bash(adr serve*)",
    "Bash(* adr serve*)",
    "Edit(adrdom/state/**)",
    "Write(adrdom/state/**)",
];
