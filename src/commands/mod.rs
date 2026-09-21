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

/// The deny entries `adr install` installs into `.claude/settings.json` and
/// `adr check`'s `install` check verifies are present. Shared so the two
/// stay in lockstep.
pub const DENY_ENTRIES: &[&str] = &[
    "Bash(adr answer*)",
    "Bash(* adr answer*)",
    "Bash(adr approve*)",
    "Bash(* adr approve*)",
    "Bash(adr unban*)",
    "Bash(* adr unban*)",
    "Bash(adr serve*)",
    "Bash(* adr serve*)",
    "Bash(adr uninstall*)",
    "Bash(* adr uninstall*)",
    "Edit(adrdom/state/**)",
    "Write(adrdom/state/**)",
];
