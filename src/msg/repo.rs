//! The messages of the repository checks every command starts with: not a repository, or the owner only.

use super::*;

// --- repo.rs ----------------------------------------------------------------

/// `adr <cmd> <args>` as a command a move names, every word quoted as a
/// shell reads it.
/// The git command that puts `rel` back as the last commit holds it, its
/// path quoted so the hint runs as printed.
fn adr_command(cmd: &str, args: &[&str]) -> Cmd {
    let mut words = vec!["adr", cmd];
    words.extend_from_slice(args);
    command(&words)
}

/// `<cmd>: not inside a git repository; ...` -- printed by every command
/// before it does anything else. `actor` is who was already confirmed to
/// be running `cmd` (the owner, for the five owner-only commands, since
/// `owner_authorized` already gated by the time this fires; the agent for
/// every other command).
/// `args` is what the command was given, so the owner's line is the whole
/// command.
pub fn not_a_repo(cmd: &str, args: &[&str], actor: Actor) -> String {
    let command = adr_command(cmd, args);
    // What clears it is where the command runs, so the move names that
    // first: the command itself is run again from inside a repository.
    let into = Act::ChangesInto("a git repository".into());
    let next = match actor {
        Actor::Agent | Actor::Builder => Next::agent(into).then(Act::Runs(command)),
        Actor::Owner => Next::owner(into).then(Act::Runs(command)),
    };
    refuse(format!("{}: not inside a git repository", cmd), next)
}

/// The refusal every owner-only command prints when stdin isn't an
/// interactive terminal: `<cmd>: refusing: this is the owner's command;
/// owner runs: adr <cmd> <args>`.
pub fn owner_refusal(cmd: &str, args: &[&str]) -> String {
    let command = adr_command(cmd, args);
    refuse(
        format!("{}: refusing: this is the owner's command", cmd),
        Next::owner_runs(command),
    )
}
